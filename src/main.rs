#[macro_use]
extern crate log;

use std::convert::TryFrom;

use anyhow::Result;
use ethers::prelude::*;

use eth_listener::events::*;
use eth_listener::exchange::matchengine_client::MatchengineClient;
use eth_listener::exchange::{UserInfo, EthLogMetadata, BalanceUpdateRequest};
use eth_listener::ConfirmedBlockStream;
use eth_listener::CONFIG;
use eth_listener::restapi::{RestClient, NewAssetReq};
use eth_listener::infos::ContractInfos;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::ops::Div;

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

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    println!("{:?}", *CONFIG);

    let contract_address: Address = CONFIG.web3().contract_address().parse().unwrap();

    let provider = Provider::connect("ws://localhost:8545").await?;
    let mut grpc_client = MatchengineClient::connect(CONFIG.exchange().grpc_endpoint()).await?;
    let rest_client = RestClient::new(CONFIG.exchange().rest_endpoint());
    let mut contract_infos = ContractInfos::new(&provider, contract_address);

    info!("start listening on eth net");

    let mut confirmed_stream =
        ConfirmedBlockStream::new(
            &provider,
            provider.get_block_number().await?.as_u64(), // TODO: replace with block height from db.
            3)
            .await?;

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
            .address(CONFIG.web3().web3_url().parse::<Address>().unwrap());
        let events = provider
            .get_logs(&log_filter)
            .await?
            .into_iter()
            .map(|log| Events::try_from(log))
            .collect::<Result<Vec<Events>, EventParseError>>()?;
        for event in events {
            match event {
                Events::Deposit(deposit) => {
                    let user_id = contract_infos.fetch_user_id(deposit.to).await?;
                    let mut delta = Decimal::from_str(deposit.amount)?;
                    delta.set_scale(erc20.decimals as u32)?;
                    if deposit.token_id == 0 {
                        // we are dealing with an ETH deposit request
                        grpc_client
                            .balance_update(BalanceUpdateRequest {
                                user_id: user_id as u32,
                                asset: "ETH".to_string(),
                                business: "".to_string(),
                                business_id: 0,
                                delta: format!("{}", delta),
                                detail: "".to_string(),
                                log_metadata: Some(deposit.origin.to_log_meta())
                            })
                            .await?;
                    } else {
                        // we are dealing with an ERC20 deposit request
                        let address = contract_infos.fetch_token_address(deposit.token_id).await?;
                        let erc20 = contract_infos.fetch_erc20(address).await;
                        grpc_client
                            .balance_update(BalanceUpdateRequest {
                                user_id: user_id as u32,
                                asset: erc20.name,
                                business: "".to_string(),
                                business_id: 0,
                                delta: format!("{}", delta),
                                detail: "".to_string(),
                                log_metadata: Some(deposit.origin.to_log_meta())
                            })
                            .await?;
                    }

                },
                Events::NewToken(new_token) => {
                    let asset = contract_infos.add_token(new_token.token_addr, new_token.token_id).await;
                    rest_client
                        .add_assets(&NewAssetReq {
                            assets: vec![asset],
                            not_reload: false
                        })
                        .await?;
                },
                Events::RegisterUser(register_user) => {
                    grpc_client
                        .register_user(UserInfo {
                            user_id: register_user.user_id as u32,
                            l1_address: register_user.eth_addr.to_string(),
                            l2_pubkey: hex::encode(register_user.bjj_pubkey),
                            log_metadata: Some(register_user.origin.to_log_meta())
                        })
                        .await?;
                }
            }
        }
        // TODO: update the current block height.
    }

    Ok(())
}
