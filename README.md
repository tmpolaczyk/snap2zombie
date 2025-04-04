# snap2zombie

Import a [try-runtime](https://github.com/paritytech/try-runtime-cli) snapshot into [zombienet](https://github.com/paritytech/zombienet)

# Install

```
cargo install --path . --locked
```

# Usage

```
snap2zombie create-snapshot --uri wss://dancebox.tanssi-api.network:443 dancebox-2025-04-01.snap
snap2zombie to-hex-snap --snapshot-path dancebox-2025-04-01.snap --output-path dancebox-2025-04-01.hexsnap.txt --pallet PooledStaking
snap2zombie merge-into-raw --chain-spec-path dancebox-raw-spec.json --hex-snapshot-path dancebox-2025-04-01.hexsnap.txt --output-path dancebox-raw-spec-snap.json --pallet PooledStaking
```

And now just use `dancebox-raw-spec-snap.json` in zombienet

Creating a snapshot for dancebox takes 7 minutes as of 2025-04-04

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

