#[macro_use]
extern crate log;

use std::convert::TryFrom;

use anyhow::Result;
use ethers::prelude::*;
use eth_listener::events::*;
use eth_listener::{ConfirmedBlockStream};

const WEB3_URL: &str = "wss://mainnet.infura.io/ws/v3/591481dbcf78432fa0786256ff0ef929";
const CONTRACT_ADDRESS: &str = "0xB87B33A9d9E85c6231eD66367F85700fC2EEFA86";


#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let provider = Provider::connect("ws://localhost:8545").await?;

    info!("start listening on eth net");

    let mut confirmed_stream = ConfirmedBlockStream::new(&provider, provider.get_block_number().await?.as_u64(), 3).await?;

    while let Some(block) = confirmed_stream.next().await {
        let block = block?;
        let block_number = block.number.unwrap();
        info!("current: {}, confirmed: {} {:?}", provider.get_block_number().await?.as_u64(), block_number, block.hash.unwrap());
        let log_filter = Filter::default()
            .from_block(block_number)
            .to_block(block_number)
            .address(CONTRACT_ADDRESS.parse::<Address>().unwrap());
        let events = provider
            .get_logs(&log_filter).await?
            .into_iter()
            .map(|log| Events::try_from(log))
            .collect::<Result<Vec<Events>, EventParseError>>()?;
        for event in events {
            info!("{:?}", event);
        }
    }

    Ok(())
}