#[macro_use]
extern crate log;

use std::convert::TryFrom;

use anyhow::Result;
use ethers::prelude::*;

use eth_listener::events::*;
use eth_listener::exchange::matchengine_client::MatchengineClient;
use eth_listener::exchange::UserInfo;
use eth_listener::ConfirmedBlockStream;
use eth_listener::CONFIG;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    println!("{:?}", *CONFIG);

    let provider = Provider::connect("ws://localhost:8545").await?;
    let mut grpc_client = MatchengineClient::connect(CONFIG.exchange().grpc_endpoint()).await?;

    info!("start listening on eth net");

    let mut confirmed_stream =
        ConfirmedBlockStream::new(&provider, provider.get_block_number().await?.as_u64(), 3)
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
                Events::Deposit(deposit) => unimplemented!(),
                Events::NewToken(new_token) => unimplemented!(),
                Events::RegisterUser(register_user) => {
                    grpc_client
                        .register_user(UserInfo {
                            user_id: register_user.user_id,
                            l1_address: register_user.eth_addr.to_string(),
                            l2_pubkey: hex::encode(register_user.bjj_pubkey),
                        })
                        .await?;
                }
            }
        }
    }

    Ok(())
}
