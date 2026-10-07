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
