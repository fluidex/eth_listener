use ethers::abi::Abi;
use ethers::prelude::*;
use std::ops::Deref;

use crate::restapi::Asset;
use hex::ToHex;

const MIN_ABI: &str = r#"[
  {
    "name":"symbol",
    "inputs":[],
    "outputs":[{"name":"","type":"string"}],
    "type":"function",
    "constant":true
  },
  {
    "name":"name",
    "inputs":[],
    "outputs":[{"name":"","type":"string"}],
    "type":"function",
    "constant":true
  },
  {
    "name":"decimals",
    "inputs":[],
    "outputs":[{"name":"","type":"uint8"}],
    "type":"function",
    "constant":true
  },
  {
    "name":"totalSupply",
    "inputs":[],
    "outputs":[{"name":"","type":"uint256"}],
    "type":"function",
    "constant":true
  }
]"#;

static ABI: Lazy<Abi> = Lazy::new(|| serde_json::from_str(MIN_ABI).unwrap());

#[derive(Debug, Clone)]
pub struct ERC20 {
    address: Address,
    symbol: String,
    name: String,
}

impl ERC20 {
    pub async fn query<P: JsonRpcClient>(client: Provider<P>, address: Address) -> Self {
        let contract = Contract::new(address.clone(), ABI.deref().clone(), client);

        let symbol = contract
            .method::<_, String>("symbol", ()).unwrap()
            .call()
            .await
            .unwrap_or_else(|_| "".to_string());

        let name = contract
            .method::<_, String>("name", ()).unwrap()
            .call()
            .await
            .unwrap_or_else(|_| "".to_string());

        Self {
            address,
            symbol,
            name,
        }
    }
}

impl From<ERC20> for Asset {
    fn from(erc20: ERC20) -> Self {
        Self {
            id: erc20.symbol.clone(),
            symbol: erc20.symbol,
            name: erc20.name,
            // reference: dingir-exchange/migrations/20210223072038_markets_preset.sql
            chain_id: 1,
            token_address: format!("0x{}", erc20.address.encode_hex()),
            rollup_token_id: 0,
            prec_save: 0,
            prec_show: 0,
            logo_uri: "".to_string()
        }
    }
}