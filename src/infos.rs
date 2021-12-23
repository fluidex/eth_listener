use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::sync::Arc;

use ethers::prelude::*;

use crate::erc20::{LocalToken, ERC20};
use crate::restapi::Asset;
use crate::Fluidex;

#[derive(Debug, Clone)]
pub struct ContractInfos<M: Middleware> {
    provider: Arc<M>,
    contract: Fluidex<M>,
    token_ids: HashMap<u16, Address>,
    token_addresses: HashMap<Address, u16>,
    user_ids: HashMap<[u8; 32], u16>,
    erc20s: HashMap<Address, ERC20>,
}

#[derive(Debug, thiserror::Error)]
pub enum ContractInfoError {
    #[error("error when calling contract: {0}")]
    ContractError(String),
    #[error("non existing entry")]
    NonExistEntry,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Account {
    id: u16,
    pubkey: String,
}

type Result<T, E = ContractInfoError> = std::result::Result<T, E>;

impl<M: Middleware> ContractInfos<M> {
    pub async fn new(provider: Arc<M>, address: Address) -> Self {
        let contract = Fluidex::new(address, provider.clone());

        let info = ContractInfos {
            provider,
            contract,
            token_ids: HashMap::new(),
            token_addresses: HashMap::new(),
            user_ids: HashMap::new(),
            erc20s: HashMap::new(),
        };

        if cfg!(feature = "offline") {
            info!("loading tokens from local file");
            info.offline_load().await.unwrap()
        } else {
            info
        }
    }

    async fn offline_load(mut self) -> anyhow::Result<Self> {
        let path = std::env::var("LOCAL_TOKEN").unwrap_or_else(|_| "/tmp/tokens.json".to_string());
        let tokens_file = tokio::fs::read(&path).await?;
        let tokens: Vec<LocalToken> = serde_json::from_slice(tokens_file.as_slice())?;
        for (idx, token) in tokens.into_iter().enumerate() {
            let token_id = (idx + 1) as u16;
            let erc20 = ERC20::try_from(token)?;
            self.token_ids.insert(token_id, erc20.address);
            self.token_addresses.insert(erc20.address, token_id);
            self.erc20s.insert(erc20.address, erc20);
        }
        let path =
            std::env::var("LOCAL_ACCOUNTS").unwrap_or_else(|_| "/tmp/accounts.json".to_string());
        let accounts_file = tokio::fs::read(&path).await?;
        let accounts: Vec<Account> = serde_json::from_slice(accounts_file.as_slice())?;
        for account in accounts {
            let pubkey = hex::decode(account.pubkey.trim_start_matches("0x"))?
                .try_into()
                .unwrap();
            self.user_ids.insert(pubkey, account.id);
        }
        Ok(self)
    }

    pub async fn add_token(&mut self, address: Address, token_id: u16) -> Asset {
        self.token_ids.insert(token_id, address);
        self.token_addresses.insert(address, token_id);
        let erc20 = self.fetch_erc20(address).await;
        (erc20, token_id).into()
    }

    #[cfg(not(feature = "offline"))]
    pub async fn fetch_erc20(&mut self, address: Address) -> ERC20 {
        if let Some(erc20) = self.erc20s.get(&address) {
            return erc20.clone();
        }
        let erc20 = ERC20::query(&self.provider, address).await;
        self.erc20s.insert(address, erc20.clone());
        erc20
    }

    #[cfg(feature = "offline")]
    pub async fn fetch_erc20(&mut self, address: Address) -> ERC20 {
        return self.erc20s.get(&address).unwrap().clone();
    }

    pub async fn fetch_assets(&mut self, token_id: u16) -> Result<Asset> {
        let address = self.fetch_token_address(token_id).await?;
        return Ok((self.fetch_erc20(address).await, token_id).into());
    }

    #[cfg(not(feature = "offline"))]
    pub async fn fetch_token_address(&mut self, token_id: u16) -> Result<Address> {
        if let Some(address) = self.token_ids.get(&token_id) {
            return Ok(*address);
        }
        let address = self
            .contract
            .token_id_to_addr(token_id)
            .call()
            .await
            .map_err(|e| ContractInfoError::ContractError(format!("{:?}", e)))?;
        self.token_ids.insert(token_id, address);
        self.token_addresses.insert(address, token_id);
        self.add_token(address, token_id).await;
        Ok(address)
    }

    #[cfg(feature = "offline")]
    pub async fn fetch_token_address(&mut self, token_id: u16) -> Result<Address> {
        return if let Some(address) = self.token_ids.get(&token_id) {
            Ok(*address)
        } else {
            error!("trying fetch non exist token #{}", token_id);
            Err(ContractInfoError::NonExistEntry)
        };
    }

    #[cfg(not(feature = "offline"))]
    pub async fn fetch_token_id(&mut self, address: Address) -> Result<u16> {
        if let Some(token_id) = self.token_addresses.get(&address) {
            return Ok(*token_id);
        }
        let token_id = self
            .contract
            .token_addr_to_id(address)
            .call()
            .await
            .map_err(|e| ContractInfoError::ContractError(format!("{:?}", e)))?;
        self.token_ids.insert(token_id, address);
        self.token_addresses.insert(address, token_id);
        self.add_token(address, token_id).await;
        Ok(token_id)
    }

    #[cfg(feature = "offline")]
    pub async fn fetch_token_id(&mut self, address: Address) -> Result<u16> {
        return if let Some(token_id) = self.token_addresses.get(&address) {
            Ok(*token_id)
        } else {
            error!("trying fetch non exist token {}", address);
            Err(ContractInfoError::NonExistEntry)
        };
    }

    #[cfg(not(feature = "offline"))]
    pub async fn fetch_user_id(&mut self, pubkey: &[u8; 32]) -> Result<u16> {
        if let Some(user_id) = self.user_ids.get(pubkey) {
            return Ok(*user_id);
        }
        let user_id = self
            .contract
            .user_bjj_pubkey_to_user_id(*pubkey)
            .call()
            .await
            .map_err(|e| ContractInfoError::ContractError(format!("{:?}", e)))?;
        self.user_ids.insert(*pubkey, user_id);
        Ok(user_id)
    }

    #[cfg(feature = "offline")]
    pub async fn fetch_user_id(&mut self, pubkey: &[u8; 32]) -> Result<u16> {
        return if let Some(user_id) = self.user_ids.get(pubkey) {
            Ok(*user_id)
        } else {
            error!(
                "trying fetch non exist user {}, current have: {:?}",
                hex::encode(pubkey),
                self.user_ids
                    .keys()
                    .map(hex::encode)
                    .collect::<Vec<String>>()
            );
            Err(ContractInfoError::NonExistEntry)
        };
    }
}

#[cfg(test)]
mod tests {
    use ethers::prelude::*;
    use std::convert::TryFrom;
    use std::str::FromStr;

    use super::*;

    const INFURA: &'static str = "https://goerli.infura.io/v3/71e500f0f56944fa80641312fdd9a6a4";
    const CONTRACT_ADDRESS: &'static str = "0x1e8b07682E5ED8e7a666605a78B74cBdc7dC9455";

    const ERC20_1: &'static str = "0x46490225a85ddfd9d79256f8c5393c0428121488";
    const USER_PK_1: &'static str =
        "5d182c51bcfe99583d7075a7a0c10d96bef82b8a059c4bf8c5f6e7124cf2bba3";

    #[tokio::test]
    async fn test_read() {
        let provider = Arc::new(Provider::try_from(INFURA).unwrap());
        let mut contract_info =
            ContractInfos::new(provider, CONTRACT_ADDRESS.parse().unwrap()).await;

        // read erc20
        let address = contract_info.fetch_token_address(1).await.unwrap();
        assert_eq!(Address::from_str(ERC20_1).unwrap(), address);

        // read asset
        let asset = contract_info.fetch_assets(1).await.unwrap();
        assert_eq!("USDT", asset.id);
        assert_eq!("USDT", asset.symbol);
        assert_eq!("Tether USD (Fluidex Test)", asset.name);
        assert_eq!(1, asset.chain_id);
        assert_eq!(ERC20_1, asset.token_address.to_ascii_lowercase());
        assert_eq!(1, asset.rollup_token_id);
        assert_eq!(6, asset.prec_save);
        assert_eq!(6, asset.prec_show);

        // read token_id
        let token_id = contract_info
            .fetch_token_id(ERC20_1.parse().unwrap())
            .await
            .unwrap();
        assert_eq!(1, token_id);

        // read user_id
        let pubkey = hex::decode(USER_PK_1).unwrap();
        let user_id = contract_info
            .fetch_user_id(pubkey.as_slice().try_into().unwrap())
            .await
            .unwrap();
        assert_eq!(1, user_id);
    }
}
