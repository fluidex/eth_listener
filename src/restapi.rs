use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct NewAssetReq {
    pub assets: Vec<Asset>,
    #[serde(default)]
    pub not_reload: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Asset {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub chain_id: i16,
    pub token_address: String,
    pub rollup_token_id: i32,
    pub prec_save: u32,
    pub prec_show: u32,
    pub logo_uri: String,
}

pub struct RestClient {

};