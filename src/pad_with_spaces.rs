use crate::parse;
use crate::should_be_public::build_executor;
use crate::to_hex_snap::{storage_iter, ToHexSnapCommand};
use crate::BlockT;
use frame_remote_externalities::RemoteExternalities;
use regex::Regex;
use sc_executor::HostFunctions;
use sp_runtime::app_crypto::sp_core::twox_128;
use sp_runtime::traits::NumberFor;
use std::cmp::min;
use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::str::FromStr;
use std::{fs, mem};
use tempfile::NamedTempFile;
use try_runtime_core::common::shared_parameters::SharedParams;
use try_runtime_core::common::state::{RuntimeChecks, State};

/// Configurations for [`pad_with_spaces`].
#[derive(Debug, Clone, clap::Parser)]
pub struct PadWithSpacesCommand {
    /// The input chain spec path to read. The chain spec must be in raw format.
    #[clap(long)]
    pub chain_spec_path: String,

    /// Output path, defaults to input chain spec path
    #[clap(long)]
    pub output_path: Option<String>,

    /// Character to use for padding, default 32 (whitespace)
    #[clap(long)]
    pub ascii_code: Option<u8>,

    /// Target size in bytes, default 2GiB
    #[clap(long)]
    pub target_size: Option<u64>,
}

pub async fn pad_with_spaces<Block, HostFns>(
    shared: SharedParams,
    command: PadWithSpacesCommand,
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
    // If output path is not none, first copy input to output, and set path to output file
    if let Some(output_path) = command.output_path.clone() {
        fs::copy(&command.chain_spec_path, output_path).inspect_err(|e| {
            log::error!("Failed to open input or output file: {}", e);
        })?;
    }

    // Now, always modify this input_path in place
    let input_path = command
        .output_path
        .unwrap_or_else(|| command.chain_spec_path.clone());

    // Print the current size of the file.
    let metadata = fs::metadata(&input_path)?;
    let final_size = metadata.len();
    log::info!("Current file size: {} bytes", final_size);
    log::info!("Target file size:  {} bytes", final_size);

    // If the file is already at or above 2 GiB, no padding is needed.
    let target_size: u64 = command.target_size.unwrap_or(2 * 1024 * 1024 * 1024);
    if final_size >= target_size {
        log::info!("No padding needed");
        return Ok(());
    }

    // Calculate the number of padding bytes required.
    let padding_needed = target_size - final_size;
    log::info!("Padding file with {} additional bytes", padding_needed);

    // Open the file in append mode.
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(&input_path)?;

    // Determine the byte to pad with (default is space, ASCII 0x20).
    let pad_byte = command.ascii_code.unwrap_or(0x20);

    // Define a chunk size (here we use 1 MiB) to write in manageable pieces.
    const CHUNK_SIZE: usize = 1024 * 1024;
    let chunk = vec![pad_byte; CHUNK_SIZE];

    // Write the padding in chunks.
    let mut bytes_remaining = padding_needed as usize;
    while bytes_remaining > 0 {
        let bytes_to_write = min(CHUNK_SIZE, bytes_remaining);
        file.write_all(&chunk[..bytes_to_write])?;
        bytes_remaining -= bytes_to_write;
    }

    // Ensure all data is flushed to disk.
    file.flush()?;

    Ok(())
}
