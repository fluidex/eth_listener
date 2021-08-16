use std::convert::TryFrom;

use anyhow::Result;
use ethers::prelude::*;
use eth_listener::events::*;

const WEB3_URL: &str = "ws://localhost:8545";
const CONTRACT_ADDRESS: &str = "0x9324F27714f4461E3c146810fe1Be202bDB0d4e5";


#[tokio::main]
async fn main() -> Result<()> {

    let provider = Provider::connect(WEB3_URL).await?;
    let filter = Filter::default()
        .address(CONTRACT_ADDRESS.parse::<H160>().unwrap());

    let mut log_stream = provider.subscribe_logs(&filter).await?;

    println!("start listening on eth net");

    while let Some(event) = log_stream.next().await {
        let height = event.block_number.unwrap().as_u64();
        match Events::try_from(event) {
            Ok(event) => match event {
                Events::RegisterUser(register_user) => println!("{} {:?}", height, register_user),
                Events::NewToken(new_token) => println!("{} {:?}", height, new_token),
                Events::Deposit(deposit) => println!("{} {:?}", height, deposit),
            }
            Err(e) => println!("{:?}", e)
        }
    }

    Ok(())
}