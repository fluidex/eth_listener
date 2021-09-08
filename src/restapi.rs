use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
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
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RestError {
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("request failed with error: {0}")]
    Http(StatusCode),
}

impl RestClient {
    pub fn new<P: AsRef<str>>(endpoint: P) -> Self {
        Self {
            client: reqwest::Client::default(),
            base_url: endpoint.as_ref().to_owned(),
        }
    }

    pub async fn add_assets(&self, req: &NewAssetReq) -> Result<(), RestError> {
        let url: String = format!("{}/manage/market/assets", self.base_url);
        debug!("rest-client: {:?}", req);
        let response = self.client.post(url.as_str()).json(req).send().await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(RestError::Http(status))
        }
    }
}
