use sc_cli::execution_method_from_cli;
use sc_executor::{DEFAULT_HEAP_ALLOC_STRATEGY, HeapAllocStrategy, HostFunctions, WasmExecutor};
use try_runtime_core::common::shared_parameters::SharedParams;

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

pub mod parse {
    pub fn hash(block_hash: &str) -> Result<String, String> {
        let (block_hash, offset) = if let Some(block_hash) = block_hash.strip_prefix("0x") {
            (block_hash, 2)
        } else {
            (block_hash, 0)
        };

        if let Some(pos) = block_hash.chars().position(|c| !c.is_ascii_hexdigit()) {
            Err(format!(
                "Expected block hash, found illegal hex character at position: {}",
                offset + pos,
            ))
        } else {
            Ok(block_hash.into())
        }
    }
}
