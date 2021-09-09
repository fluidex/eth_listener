use crate::erc20::ERC20;
use crate::restapi::Asset;
use ethers::abi::Abi;
use ethers::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

const CONTRACT_ABI: &str =
    include_str!("../contracts/artifacts/contracts/Fluidex.sol/FluidexDemo.json");

#[derive(Deserialize)]
struct ContractMeta {
    abi: Abi,
}

#[derive(Debug, Clone)]
pub struct ContractInfos<'a, P> {
    provider: &'a Provider<P>,
    contract: Contract<&'a Provider<P>>,
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

impl<'a, P: JsonRpcClient> ContractInfos<'a, P> {
    pub fn new(provider: &'a Provider<P>, address: Address) -> Self {
        let meta: ContractMeta = serde_json::from_str(CONTRACT_ABI).unwrap();
        let contract = Contract::new(address, meta.abi, provider);

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
        let erc20 = ERC20::query(self.provider, address).await;
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
            .method::<u16, Address>("tokenIdToAddr", token_id)
            .unwrap()
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
            .method::<Address, u16>("tokenAddrToId", address)
            .unwrap()
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
            .method::<[u8; 32], u16>("userBjjPubkeyToUserId", *pubkey)
            .unwrap()
            .call()
            .await
            .map_err(|e| ContractInfoError::ContractError(format!("{:?}", e)))?;
        self.user_ids.insert(*pubkey, user_id);
        Ok(user_id)
    }
}
