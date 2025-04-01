use frame_metadata::v14::{RuntimeMetadataV14, StorageEntryType};
use frame_metadata::{RuntimeMetadata, RuntimeMetadataPrefixed};
use parity_scale_codec::{Decode, DecodeAll};
use scale_info::form::PortableForm;
use scale_info::TypeDef;
use std::collections::BTreeSet;
use frame_remote_externalities::{
    Builder, Mode, OfflineConfig, OnlineConfig, RemoteExternalities, SnapshotConfig,
};
use sp_runtime::{
    generic::{Block as BlockGeneric, Header},
    traits::BlakeTwo256,
    OpaqueExtrinsic,
};
use try_runtime_core::{
    common::{
        shared_parameters,
        state::{LiveState, RuntimeChecks, State},
    },
};
use try_runtime_core::common::shared_parameters::{Runtime, SharedParams};
use sc_cli::{execution_method_from_cli, WasmExecutionMethod, WasmtimeInstantiationStrategy, DEFAULT_WASMTIME_INSTANTIATION_STRATEGY, DEFAULT_WASM_EXECUTION_METHOD};
use sp_runtime::StateVersion;
use sc_executor::{
    sp_wasm_interface::HostFunctions, HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY,
};

type Block = BlockGeneric<Header<u32, BlakeTwo256>, OpaqueExtrinsic>;
type HostFns = sp_io::SubstrateHostFunctions;

pub fn build_executor<H: HostFunctions>(shared: &SharedParams) -> WasmExecutor<H> {
    let heap_pages =
        shared
            .heap_pages
            .map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |p| HeapAllocStrategy::Static {
                extra_pages: p as _,
            });

    WasmExecutor::builder()
        .with_execution_method(execution_method_from_cli(
            shared.wasm_method,
            shared.wasmtime_instantiation_strategy,
        ))
        .with_onchain_heap_alloc_strategy(heap_pages)
        .with_offchain_heap_alloc_strategy(heap_pages)
        // There is not that much we can do if someone is using unknown host functions.
        // They would need to fork the `cli` to add their custom host functions.
        .with_allow_missing_host_functions(true)
        .build()
}

//#[tokio::main]
fn main() {
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
        let executor = build_executor::<HostFns>(&shared);
        let runtime_checks = RuntimeChecks {
            name_matches: false,
            version_increases: false,
            try_runtime_feature_enabled: false,
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let ext = rt.block_on(async {
            state
                .to_ext::<Block, HostFns>(&shared, &executor, None, runtime_checks).await}
        ).unwrap();

        ext
    };

    let mut ext: RemoteExternalities<Block> = ext;

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
    sn.retain(|(key, (value, refcount))| {
        key.starts_with(&pallet_prefix)
    });

    for (key, (value, refcount)) in sn {
        println!("\"0x{}\": \"0x{}\",", hex::encode(&key), hex::encode(&value));
    }
}