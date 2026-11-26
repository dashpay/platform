# Historical compatibility vector generation

From the repository root, run:

```sh
scripts/generate_historical_test_vectors.sh /data/tmp/platform-historical-vectors
```

The destination must not exist and its parent must exist. The script creates a
separate local checkout of Platform `6499c680c6bc311e8933f396423a79459e2a88bd`,
installs these modules as ignored integration tests, and runs them with
`cargo test --locked -- --ignored`. Encryption runs before signing, whose
DashPay batch embeds its ciphertexts. Set `HISTORICAL_VECTOR_CARGO` to a Cargo
wrapper executable when needed.

This revision uses rust-dashcore `40268cc0402a8933ec539f16b2d634c4e25876ad`,
secp256k1 0.30.0 and blsful `0c34a7a488a0bd1c9a9a2196e793b303ad35c900`.
It reproduces the BLS and quorum vectors originally captured at the older
revisions recorded in their fixture READMEs. Generation must use this pinned
historical graph even after the current branch upgrades its dependencies.

| Output directory under `vectors/` | Contents | Frozen expectations |
| --- | --- | --- |
| `bls` | BLS JSON corpus | `packages/rs-dpp/tests/fixtures/bls_compatibility/vectors.json` |
| `core` | Eight transaction encodings and txids | `packages/rs-dpp/tests/fixtures/historical_hashes/core_transactions.rs` |
| `signing` | Encryption bytes, signed transitions, hashes, lock IDs | DPP `historical_hashes/vectors.rs` and encryption `tests/historical_compatibility.rs` |
| `quorum` | V0/V1/V2 quorum, saved-state and checkpoint bytes | Drive ABCI `signature_verification_quorum_set/v0/storage_vectors.rs` |
| `populated` | Masternode/validator entries and populated states | The same Drive ABCI storage constants |

Compare generated output with the committed expectations; the script does not
rewrite them. All inputs are public synthetic test material. The node-ID vector
is independently derived from RFC 8032's public key and SHA-256 and needs no
historical generator.

The output contains `platform-revision.txt` and retains `source/` for inspection
or rerunning the ignored tests. The local checkout borrows Git objects from the
source repository. Normal builds and tests do not register these generators as
examples or binaries; their APIs belong to the historical revision.
