# Signed contract cache responses

`signed-responses.json` contains nine complete DAPI protobuf responses generated
offline from genuine GroveDB state/proofs and deterministic Tenderdash BLS
signatures. Client tests wrap those responses in the existing raw transport
fixtures and run the actual SDK proof verifier. The server example owns proof
generation; `wasm-sdk` needs only Drive verification features, including in its
native tests.

The bundle records its format and protocol versions, generator path, deterministic
IDs/signing inputs, quorum public key and five pre/post-document state roots.
It covers owner and second-writer update/document/nonce proofs, a submitted-v3
update against committed v2, and a signed older-v1 contract fetch. The latter
is used by the cold shared-waiter and no-trusted-cache regressions. Negative
signature/proof tests mutate private decoded copies after validating the
canonical bundle's provenance.

Regenerate from the repository root, using the checked-in lockfile:

```sh
CARGO_BUILD_JOBS=2 cargo run -p drive-abci --example generate_wasm_contract_cache_vectors --locked -j 2 -- /tmp/contract-cache-first.json
CARGO_BUILD_JOBS=2 cargo run -p drive-abci --example generate_wasm_contract_cache_vectors --locked -j 2 -- /tmp/contract-cache-second.json
cmp /tmp/contract-cache-first.json /tmp/contract-cache-second.json
cmp /tmp/contract-cache-first.json packages/wasm-sdk/tests/fixtures/contract-cache/signed-responses.json
CARGO_BUILD_JOBS=2 cargo clippy -p drive-abci --example generate_wasm_contract_cache_vectors --locked --no-deps -j 2 -- -D warnings
CARGO_BUILD_JOBS=2 cargo test -p wasm-sdk --lib --locked -j 2
```

The named example command compiles the normal server library and its dependencies;
it does not compile server integration-test targets. Existing workspace Clippy
also compiles/lints the example, but CI does not regenerate the bundle.

Generation uses `PlatformVersion::latest()`. Client setup rejects a recorded
protocol that differs from latest, missing records, inconsistent metadata and
unsupported fixture inputs. After a protocol or fixture-input change, regenerate
both copies, review their identical diff, update the committed bundle and run
the client regressions. Signed response metadata remains intact so verification
uses the authenticated protocol version. These vectors establish client
authentication and schema/cache behavior; they do not execute server transitions
or measure fees.
