use crate::merge_into_raw::{MergeIntoRawCommand, merge_into_raw};
use crate::pad_with_spaces::{PadWithSpacesCommand, pad_with_spaces};
use crate::should_be_public::parse;
use crate::to_json::ToJsonCommand;
use crate::to_json::to_json;
use clap::Parser;
use sc_executor::sp_wasm_interface::HostFunctions;
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
use try_runtime_core::commands::create_snapshot;
use try_runtime_core::common::shared_parameters::SharedParams;

mod merge_into_raw;
mod pad_with_spaces;
mod should_be_public;
mod to_json;

type Block = BlockGeneric<Header<u32, BlakeTwo256>, OpaqueExtrinsic>;
type HostFns = sp_io::SubstrateHostFunctions;

/// Possible actions of `snap2zombie`.
#[derive(Debug, Clone, clap::Subcommand)]
pub enum Action {
    /// Convert snaphost to hex json format
    ToJson(ToJsonCommand),
    /// Merge hex snapshot into raw chain spec file
    MergeIntoRaw(MergeIntoRawCommand),
    /// Increase size of a file by padding with a single byte
    PadWithSpaces(PadWithSpacesCommand),
    /// Re-export of create-snapshot command from try-runtime, to avoid an extra cargo install if
    /// the user does not have try-runtime already installed.
    CreateSnapshot(create_snapshot::Command),
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
            Action::ToJson(cmd) => {
                to_json::<Block, HostFns>(shared.clone(), cmd.clone()).await?;
            }
            Action::MergeIntoRaw(cmd) => {
                merge_into_raw::<Block, HostFns>(shared.clone(), cmd.clone()).await?;
            }
            Action::PadWithSpaces(cmd) => {
                pad_with_spaces::<Block, HostFns>(shared.clone(), cmd.clone()).await?;
            }
            Action::CreateSnapshot(cmd) => {
                create_snapshot::run::<Block, HostFns>(shared.clone(), cmd.clone()).await?;
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
        // Safety: actually unsound because `tokio::main` starts a multithreaded runtime, so if they
        // decide to call `set_var` somewhere we got a race condition.
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
