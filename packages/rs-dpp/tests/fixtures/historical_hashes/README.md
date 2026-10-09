# Historical hash and signature vectors

The constants freeze outputs of Platform
`6499c680c6bc311e8933f396423a79459e2a88bd` (the base before #5307): rust-dashcore
`40268cc0402a8933ec539f16b2d634c4e25876ad`, secp256k1 0.30.0 and blsful
`0c34a7a488a0bd1c9a9a2196e793b303ad35c900`.

`core_transactions.rs` holds eight consensus-encoded transactions and their txids:
regular/Evo registration, registrar/service/revocation updates, and coinbase
payload versions 1, 2 and 3. They are synthetic encoding examples; their byte-pattern
BLS fields are not valid signatures.

`vectors.rs` holds actual signing inputs, hashes and signatures:

- a two-input address transfer, including ECDSA and Basic BLS signatures;
- InstantLock and ChainLock consensus bytes, request IDs and sign IDs;
- a synthetic DashPay contactRequest batch containing historical ciphertexts.

Reproduce these bytes with the [historical generation script](../../../../../scripts/historical-test-vectors/README.md).
It generates encryption bytes before the signed DashPay batch and writes all
results to a separate output directory. Do not refresh expectations using
upgraded dependencies. All keys and scalars are public test material.
Ciphertext construction is covered separately by
`platform-encryption/tests/historical_compatibility.rs`; the DPP test covers its
inclusion in the signed batch, not contract validation or encryption policy.
