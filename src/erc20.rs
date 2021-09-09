use ethers::abi::Abi;
use ethers::prelude::*;
use std::ops::Deref;

use crate::restapi::Asset;

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
    pub address: Address,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}

impl ERC20 {
    pub async fn query<M: Middleware>(client: M, address: Address) -> Self {
        let contract = Contract::new(address, ABI.deref().clone(), client);

        let symbol = contract
            .method::<_, String>("symbol", ())
            .unwrap()
            .call()
            .await
            .unwrap_or_else(|_| "".to_string());

        let name = contract
            .method::<_, String>("name", ())
            .unwrap()
            .call()
            .await
            .unwrap_or_else(|_| "".to_string());

        let decimals = contract
            .method::<_, u8>("decimals", ())
            .unwrap()
            .call()
            .await
            .unwrap();

        Self {
            address,
            symbol,
            name,
            decimals,
        }
    }
}

impl From<(ERC20, u16)> for Asset {
    fn from((erc20, token_id): (ERC20, u16)) -> Self {
        Self {
            id: erc20.symbol.clone(),
            symbol: erc20.symbol,
            name: erc20.name,
            // reference: dingir-exchange/migrations/20210223072038_markets_preset.sql
            chain_id: 1,
            token_address: format!("{:#x}", erc20.address),
            rollup_token_id: token_id as i32,
            // TODO: review this
            prec_save: 6,
            prec_show: 6,
            logo_uri: "".to_string(),
        }
    }
}
