use std::{env, fs};

use once_cell::sync::Lazy;
use serde::Deserialize;

pub static CONFIG: Lazy<Config> = Lazy::new(Config::init);

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    web3: Web3,
    exchange: Exchange,
    storage: Storage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Web3 {
    web3_ws: Option<String>,
    web3_http: Option<String>,
    #[serde(default)]
    network: String,
    infura_api_key: Option<String>,
    contract_address: String,
    inner_contract_address: String,
    base_block: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Exchange {
    grpc_endpoint: String,
    rest_endpoint: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Storage {
    db: String,
}

impl Config {
    fn init() -> Self {
        let file = fs::read_to_string(
            env::var("LISTENER_CONFIG").unwrap_or_else(|_| "config.toml".to_string()),
        )
        .unwrap();
        toml::from_str(&file).unwrap()
    }

    pub fn web3(&'static self) -> &'static Web3 {
        &self.web3
    }

    pub fn exchange(&'static self) -> &'static Exchange {
        &self.exchange
    }

    pub fn storage(&'static self) -> &'static Storage {
        &self.storage
    }
}

impl Default for Web3 {
    fn default() -> Self {
        Self {
            web3_ws: None,
            web3_http: None,
            network: "goerli".to_string(),
            infura_api_key: None,
            contract_address: "".to_string(),
            inner_contract_address: "".to_string(),
            base_block: 0,
        }
    }
}

impl Web3 {
    pub fn web3_http(&'static self) -> String {
        self.web3_http
            .clone()
            .unwrap_or_else(|| self.infura_http().unwrap())
    }
    pub fn web3_ws(&'static self) -> String {
        self.web3_ws
            .clone()
            .unwrap_or_else(|| self.infura_ws().unwrap())
    }
    pub fn infura_http(&'static self) -> Option<String> {
        self.infura_api_key
            .as_ref()
            .map(|key| format!("https://{}.infura.io/v3/{}", self.network, key))
    }
    pub fn infura_ws(&'static self) -> Option<String> {
        self.infura_api_key
            .as_ref()
            .map(|key| format!("wss://{}.infura.io/ws/v3/{}", self.network, key))
    }
    pub fn contract_address(&'static self) -> &'static str {
        &self.contract_address
    }
    pub fn inner_contract_address(&'static self) -> &'static str {
        &self.inner_contract_address
    }
    pub fn base_block(&self) -> u64 {
        self.base_block
    }
}

impl Exchange {
    pub fn grpc_endpoint(&'static self) -> &'static str {
        &self.grpc_endpoint
    }
    pub fn rest_endpoint(&'static self) -> &'static str {
        &self.rest_endpoint
    }
}

impl Storage {
    pub fn db(&'static self) -> &'static str {
        &self.db
    }
}
