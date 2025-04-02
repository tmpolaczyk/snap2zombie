use crate::BlockT;
use crate::parse;
use crate::should_be_public::build_executor;
use frame_remote_externalities::RemoteExternalities;
use sc_cli::{WasmExecutionMethod, WasmtimeInstantiationStrategy};
use sc_executor::HostFunctions;
use sp_runtime::traits::NumberFor;
use std::fmt::Debug;
use std::str::FromStr;
use try_runtime_core::common::shared_parameters::{Runtime, SharedParams};
use try_runtime_core::common::state::{RuntimeChecks, State};

/// Configurations for [`run_snap_2_json`].
#[derive(Debug, Clone, clap::Parser)]
pub struct Snap2JsonCommand {
    /// A pallet to scrape. Can be provided multiple times. If empty, entire chain state will
    /// be scraped.
    ///
    /// This is equivalent to passing `xx_hash_64(pallet)` to `--hashed_prefixes`.
    #[arg(short, long, num_args = 1..)]
    pub pallet: Vec<String>,

    /// Storage entry key prefixes to scrape and inject into the test externalities. Pass as 0x
    /// prefixed hex strings. By default, all keys are scraped and included.
    #[arg(long, value_parser = parse::hash, num_args = 1..)]
    pub prefix: Vec<String>,

    /// The snapshot path to read.
    #[clap(long)]
    pub snapshot_path: String,

    #[clap(long)]
    pub output_path: String,
}

pub async fn run_snap_2_json<Block, HostFns>(
    shared: SharedParams,
    command: Snap2JsonCommand,
) -> sc_cli::Result<()>
where
    Block: BlockT + serde::de::DeserializeOwned,
    Block::Hash: serde::de::DeserializeOwned,
    Block::Header: serde::de::DeserializeOwned,
    <Block::Hash as FromStr>::Err: Debug,
    NumberFor<Block>: FromStr,
    <NumberFor<Block> as FromStr>::Err: Debug,
    HostFns: HostFunctions,
{
    let ext = {
        // TODO: add clap
        let filename = std::env::args()
            .nth(1)
            .expect("Usage: cargo run -- dancebox.snap");
        let shared = SharedParams {
            runtime: Runtime::Existing,
            disable_spec_name_check: false,
            wasm_method: WasmExecutionMethod::Interpreted,
            wasmtime_instantiation_strategy: WasmtimeInstantiationStrategy::PoolingCopyOnWrite,
            heap_pages: None,
            export_proof: None,
            overwrite_state_version: None,
        };

        let state = State::Snap {
            path: Some(filename.into()),
        };
        let executor = build_executor::<crate::HostFns>(&shared);
        let runtime_checks = RuntimeChecks {
            name_matches: false,
            version_increases: false,
            try_runtime_feature_enabled: false,
        };

        state
            .to_ext::<crate::Block, crate::HostFns>(&shared, &executor, None, runtime_checks)
            .await?
    };

    let mut ext: RemoteExternalities<crate::Block> = ext;

    let mut sn = ext.execute_with(|| {
        let mut res = vec![];
        let mut prefix = vec![];
        while let Some(key) = sp_io::storage::next_key(&prefix) {
            let value = frame_support::storage::unhashed::get_raw(&key).unwrap();
            prefix = key.clone();
            prefix.push(0x00);

            res.push((key, (value, 0i32)));
        }

        res
    });

    /*
    // This method doesnt work, DO NOT USE IT
        // ext.into_raw_snapshot() doesn't compile ???
        let mut sn = ext
            .backend
            .backend_storage_mut()
            .drain()
            .into_iter()
            .filter(|(_, (_, r))| *r > 0)
            .collect::<Vec<(Vec<u8>, (Vec<u8>, i32))>>();
         */

    // Only keep requested pallet storage
    // PooledStaking
    let pallet_prefix = hex::decode("359e684ff9b0738b7dc97123fd114c24").unwrap();
    sn.retain(|(key, (value, refcount))| key.starts_with(&pallet_prefix));

    for (key, (value, refcount)) in sn {
        println!(
            "\"0x{}\": \"0x{}\",",
            hex::encode(&key),
            hex::encode(&value)
        );
    }

    Ok(())
}
