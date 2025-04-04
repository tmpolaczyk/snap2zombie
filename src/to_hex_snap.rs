use crate::parse;
use crate::should_be_public::build_executor;
use crate::BlockT;
use frame_remote_externalities::RemoteExternalities;
use sc_executor::HostFunctions;
use sp_runtime::app_crypto::sp_core::twox_128;
use sp_runtime::traits::NumberFor;
use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::mem;
use std::str::FromStr;
use try_runtime_core::common::shared_parameters::SharedParams;
use try_runtime_core::common::state::{RuntimeChecks, State};

/// Configurations for [`to_hex_snap`].
#[derive(Debug, Clone, clap::Parser)]
pub struct ToHexSnapCommand {
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

pub async fn to_hex_snap<Block, HostFns>(
    shared: SharedParams,
    command: ToHexSnapCommand,
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
    // Only keep requested pallet storage
    // PooledStaking
    //let pallet_prefix = hex::decode("359e684ff9b0738b7dc97123fd114c24").unwrap();
    let keep_prefixes = command
        .prefix
        .into_iter()
        .map(|x| {
            hex::decode(&x).unwrap_or_else(|_e| {
                panic!(
                    "Failed to parse prefix key, should be in hex format (without leading 0x): {}",
                    x
                )
            })
        })
        .chain(
            command
                .pallet
                .into_iter()
                .map(|pallet_name| twox_128(pallet_name.as_bytes()).to_vec()),
        )
        .collect::<Vec<_>>();

    if !keep_prefixes.is_empty() {
        log::info!(
            "Will only keep prefixes: {:#?}",
            keep_prefixes
                .iter()
                .map(|x| hex::encode(x))
                .collect::<Vec<_>>()
        );
    }

    let mut output_file = File::create(command.output_path).inspect_err(|e| {
        log::error!("Failed to create output file: {}", e);
    })?;

    let ext = {
        let filename = command.snapshot_path;

        let state = State::Snap {
            path: Some(filename.into()),
        };
        let executor = build_executor::<HostFns>(&shared);
        let runtime_checks = RuntimeChecks {
            name_matches: false,
            version_increases: false,
            try_runtime_feature_enabled: false,
        };

        state
            .to_ext::<Block, HostFns>(&shared, &executor, None, runtime_checks)
            .await?
    };

    let mut ext: RemoteExternalities<Block> = ext;

    for (key, value) in storage_iter(&mut ext) {
        if !keep_prefixes.is_empty()
            && !keep_prefixes
                .iter()
                .any(|pallet_prefix| key.starts_with(pallet_prefix))
        {
            // Skip this key as it doesn't match any of the requested prefixes
            continue;
        }

        writeln!(
            output_file,
            "\"0x{}\": \"0x{}\",",
            hex::encode(&key),
            hex::encode(&value)
        )?;
    }

    Ok(())
}

/*
// This method doesnt work, DO NOT USE IT
// The resulting "keys" and "values" are not the key and values that you see in the runtime,
// but something else
// ext.into_raw_snapshot() doesn't compile ???
let mut sn = ext
    .backend
    .backend_storage_mut()
    .drain()
    .into_iter()
    .filter(|(_, (_, r))| *r > 0)
    .collect::<Vec<(Vec<u8>, (Vec<u8>, i32))>>();
 */

// Old version that loads all the storage key values into memory
#[allow(unused)]
pub fn storage_iter_in_mem<Block>(
    ext: &mut RemoteExternalities<Block>,
) -> impl Iterator<Item = (Vec<u8>, Vec<u8>)>
where
    Block: BlockT + serde::de::DeserializeOwned,
    Block::Hash: serde::de::DeserializeOwned,
    Block::Header: serde::de::DeserializeOwned,
    <Block::Hash as FromStr>::Err: Debug,
    NumberFor<Block>: FromStr,
    <NumberFor<Block> as FromStr>::Err: Debug,
{
    ext.execute_with(|| {
        let mut res = vec![];
        let mut prefix = vec![];
        while let Some(key) = sp_io::storage::next_key(&prefix) {
            let value = frame_support::storage::unhashed::get_raw(&key).unwrap();
            prefix = key.clone();

            res.push((key, value));
        }

        res.into_iter()
    })
}

struct StorageIter<'a, Block>
where
    Block: BlockT + serde::de::DeserializeOwned,
    Block::Hash: serde::de::DeserializeOwned,
    Block::Header: serde::de::DeserializeOwned,
    <Block::Hash as FromStr>::Err: Debug,
    NumberFor<Block>: FromStr,
    <NumberFor<Block> as FromStr>::Err: Debug,
{
    ext: &'a mut RemoteExternalities<Block>,
    prefix: Vec<u8>,
}

impl<'a, Block> Iterator for StorageIter<'a, Block>
where
    Block: BlockT + serde::de::DeserializeOwned,
    Block::Hash: serde::de::DeserializeOwned,
    Block::Header: serde::de::DeserializeOwned,
    <Block::Hash as FromStr>::Err: Debug,
    NumberFor<Block>: FromStr,
    <NumberFor<Block> as FromStr>::Err: Debug,
{
    type Item = (Vec<u8>, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        self.ext.execute_with(|| {
            let key = sp_io::storage::next_key(&self.prefix)?;
            let value = frame_support::storage::unhashed::get_raw(&key).unwrap();
            self.prefix = key.clone();

            Some((key, value))
        })
    }
}

/// Iterate over all storage items. There should be a similar function somewhere in [`frame_support`] but I cannot find it.
pub fn storage_iter<Block>(
    ext: &mut RemoteExternalities<Block>,
) -> impl Iterator<Item = (Vec<u8>, Vec<u8>)> + '_
where
    Block: BlockT + serde::de::DeserializeOwned,
    Block::Hash: serde::de::DeserializeOwned,
    Block::Header: serde::de::DeserializeOwned,
    <Block::Hash as FromStr>::Err: Debug,
    NumberFor<Block>: FromStr,
    <NumberFor<Block> as FromStr>::Err: Debug,
{
    StorageIter {
        ext,
        prefix: vec![],
    }
}
