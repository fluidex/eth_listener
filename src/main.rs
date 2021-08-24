#[macro_use]
extern crate log;

use std::convert::TryFrom;

use anyhow::Result;
use ethers::prelude::*;
use eth_listener::events::*;
use eth_listener::{ConfirmedBlockStream, ConfirmedBlockStreamError};

const WEB3_URL: &str = "wss://mainnet.infura.io/ws/v3/591481dbcf78432fa0786256ff0ef929";
const CONTRACT_ADDRESS: &str = "0x9324F27714f4461E3c146810fe1Be202bDB0d4e5";


#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let provider = Provider::connect(WEB3_URL).await?;

    info!("start listening on eth net");

    let mut confirmed_stream = ConfirmedBlockStream::new(&provider, provider.get_block_number().await?.as_u64(), 3).await?;

    while let Some(block) = confirmed_stream.next().await {
        match block {
            Ok(block) => info!("current: {}, confirmed: {} {:?}", provider.get_block_number().await?.as_u64(), block.number.unwrap(), block.hash.unwrap()),
            Err(e) => error!("error: {:?}", e)
        }
    }

    Ok(())
}