use ethers::prelude::*;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;
use std::ops::DerefMut;
use std::cell::RefCell;

pub mod events {
    include!(concat!(env!("OUT_DIR"), "/events.rs"));
}

#[derive(Debug, thiserror::Error)]
pub enum ConfirmedBlockSubscribeError {
    #[error(transparent)]
    Provider(#[from] ProviderError),
}

#[derive(Debug, thiserror::Error)]
pub enum ConfirmedBlockStreamError {
    #[error(transparent)]
    Provider(#[from] ProviderError),
}


pub struct ConfirmedBlockStream<'a, P: PubsubClient> {
    provider: &'a Provider<P>,
    rx: RefCell<Pin<Box<SubscriptionStream<'a, P, Block<H256>>>>>,
    last_confirmed_block: RefCell<u64>,
    newest_block: RefCell<u64>,
    n_confirmations: u64,
    last_poll: RefCell<Option<Pin<Box<(dyn Future<Output=Result<Option<Block<H256>>, ProviderError>> + 'a)>>>>
}

impl <'a, P: PubsubClient> ConfirmedBlockStream<'a, P> {
    pub async fn new(
        provider: &'a Provider<P>,
        from: u64,
        n_confirmations: u64,
    ) -> Result<ConfirmedBlockStream<'a, P>, ConfirmedBlockSubscribeError> {
        let rx = provider.subscribe_blocks().await?;
        let newest_block = provider.get_block_number().await?.as_u64();
        Ok(Self {
            provider,
            rx: RefCell::new(Box::pin(rx)),
            last_confirmed_block: RefCell::new(from),
            newest_block: RefCell::new(newest_block),
            n_confirmations,
            last_poll: RefCell::new(None),
        })
    }
}

impl <'a, P: PubsubClient> Stream for ConfirmedBlockStream<'a, P> {
    type Item = Result<Block<H256>, ConfirmedBlockStreamError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // poll future if exist
        if let Some(mut fut) = self.last_poll.take() {
            if let Poll::Ready(poll_result) = fut.as_mut().poll(cx) {
                let result = match poll_result {
                    Ok(block) => {
                        *self.last_confirmed_block.borrow_mut() += 1;
                        Ok(block.unwrap())
                    },
                    Err(e) => {
                        Err(e.into())
                    }
                };
                return Poll::Ready(Some(result));
            }
            self.last_poll.replace(Some(fut));
        }

        // assign future if there is remaining block
        if *self.last_confirmed_block.borrow() < *self.newest_block.borrow() - self.n_confirmations {
            let fut = Box::pin(self.provider.get_block(*self.last_confirmed_block.borrow() + 1));
            self.last_poll.replace(Some(fut));
            // immediately poll after create
            return self.poll_next(cx);
        }

        // poll the stream for new block
        let block = match futures_util::ready!(self.rx.borrow_mut().as_mut().poll_next(cx)) {
            Some(block) => block,
            // the stream is terminated
            None => return Poll::Ready(None),
        };

        // we got new block here, update
        self.newest_block.replace(block.number.unwrap().as_u64());
        return self.poll_next(cx);
    }
}
