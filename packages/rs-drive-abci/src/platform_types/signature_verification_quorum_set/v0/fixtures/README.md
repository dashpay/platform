# Historical V0 quorum storage

`quorum-storage-v0.hex` contains a synthetic, complete
`SignatureVerificationQuorumSetForSaving::V0` encoded by Platform commit
`7c73e983b4c3cd9b5e2ead34bbe0360646b7b4d9`, before the DPP BLS backend migration.
It uses `grovedb-bincode` with `standard().with_big_endian().with_no_limit()`
and the C++ `bls-signatures` public-key Serde implementation at revision
`0842b17583888e8f46c252a4ee84cdfd58e0546f`.

The hex file (including its final newline) has SHA-256
`8a2c8e59633ca1b126e3c0cc358a9347667d0f54068a874b1ae1435b85af830f`
and encodes 350 bytes.

The four public keys come from DPP's frozen BLS compatibility vectors. Current
quorums have repeated-byte hashes `11` and `22`, with indexes `None` and `Some(0)`;
previous quorums have hashes `33` and `44`, with indexes `Some(1)` and `None`.
The previous-quorum heights are 1000 (last active), 1008 (updated), and
`Some(900)` (previous change). Configuration is `Llmq400_60`, four active signers,
no rotation, and a 288-block window.

The test decodes the production storage enum and converts it to the runtime
quorum set, exercising the historical C++-to-DPP key conversion for both lists.
Expected keys and metadata are asserted independently of the current writer.

To reproduce, copy `generate-quorum-storage-v0.rs` from this directory to
`packages/rs-drive-abci/examples/generate_quorum_storage_v0.rs` in a separate
checkout of the commit above, then run from that checkout:

```sh
cargo run -p drive-abci --example generate_quorum_storage_v0 --all-features --locked -- /data/tmp/quorum-storage-v0.hex
```

Compare the generated file with the committed fixture. Do not regenerate the
expected bytes with the migrated backend. The generator explicitly selects V0;
the ordinary runtime-to-storage conversion writes V2.

## Historical V1/V2 quorum, saved-state and checkpoint records

`generate-quorum-storage.rs` was run against the same pre-migration Platform
commit above, using `blsful` at `0c34a7a488a0bd1c9a9a2196e793b303ad35c900`.
The configuration, keys, hashes, indexes and previous-quorum heights are the same
as the V0 fixture. V1 stores previous keys with the C++ library; V2 stores both
lists with blsful. Its public keys occupy exactly 48 compressed bytes, without
a length prefix.

The full state fixtures contain both chain-lock and instant-lock quorum sets,
protocol versions 13 (current) and 14 (next), and empty masternode/validator
collections. `platform-state-v1.hex` comes from `serialize_to_bytes`;
`checkpoint-platform-state.hex` comes from `serialize_standalone_to_bytes`, the
writer used for checkpoint `platform_state.bin`. These two files intentionally
have identical bytes. `platform-state-v2.hex` contains the per-entry storage
record; its external collections are empty. These are synthetic historical
records, not snapshots of a live database.

Copy the generator to `packages/rs-drive-abci/examples/generate_quorum_storage.rs`
in that historical checkout and run:

```sh
cargo run -p drive-abci --example generate_quorum_storage --all-features --locked -- /path/to/output-directory
```

Checksums below include the final newline in each hex file.

| File | Decoded bytes | SHA-256 |
| --- | ---: | --- |
| `quorum-storage-v1.hex` | 348 | `33cfbd097457cce4d8d5fadc214a12beee7a03b3b2a6deecabe77e74299bfee1` |
| `quorum-storage-v2.hex` | 346 | `bc2eefbb94cd7e8f2dae643eed1d76bff3953412955bb8a96960f755b34235de` |
| `platform-state-v1.hex` | 734 | `e2b83244feff85195d45fe4092ea85d298037f53cecdddd55c0eb7a5dda27ad7` |
| `checkpoint-platform-state.hex` | 734 | `e2b83244feff85195d45fe4092ea85d298037f53cecdddd55c0eb7a5dda27ad7` |
| `platform-state-v2.hex` | 732 | `d5eb9be1fac002863a22f1cfec3bb770ee9f67dde44ed3cba4734b18135f9616` |
