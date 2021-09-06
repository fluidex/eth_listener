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
    web3_url: String,
    contract_address: String,
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

impl Web3 {
    pub fn web3_url(&'static self) -> &'static str {
        &self.web3_url
    }
    pub fn contract_address(&'static self) -> &'static str {
        &self.contract_address
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
