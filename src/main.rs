#[macro_use]
extern crate log;

use std::convert::TryFrom;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use eth_listener::events::*;
use eth_listener::exchange::matchengine_client::MatchengineClient;
use eth_listener::exchange::{BalanceUpdateRequest, EthLogMetadata, UserInfo};
use eth_listener::infos::ContractInfos;
use eth_listener::persist::Persistor;
use eth_listener::restapi::{NewAssetReq, RestClient};
use eth_listener::ConfirmedBlockStream;
use eth_listener::CONFIG;
use ethers::prelude::*;
use rust_decimal::Decimal;
use tonic::transport::Channel;
use std::sync::atomic::{AtomicU64, Ordering};

/// A helper to convert ethers Log to EthLogMetadata
trait ToLogMeta {
    fn to_log_meta(&self) -> EthLogMetadata;
}

impl ToLogMeta for Log {
    fn to_log_meta(&self) -> EthLogMetadata {
        EthLogMetadata {
            block_number: self.block_number.unwrap().as_u64(),
            tx_hash: format!("{:#x}", self.transaction_hash.unwrap()),
            log_index: format!("{:#x}", self.log_index.unwrap()),
        }
    }
}

fn get_business_id() -> u64 {
    static BUSINESS_ID_SERIAL: AtomicU64 = AtomicU64::new(0);
    BUSINESS_ID_SERIAL.fetch_add(1, Ordering::SeqCst)
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    info!("{:?}", *CONFIG);

    let contract_address: Address = CONFIG.web3().contract_address().parse().unwrap();

    let provider = Arc::new(Provider::connect(CONFIG.web3().web3_url()).await?);
    let grpc_channel = Channel::from_static(CONFIG.exchange().grpc_endpoint())
        .connect_timeout(Duration::from_secs(10))
        .connect()
        .await?;
    let mut grpc_client = MatchengineClient::new(grpc_channel);
    info!("grpc client ready");

    let rest_client = RestClient::new(CONFIG.exchange().rest_endpoint());
    info!("rest client ready");

    let mut contract_infos = ContractInfos::new(provider.clone(), contract_address);

    let persistor = Persistor::new(CONFIG.storage().db(), CONFIG.web3().base_block()).await?;
    info!("persistor ready");

    info!("start listening on eth net");

    let mut confirmed_stream =
        ConfirmedBlockStream::new(&provider, persistor.get_block_number().await?, 3).await?;

    while let Some(block) = confirmed_stream.next().await {
        let block = block?;
        let block_number = block.number.unwrap();
        info!(
            "current: {}, confirmed: {} {:?}",
            provider.get_block_number().await?.as_u64(),
            block_number,
            block.hash.unwrap()
        );
        let log_filter = Filter::default()
            .from_block(block_number)
            .to_block(block_number)
            .address(CONFIG.web3().contract_address().parse::<Address>().unwrap());
        let events = provider
            .get_logs(&log_filter)
            .await?
            .into_iter()
            .map(Events::try_from)
            .collect::<Result<Vec<Events>, EventParseError>>()?;
        for event in events {
            info!("process event: {:?}", event);
            match event {
                Events::Deposit(deposit) => {
                    let user_id = contract_infos.fetch_user_id(&deposit.to).await?;
                    let mut delta = Decimal::from_str(deposit.amount.to_string().as_str())?;
                    if deposit.token_id == 0 {
                        // we are dealing with an ETH deposit request
                        // 1 ETH = 10^18 wei
                        delta.set_scale(18)?;
                        grpc_client
                            .balance_update(BalanceUpdateRequest {
                                user_id: user_id as u32,
                                asset: "ETH".to_string(),
                                business: "deposit".to_string(),
                                business_id: get_business_id(),
                                delta: format!("{}", delta),
                                detail: "".to_string(),
                                log_metadata: Some(deposit.origin.to_log_meta()),
                            })
                            .await?;
                    } else {
                        // we are dealing with an ERC20 deposit request
                        let address = contract_infos.fetch_token_address(deposit.token_id).await?;
                        let erc20 = contract_infos.fetch_erc20(address).await;
                        delta.set_scale(erc20.decimals as u32)?;
                        grpc_client
                            .balance_update(BalanceUpdateRequest {
                                user_id: user_id as u32,
                                asset: erc20.name,
                                business: "deposit".to_string(),
                                business_id: get_business_id(),
                                delta: format!("{}", delta),
                                detail: "".to_string(),
                                log_metadata: Some(deposit.origin.to_log_meta()),
                            })
                            .await?;
                    }
                }
                Events::NewToken(new_token) => {
                    let asset = contract_infos
                        .add_token(new_token.token_addr, new_token.token_id)
                        .await;
                    rest_client
                        .add_assets(&NewAssetReq {
                            assets: vec![asset],
                            not_reload: false,
                        })
                        .await?;
                }
                Events::RegisterUser(register_user) => {
                    grpc_client
                        .register_user(UserInfo {
                            user_id: register_user.user_id as u32,
                            l1_address: register_user.eth_addr.to_string(),
                            l2_pubkey: hex::encode(register_user.bjj_pubkey),
                            log_metadata: Some(register_user.origin.to_log_meta()),
                        })
                        .await?;
                }
            }
        }
        persistor.save_block_number(block_number.as_u64()).await?;
    }

    Ok(())
}
