use ethers::abi::Abi;
use ethers::prelude::*;
use std::ops::Deref;

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
    symbol: String,
    name: String,
    decimals: u8,
}

impl ERC20 {
    pub async fn query<P: JsonRpcClient>(client: Provider<P>, address: Address) -> Self {
        let contract = Contract::new(address, ABI.deref().clone(), client);

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

        let decimals = contract
            .method::<_, u8>("u8", ()).unwrap()
            .call()
            .await
            .unwrap_or(18);

        Self {
            symbol,
            name,
            decimals,
        }
    }
}