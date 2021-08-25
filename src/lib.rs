#[macro_use]
extern crate log;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use ethers::prelude::*;
use futures::Stream;
pub use orchestra::rpc::exchange;

pub use crate::config::CONFIG;
pub use crate::block_stream::ConfirmedBlockStream;

pub mod config;
pub mod block_stream;

pub mod events {
    include!(concat!(env!("OUT_DIR"), "/events.rs"));
}