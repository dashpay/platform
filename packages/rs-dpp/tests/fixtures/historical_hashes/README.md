# Historical hash and signature vectors

The constants freeze outputs of Platform
`6499c680c6bc311e8933f396423a79459e2a88bd` (the base before #5307): rust-dashcore
`40268cc0402a8933ec539f16b2d634c4e25876ad`, secp256k1 0.30.0 and blsful
`0c34a7a488a0bd1c9a9a2196e793b303ad35c900`.

`core_transactions.rs` holds eight consensus-encoded transactions and their txids:
regular/Evo registration, registrar/service/revocation updates, and coinbase
payload versions 1, 2 and 3. They are synthetic encoding examples; their byte-pattern
BLS fields are not valid signatures. Copy `generate-core-transactions.rs` to the
old DPP examples directory as `generate_core_transactions.rs`, then run:

```sh
cargo run -p dpp --example generate_core_transactions --locked -- /path/to/output
```

`vectors.rs` holds actual signing inputs, hashes and signatures:

- a two-input address transfer, including ECDSA and Basic BLS signatures;
- InstantLock and ChainLock consensus bytes, request IDs and sign IDs;
- a synthetic DashPay contactRequest batch containing historical ciphertexts.

For reproduction, first run
`packages/rs-platform-encryption/tests/fixtures/generate-historical-encryption.rs`
as an example in the old encryption crate. This emits `encrypted-xpub.bin` and
`encrypted-label.bin`. Copy `generate.rs` here to the old DPP examples directory
as `generate_historical_hashes.rs`, then run it with the same output directory:

```sh
cargo run -p dpp --example generate_historical_hashes --locked -- /path/to/output
```

Compare the hex outputs with the constants; do not refresh expectations using
the current dependencies. Generators live below `tests/fixtures` so the current
build does not compile their historical APIs. All keys and scalars are public
test material. Ciphertext construction is covered separately by
`platform-encryption/tests/historical_compatibility.rs`; the DPP test covers its
inclusion in the signed batch, not contract validation or encryption policy.
