# snap2zombie

Import a [try-runtime](https://github.com/paritytech/try-runtime-cli) snapshot into [zombienet](https://github.com/paritytech/zombienet)

# Install

```
cargo install --git https://github.com/tmpolaczyk/snap2zombie --locked
```

# Usage

```
snap2zombie create-snapshot --uri wss://dancebox.tanssi-api.network:443 dancebox-2025-04-01.snap
snap2zombie to-hex-snap --snapshot-path dancebox-2025-04-01.snap --output-path dancebox-2025-04-01.hexsnap.txt --pallet PooledStaking
snap2zombie merge-into-raw --chain-spec-path dancebox-raw-spec.json --hex-snapshot-path dancebox-2025-04-01.hexsnap.txt --output-path dancebox-raw-spec-snap.json --pallet PooledStaking
```

And now just use `dancebox-raw-spec-snap.json` in zombienet

Creating a snapshot for dancebox takes 7 minutes as of 2025-04-04

# Subcommands

## create-snapshot

This is just a re-export of the `try-runtime create-snapshot` command, included for convenience and to avoid issues with different snapshot versions.
Since the latest commit in `try-runtime` repo is from 6 months ago, you probably have the latest version already installed and you can use `try-runtime` for the snapshot.

## to-hex-snap

Extracts the raw key-values from the snapshot file, and saves it using a "hex snapshot" format.

[Snapshot](https://github.com/paritytech/polkadot-sdk/blob/f5de39196e8c30de4bc47a2d46b1a0fe1e9aaee0/substrate/utils/frame/remote-externalities/src/lib.rs#L66) struct

```rust
const SNAPSHOT_VERSION: SnapshotVersion = Compact(4);

/// The snapshot that we store on disk.
#[derive(Decode, Encode)]
struct Snapshot<B: BlockT> {
	snapshot_version: SnapshotVersion,
	state_version: StateVersion,
	// <Vec<Key, (Value, MemoryDbRefCount)>>
	raw_storage: Vec<(Vec<u8>, (Vec<u8>, i32))>,
	// The storage root of the state. This may vary from the storage root in the header, if not the
	// entire state was fetched.
	storage_root: B::Hash,
	header: B::Header,
}
```

Here you can see the `raw_storage` field which looks like a `Vec<(Key, Value)>`, but it's actually not. Trying to use that directly doesn't work.
The keys are almost the same as the actual keys except for the last portion, but the values are very different. I guess its some trie-db format.
If we had a way to decode this without using `ext` code, it would simplify this tool a lot.

The resulting hex snapshot file looks like this:

```
"0x012345": "0xaabbccdd",
"0x012346": "0xbbdd",
```

The idea is to be able to copy-paste it easily into the raw chain spec file.

## merge-into-raw

This command does a smart copy-paste from the hex snapshot into the raw chain spec file.
It is smart because before inserting the new values it first removes all the storage from the selected pallets.

## pad-with-spaces

This is a hack to artificially increase chain spec file size, because if the output file size is less than 2GB, zombienet will attempt to modify it and that may fail.
I believe the limit of RAM usage for a node.js program is 1.5 GB, and because of the way zombienet reads the chain spec file, it probably will crash if the chain spec
is more than 0.8 GB, because it needs to store in memory both the raw string as well as the deserialized object, and then it serializes it again into a string.

[zombienet code](https://github.com/paritytech/zombienet/blob/2564de11ad1513c1a523389ddb665b5a9e93b908/javascript/packages/orchestrator/src/paras.ts#L205)

# Sample run

```
$ snap2zombie create-snapshot --uri wss://dancebox.tanssi-api.network:443 dancebox-2025-04-01.snap
[2025-04-04T14:43:00Z INFO  remote-ext] replacing wss:// in uri with https://: "https://dancebox.tanssi-api.network:443" (ws is currently unstable for fetching remote storage, for more see https://github.com/paritytech/jsonrpsee/issues/1086)
[2025-04-04T14:43:00Z INFO  remote-ext] since no at is provided, setting it to latest finalized head, 0x595c79e0c39bcccc13f11115406a709b786cccd6cdc61cea383c6cda0b806963
[2025-04-04T14:43:00Z INFO  remote-ext] since no prefix is filtered, the data for all pallets will be downloaded
[2025-04-04T14:43:00Z INFO  remote-ext] scraping key-pairs from remote at block height 0x595c79e0c39bcccc13f11115406a709b786cccd6cdc61cea383c6cda0b806963
✅ Found 1047168 keys (269.80s)
[00:01:39] Downloading key values 10,872.6853/s [========================>----] 879716/1047168 (15s)[2025-04-04T14:49:10Z WARN  frame_remote_externalities] Batch request failed (2/12 retries). Error: Parse error: invalid type: map, expected a sequence at line 1 column 0
[00:01:47] Downloading key values 9,882.5089/s [============================>--] 953305/1047168 (9s)[2025-04-04T14:49:19Z WARN  frame_remote_externalities] Batch request failed (2/12 retries). Error: Parse error: invalid type: map, expected a sequence at line 1 column 0
[00:02:11] ✅ Downloaded key values 7,959.3761/s [============================] 1047168/1047168 (0s)
✅ Inserted keys into DB (2.97s)
[2025-04-04T14:49:45Z INFO  remote-ext] adding data for hashed prefix: , took 405.01s
[2025-04-04T14:49:45Z INFO  remote-ext] adding data for hashed key: 3a636f6465
[2025-04-04T14:49:46Z INFO  remote-ext] adding data for hashed key: 26aa394eea5630e07c48ae0c9558cef7f9cce9c888469bb1a0dceaa129672ef8
[2025-04-04T14:49:46Z INFO  remote-ext] adding data for hashed key: 26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac
[2025-04-04T14:49:46Z INFO  remote-ext] 👩‍👦 no child roots found to scrape
[2025-04-04T14:49:47Z INFO  remote-ext] writing snapshot of 314169966 bytes to "dancebox-2025-04-01.snap"
[2025-04-04T14:49:48Z INFO  remote-ext] initialized state externalities with storage root 0x7a8a35c1b8b9a7bfdce7299db4f558bfc1874aeb69a605c16b2908a35fbf8e78 and state_version V1
```

```
$ snap2zombie to-hex-snap --snapshot-path dancebox-2025-04-01.snap --output-path dancebox-2025-04-01.hexsnap.txt --pallet PooledStaking
[2025-04-04T14:52:08Z INFO  snap2zombie::to_hex_snap] Will only keep prefixes: [
        "359e684ff9b0738b7dc97123fd114c24",
    ]
[2025-04-04T14:52:09Z INFO  remote-ext] Loading snapshot from "dancebox-2025-04-01.snap"
✅ Loaded snapshot (0.78s)
[2025-04-04T14:52:10Z INFO  remote-ext] initialized state externalities with storage root 0x7a8a35c1b8b9a7bfdce7299db4f558bfc1874aeb69a605c16b2908a35fbf8e78 and state_version V1
```

```
$ snap2zombie merge-into-raw --chain-spec-path dancebox-raw-spec.json --hex-snapshot-path dancebox-2025-04-01.hexsnap.txt --output-path dancebox-raw-spec-snap.json --pallet PooledStaking
[2025-04-04T14:54:35Z INFO  snap2zombie::merge_into_raw] Will remove these key prefixes from original chain spec, and copy them from the hex snapshot: [
        "359e684ff9b0738b7dc97123fd114c24",
    ]
[2025-04-04T14:54:35Z INFO  snap2zombie::merge_into_raw] Removed 1 keys from existing chain spec
[2025-04-04T14:54:35Z INFO  snap2zombie::merge_into_raw] Inserted 981498 new keys from snapshot
[2025-04-04T14:54:35Z INFO  snap2zombie::merge_into_raw] Final file size: 298824554 bytes
[2025-04-04T14:54:35Z WARN  snap2zombie::merge_into_raw] Output file size is less than 2GB, zombienet will attempt to modify it and that may fail
[2025-04-04T14:54:35Z WARN  snap2zombie::merge_into_raw] Use pad-with-spaces subcommand to workaround that. Note that this may not be needed, so try without it first.
```

```
$ ls -lh dancebox-*
-rw-rw-r-- 1 tomasz tomasz 283M abr  4 16:52 dancebox-2025-04-01.hexsnap.txt
-rw-rw-r-- 1 tomasz tomasz 300M abr  4 16:49 dancebox-2025-04-01.snap
-rw-rw-r-- 1 tomasz tomasz 2,6M abr  4 16:53 dancebox-raw-spec.json
-rw------- 1 tomasz tomasz 285M abr  4 16:54 dancebox-raw-spec-snap.json
```

