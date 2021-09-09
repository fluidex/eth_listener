use std::collections::HashMap;
use std::sync::Arc;

use ethers::prelude::*;

use crate::erc20::ERC20;
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
}

type Result<T, E = ContractInfoError> = std::result::Result<T, E>;

impl<M: Middleware> ContractInfos<M> {
    pub fn new(provider: Arc<M>, address: Address) -> Self {
        let contract = Fluidex::new(address, provider.clone());

        ContractInfos {
            provider,
            contract,
            token_ids: HashMap::new(),
            token_addresses: HashMap::new(),
            user_ids: HashMap::new(),
            erc20s: HashMap::new(),
        }
    }

    pub async fn add_token(&mut self, address: Address, token_id: u16) -> Asset {
        self.token_ids.insert(token_id, address);
        self.token_addresses.insert(address, token_id);
        let erc20 = self.fetch_erc20(address).await;
        (erc20, token_id).into()
    }

    pub async fn fetch_erc20(&mut self, address: Address) -> ERC20 {
        if let Some(erc20) = self.erc20s.get(&address) {
            return erc20.clone();
        }
        let erc20 = ERC20::query(&self.provider, address).await;
        self.erc20s.insert(address, erc20.clone());
        erc20
    }

    pub async fn fetch_assets(&mut self, token_id: u16) -> Result<Asset> {
        let address = self.fetch_token_address(token_id).await?;
        return Ok((self.fetch_erc20(address).await, token_id).into());
    }

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
}
