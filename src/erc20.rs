use ethers::abi::Abi;
use ethers::prelude::*;
use serde::Deserialize;
use std::convert::TryFrom;
use std::ops::Deref;
use std::str::FromStr;

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

#[derive(Debug, Clone, Deserialize)]
pub struct LocalToken {
    pub symbol: String,
    pub address: String,
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

impl TryFrom<LocalToken> for ERC20 {
    type Error = <Address as FromStr>::Err;

    fn try_from(token: LocalToken) -> std::result::Result<Self, Self::Error> {
        let address = token.address.parse()?;
        Ok(Self {
            address,
            symbol: token.symbol.clone(),
            name: token.symbol,
            decimals: 6,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;
    use tokio_tungstenite::connect_async;

    const INFURA: &'static str = "https://goerli.infura.io/v3/71e500f0f56944fa80641312fdd9a6a4";
    const INFURA_WS: &'static str = "wss://goerli.infura.io/ws/v3/71e500f0f56944fa80641312fdd9a6a4";

    const TEST_TOKEN: &'static str = "0x83658bb4bf0fc6780e6cc6170aacc4de9d700226";

    #[tokio::test]
    async fn test() {
        let provider = Provider::try_from(INFURA).unwrap();

        let token = ERC20::query(provider, TEST_TOKEN.parse().unwrap()).await;
        assert_eq!("USDT", token.symbol);
        assert_eq!("Tether USD (Fluidex Test)", token.name);
        assert_eq!(6, token.decimals);
    }

    #[tokio::test]
    async fn test_ws() {
        let (ws, _) = connect_async(INFURA_WS).await.unwrap();
        let ws = Ws::new(ws);
        let provider = Provider::new(ws);

        let token = ERC20::query(provider, TEST_TOKEN.parse().unwrap()).await;
        assert_eq!("USDT", token.symbol);
        assert_eq!("Tether USD (Fluidex Test)", token.name);
        assert_eq!(6, token.decimals);
    }
}
