use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use ethers::core::types::{Block, H256};
use ethers::prelude::{Middleware, Provider, ProviderError, PubsubClient, SubscriptionStream};
use futures::Stream;

#[derive(Debug, thiserror::Error)]
pub enum ConfirmedBlockSubscribeError {
    #[error("provider got error when subscribe blocks: {0}")]
    Provider(#[from] ProviderError),
}

#[derive(Debug, thiserror::Error)]
pub enum ConfirmedBlockStreamError {
    #[error("provider got error when get blocks: {0}")]
    Provider(#[from] ProviderError),
}

type PollResult = Result<Option<Block<H256>>, ProviderError>;

pub struct ConfirmedBlockStream<'a, P: PubsubClient> {
    provider: &'a Provider<P>,
    rx: Pin<Box<SubscriptionStream<'a, P, Block<H256>>>>,
    last_confirmed_block: u64,
    newest_block: u64,
    n_confirmations: u64,
    last_poll: Option<Pin<Box<(dyn Future<Output = PollResult> + 'a)>>>,
}

impl<'a, P: PubsubClient> ConfirmedBlockStream<'a, P> {
    pub async fn new(
        provider: &'a Provider<P>,
        from: u64,
        n_confirmations: u64,
    ) -> Result<ConfirmedBlockStream<'a, P>, ConfirmedBlockSubscribeError> {
        let rx = provider.subscribe_blocks().await?;
        debug!("subscribed on eth blocks");
        let newest_block = provider.get_block_number().await?.as_u64();
        debug!("current eth block is block#{}", newest_block);
        Ok(Self {
            provider,
            rx: Box::pin(rx),
            last_confirmed_block: from,
            newest_block,
            n_confirmations,
            last_poll: None,
        })
    }
}

impl<'a, P: PubsubClient> Stream for ConfirmedBlockStream<'a, P> {
    type Item = Result<Block<H256>, ConfirmedBlockStreamError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // poll future if exist
        let this = self.get_mut();
        if let Some(mut fut) = this.last_poll.take() {
            debug!("polling pending get block future");
            if let Poll::Ready(poll_result) = fut.as_mut().poll(cx) {
                trace!("get block future is ready");
                let ret = match poll_result {
                    Ok(Some(block)) => {
                        this.last_confirmed_block = block.number.unwrap().as_u64();
                        debug!(
                            "confirm block#{} (latest block at #{})",
                            this.last_confirmed_block, this.newest_block,
                        );
                        Ok(block)
                    }
                    Ok(None) => {
                        panic!("polling got empty result");
                    }
                    Err(e) => Err(e.into()),
                };
                return Poll::Ready(Some(ret));
            }
            trace!("get block future is not ready yet");
            this.last_poll.replace(fut);
            return Poll::Pending;
        }

        // assign future if there is remaining block
        if this.last_confirmed_block < this.newest_block.saturating_sub(this.n_confirmations) {
            debug!(
                "assign new future for block#{} (latest block at #{})",
                this.last_confirmed_block + 1,
                this.newest_block,
            );
            let fut = Box::pin(this.provider.get_block(this.last_confirmed_block + 1));
            this.last_poll.replace(fut);
            // immediately poll after create
            return Pin::new(this).poll_next(cx);
        }

        // poll the stream for new block
        debug!("poll underlying subscribed stream");
        let block = match futures_util::ready!(this.rx.as_mut().poll_next(cx)) {
            Some(block) => block,
            // the stream is terminated
            None => return Poll::Ready(None),
        };

        // we got new block here, update
        this.newest_block = block.number.unwrap().as_u64();
        debug!("newest_block updated to block#{}", this.newest_block);
        Pin::new(this).poll_next(cx)
    }
}
