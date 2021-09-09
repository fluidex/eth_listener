#[macro_use]
extern crate log;

pub use orchestra::rpc::exchange;

pub use crate::block_stream::ConfirmedBlockStream;
pub use crate::config::CONFIG;
pub use crate::fluidex::Fluidex;

pub mod block_stream;
pub mod config;
pub mod erc20;
pub mod infos;
pub mod persist;
pub mod restapi;

pub mod events {
    include!(concat!(env!("OUT_DIR"), "/events.rs"));
}

pub mod fluidex {
    include!(concat!(env!("OUT_DIR"), "/fluidex.rs"));
}
