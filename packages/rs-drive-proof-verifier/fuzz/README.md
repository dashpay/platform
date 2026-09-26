# drive-proof-verifier fuzz targets

[cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) targets that feed arbitrary
bytes to `FromProof::maybe_from_proof` as the GroveDB proof of a DAPI response.

| target | entry points | queries |
|---|---|---|
| `identity` | `Identity`, `IdentityBalance`, `IdentityPublicKeys`, `IdentityContractNonceFetcher` | the fixture identity by id; its balance, all keys, DPNS contract nonce |
| `documents` | `Documents` via the SDK's `DocumentQuery` | DPNS `domain` by exact `normalizedLabel` (limit 1) and by label prefix (ascending, limit 25) |
| `contested` | `Contenders` | DPNS contested vote state of an active, a finished and an absent name contest |

The crate is its own workspace, so it never takes part in the repository build.

## What the fuzzer reaches

Each response wraps the fuzzer's bytes in the Tenderdash envelope recorded in the
proof-vector corpus (`../tests/vectors`): the same quorum, signature, block id and
metadata. A fixed context provider serves the corpus quorum public key for any
quorum and the DPNS contract.

The verifier checks the quorum signature only after it has verified the GroveDB
proof against the query and decoded the proven elements, so all of that runs on
attacker-controlled bytes: the GroveDB envelope version check, GroveDB proof
decoding and Merk verification, and Drive's interpretation of the proven elements
(balances, revisions, public keys, documents, contenders and vote tallies).

Every input is verified under protocol version 13 and under the latest one. The
version sets the oldest GroveDB proof envelope the verifier accepts: 13, where
SDK clients on mainnet and testnet start, still hands legacy V0 envelopes to
GroveDB's V0 decoder, while 14 rejects them before GroveDB sees them. V1
envelopes, the format of every corpus proof, reach GroveDB under both.

The signature check passes only when the proof rebuilds the root hash the fixture
quorum signed, and every corpus proof commits to that root, so mutations that keep
the root get a result back. For those, each target asserts that the result is the
one the recorded proof of that root yields for the same query. A different result
would mean that a proof accepted under a genuine quorum signature proves something
the signed state does not hold.

The corpus stores a placeholder payload where the DPNS documents are, so every
proof of the signed root fails document decoding. The `documents` target
therefore asserts that neither query is ever accepted.

## Running

Requires a nightly toolchain at least as new as the `channel` in
`rust-toolchain.toml` (CI pins the one in
`.github/workflows/tests-rs-nightly-long-running.yml`), and
`cargo install cargo-fuzz`. The crate has no lock of its own: copy the
workspace lock so it builds with the workspace's exact pins, seed each target
with the corpus proofs, then run it:

```bash
cd packages/rs-drive-proof-verifier/fuzz
cp ../../../Cargo.lock .
for target in identity documents contested; do
  mkdir -p "corpus/$target"
  for proof in ../tests/vectors/*/proof.hex; do
    xxd -r -p "$proof" "corpus/$target/$(basename "$(dirname "$proof")")"
  done
done
cargo +nightly fuzz run identity -- -max_total_time=60
```

A crash is written to `artifacts/<target>/`; replay it with
`cargo +nightly fuzz run <target> artifacts/<target>/<file>`.

The nightly workflow (`.github/workflows/tests-rs-nightly-long-running.yml`) runs
each target for 60 seconds; the PR workflow (`tests-rs-workspace.yml`) runs
`cargo fmt --check` and `cargo clippy` over the crate on the stable toolchain.
