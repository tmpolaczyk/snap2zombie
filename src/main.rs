use crate::should_be_public::parse;
use crate::snap_2_json::Snap2JsonCommand;
use crate::snap_2_json::run_snap_2_json;
use clap::Parser;
use parity_scale_codec::{Decode, DecodeAll};
use sc_executor::{
    DEFAULT_HEAP_ALLOC_STRATEGY, HeapAllocStrategy, WasmExecutor, sp_wasm_interface::HostFunctions,
};
use serde::de::DeserializeOwned;
use sp_runtime::testing::H256;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::traits::NumberFor;
use sp_runtime::{
    OpaqueExtrinsic,
    generic::{Block as BlockGeneric, Header},
    traits::BlakeTwo256,
};
use std::env;
use std::fmt::Debug;
use std::str::FromStr;
use try_runtime_core::common::shared_parameters::{Runtime, SharedParams};

mod should_be_public;

type Block = BlockGeneric<Header<u32, BlakeTwo256>, OpaqueExtrinsic>;
type HostFns = sp_io::SubstrateHostFunctions;

mod snap_2_json;

/// Possible actions of `try-runtime`.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum Action {
    Snap2Json(Snap2JsonCommand),
}

impl Action {
    async fn run<Block, HostFns>(&self, shared: &SharedParams) -> sc_cli::Result<()>
    where
        Block: BlockT + serde::de::DeserializeOwned,
        Block::Hash: serde::de::DeserializeOwned,
        Block::Header: serde::de::DeserializeOwned,
        <Block::Hash as FromStr>::Err: Debug,
        NumberFor<Block>: FromStr,
        <NumberFor<Block> as FromStr>::Err: Debug,
        HostFns: HostFunctions,
    {
        match self {
            Action::Snap2Json(cmd) => {
                run_snap_2_json::<Block, HostFns>(shared.clone(), cmd.clone()).await?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, clap::Parser)]
#[command(author, version, about)]
pub struct TryRuntime2 {
    #[clap(flatten)]
    pub shared: SharedParams,

    #[command(subcommand)]
    pub action: Action,
}

impl TryRuntime2 {
    pub async fn run<Block, HostFns>(&self) -> sc_cli::Result<()>
    where
        Block: BlockT<Hash = H256> + DeserializeOwned,
        Block::Header: DeserializeOwned,
        Block::Hash: FromStr,
        <Block::Hash as FromStr>::Err: Debug,
        <NumberFor<Block> as FromStr>::Err: Debug,
        <NumberFor<Block> as TryInto<u64>>::Error: Debug,
        NumberFor<Block>: FromStr,
        HostFns: HostFunctions,
    {
        self.action.run::<Block, HostFns>(&self.shared).await
    }
}

fn init_env() {
    if env::var(env_logger::DEFAULT_FILTER_ENV).is_err() {
        unsafe {
            env::set_var(env_logger::DEFAULT_FILTER_ENV, "info");
        }
    }
    env_logger::init();
}

#[tokio::main]
async fn main() {
    init_env();

    let cmd = TryRuntime2::parse();
    cmd.run::<Block, HostFns>().await.unwrap();
}
