use crate::BlockT;
use crate::parse;
use crate::should_be_public::build_executor;
use crate::to_json::{ToJsonCommand, storage_iter};
use frame_remote_externalities::RemoteExternalities;
use regex::Regex;
use sc_executor::HostFunctions;
use sp_runtime::app_crypto::sp_core::twox_128;
use sp_runtime::traits::NumberFor;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::str::FromStr;
use std::{fs, mem};
use tempfile::NamedTempFile;
use try_runtime_core::common::shared_parameters::SharedParams;
use try_runtime_core::common::state::{RuntimeChecks, State};

/// Configurations for [`merge_into_raw`].
#[derive(Debug, Clone, clap::Parser)]
pub struct MergeIntoRawCommand {
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

    /// The input chain spec path to read. The chain spec must be in raw format.
    #[clap(long)]
    pub chain_spec_path: String,

    /// The snapshot path to read. Must be in hex format, the output of the [`to_json`]  subcommand.
    #[clap(long)]
    pub hex_snapshot_path: String,

    /// Output path, defaults to input chain spec path
    #[clap(long)]
    pub output_path: Option<String>,

    /// Remove ALL keys from original chain spec, copy all from the snapshot.
    #[clap(long)]
    pub all: bool,
}

pub async fn merge_into_raw<Block, HostFns>(
    shared: SharedParams,
    command: MergeIntoRawCommand,
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
            "Will remove these key prefixes from original chain spec, and copy them from the hex snapshot: {:#?}",
            keep_prefixes
                .iter()
                .map(|x| hex::encode(x))
                .collect::<Vec<_>>()
        );
    }

    if keep_prefixes.is_empty() && !command.all {
        // Not sure if this will work, probably not because the result will have a duplicate pallet version key
        panic!("Add at least one --pallet arg, or pass --all flag");
    }

    // Convert each prefix (Vec<u8>) to a hex string.
    let prefix_hexes: Vec<String> = keep_prefixes.iter().map(|p| hex::encode(p)).collect();

    // Build a regex pattern that matches a line whose key (inside quotes) starts with "0x" and then one of the prefixes,
    // followed by any hexadecimal digits, then a closing quote, optional whitespace, and a colon.
    // For example:    "0x359e684f...":
    let deletion_pattern = format!(r#"^\s*"0x(?:{})[0-9a-fA-F]*"\s*:"#, prefix_hexes.join("|"));
    let deletion_regex = Regex::new(&deletion_pattern).expect("Invalid deletion regex");

    // If output path is none, overwrite input file as the last step
    let output_path = command
        .output_path
        .unwrap_or_else(|| command.chain_spec_path.clone());
    // input_path: command.chain_spec_path
    // patch_path: command.hex_snapshot_path

    // This needs to open input file, copy to output
    // Remove all storage keys that match prefixes
    // If --all flag is set, remove all storage keys
    // Then paste the snapshot keys into the raw spec, without loading into memory, just plain cat or whatever
    // And finally move the output to input if the file is the same

    // The insertion pattern: for example, a line like:    "top": {
    let insertion_pattern = r#""top":\s*\{"#;
    let top_regex = Regex::new(insertion_pattern).expect("Invalid insertion pattern regex");

    let mut temp = NamedTempFile::new_in(Path::new(&command.chain_spec_path).parent().unwrap())?;
    let input = File::open(command.chain_spec_path).inspect_err(|e| {
        log::error!("Failed to open chain spec file: {}", e);
    })?;
    let reader = BufReader::new(input);
    let patch_input = File::open(command.hex_snapshot_path).inspect_err(|e| {
        log::error!("Failed to open hex snapshot file: {}", e);
    })?;
    let mut patch_reader = Some(BufReader::new(patch_input));
    let mut count_removed_keys = 0u64;
    let mut count_inserted_keys = 0u64;
    let mut count_skipped_from_snap = 0u64;
    {
        let mut writer = BufWriter::new(&mut temp);
        // This is a weird state machine that depends on which line we are processing currently
        let mut inserted = false;
        let mut inside_top = false;
        let mut trailing_comma_edge_case = false;
        for line in reader.lines() {
            let line = line?;
            if inside_top && line.contains("}") {
                // End of top object
                inside_top = false;
                if trailing_comma_edge_case {
                    log::warn!("Need to manually remove trailing comma from top object");
                }
            }
            if inside_top && (command.all || deletion_regex.is_match(&line)) {
                // Skip this line (i.e. delete it)
                count_removed_keys += 1;
                continue;
            }
            // Copy line to output
            writeln!(writer, "{}", line)?;
            trailing_comma_edge_case = false;

            // Find start of "top" object
            if !inserted && top_regex.is_match(&line) {
                inside_top = true;
                // Stream the patch file line by line and write to output.
                let patch_reader = patch_reader.take().unwrap();
                for patch_line in patch_reader.lines() {
                    let patch_line = patch_line?;
                    if command.all || deletion_regex.is_match(&patch_line) {
                        writeln!(writer, "{}", patch_line)?;
                        count_inserted_keys += 1;
                    } else {
                        count_skipped_from_snap += 1;
                    }
                }
                inserted = true;
                // TODO: Edge case, if there are no more keys in top object, we need to remove trailing comma
                // Detect that but do not fix it
                trailing_comma_edge_case = true;
            }
        }
        writer.flush()?;
    }

    log::info!(
        "Removed {} keys from existing chain spec",
        count_removed_keys
    );
    log::info!("Inserted {} new keys from snapshot", count_inserted_keys);
    if count_skipped_from_snap > 0 {
        log::info!(
            "{} keys not inserted from snapshot based on pallet prefix",
            count_skipped_from_snap
        );
    }

    temp.persist(&output_path)
        .expect("Failed to persist output file");

    // Now, print the size in bytes of the final file.
    let metadata = fs::metadata(output_path)?;
    let final_size = metadata.len();
    log::info!("Final file size: {} bytes", final_size);
    if final_size < 2 * 1024 * 1024 * 1024 {
        log::warn!(
            "Output file size is less than 2GB, zombienet will attempt to modify it and that may fail"
        );
        log::warn!("Use pad-with-spaces subcommand to workaround that");
    }

    Ok(())
}

// Format of the chain spec file:
/*
{
"name": "Dancebox Local Testnet",
"id": "dancebox_local",
"chainType": "Local",
"bootNodes": [
"/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWHymDsiF5GSgmMR5H3E8tnwypQUzixpTmVBgfpH9ebPnu"
],
"telemetryEndpoints": null,
"protocolId": "orchestrator",
"properties": {
"isEthereum": false,
"ss58Format": 42,
"tokenDecimals": 12,
"tokenSymbol": "DANCE"
},
"relay_chain": "rococo-local",
"para_id": 1000,
"codeSubstitutes": {},
"genesis": {
"raw": {
  "top": {
"0x359e684ff9b0738b7dc97123fd114c2447e452f56d134f8b40e99dab668d6d83000ea0654bc248049f933dd471e5ef785c92fd693ef1dcc643d5570e19528d971f05fb678efebd7995f6aeb929df0f4d0015cea5639b505698ba1db4cddfd0fe01f4c786f895105cd28ee7acf453fc03e91503616fda5843670a777f617156bb0312360000": "0x0068292f260000000000000000000000",
"0x359e684ff9b0738b7dc97123fd114c2447e452f56d134f8b40e99dab668d6d83000ea0654bc248049f933dd471e5ef785c92fd693ef1dcc643d5570e19528d971f05fb678efebd7995f6aeb929df0f4d4d8d5f98db130fef2ca5f1d2408bf17500f4c786f895105cd28ee7acf453fc03e91503616fda5843670a777f617156bb0312360000": "0x0068292f260000000000000000000000",

...
 */

// Format of the patch file:

/*
"0x359e684ff9b0738b7dc97123fd114c2447e452f56d134f8b40e99dab668d6d83000ea0654bc248049f933dd471e5ef785c92fd693ef1dcc643d5570e19528d971f05fb678efebd7995f6aeb929df0f4d0015cea5639b505698ba1db4cddfd0fe01f4c786f895105cd28ee7acf453fc03e91503616fda5843670a777f617156bb0312360000": "0x0068292f260000000000000000000000",
"0x359e684ff9b0738b7dc97123fd114c2447e452f56d134f8b40e99dab668d6d83000ea0654bc248049f933dd471e5ef785c92fd693ef1dcc643d5570e19528d971f05fb678efebd7995f6aeb929df0f4d4d8d5f98db130fef2ca5f1d2408bf17500f4c786f895105cd28ee7acf453fc03e91503616fda5843670a777f617156bb0312360000": "0x0068292f260000000000000000000000",
"0x359e684ff9b0738b7dc97123fd114c2447e452f56d134f8b40e99dab668d6d8301325c8173a1762cfa6fa719e2530877067d7c05e9351aec66048e38afb1b3caa71ed6330341bc3fbdbe9d77225d6d363ab1a8910b98c09619caba7cdb27fafd003a95afb26ed32825195e4457d55f4eda83a9992112f3284e8b3f826c0dce484a12360000": "0x005039278c0400000000000000000000",
 */
