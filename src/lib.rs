use ethers::prelude::*;

pub mod events {
    include!(concat!(env!("OUT_DIR"), "/events.rs"));
}

pub struct EthListener<'a, T: PubsubClient> {
    subscribe: SubscriptionStream<'a, T, Log>
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ProviderError(#[from] ProviderError)
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl<'a, T: PubsubClient> EthListener<'a, T> {
    pub async fn new(provider: &'a Provider<T>, filter: &Filter) -> Result<EthListener<'a, T>> {
        Ok(Self {
            subscribe: provider.subscribe_logs(filter).await?
        })
    }

    async fn foo(&self) -> Result<()> {
        Ok(())
    }

}