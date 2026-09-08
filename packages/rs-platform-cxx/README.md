# dash-platform-cxx

Dash Platform for C++ applications, as a thin [`cxx`](https://cxx.rs) bridge
over `dash-sdk`. The embedder supplies what only it knows: the evonode
endpoints from its masternode list, the Platform quorum keys from its LLMQ
store, its best ChainLock height, and signatures from its wallet. The SDK
supplies everything else: query construction, transport (TLS to the
evonodes), retries with address banning, proof verification, protocol-version
tracking and the DPNS / DashPay document builders. Dash Core's platform GUI is
the first consumer.

## What it is

- `PlatformClient` (`client.rs`): one `Sdk` instance plus the tokio runtime
  that drives it and the embedder's freshness state. Every bridge call blocks
  the calling thread until the SDK operation completes or `shutdown`
  interrupts it; the embedder keeps its own worker thread and callback
  marshalling.
- Proved queries (`queries.rs`): identities (by id, by unique public key
  hash), identity and identity-contract nonces, DPNS resolve / search /
  names-of-identity, DashPay profile and contact requests, contested-name vote
  state, and state-transition broadcast. Each goes through the SDK's
  `Fetch` / `FetchMany`, so the SDK retains the rich query it built and
  verifies the response against it: no wire request is ever reconstructed
  from bytes.
- Trust context (`provider.rs`): a `ContextProvider` the SDK reads quorum keys
  and the pinned system contracts from. Nothing fetches from a trusted HTTP
  service; a proof signed by any quorum the embedder did not push, or by a
  quorum type other than the network's Platform type, fails verification.
- Signing (`st.rs`): dpp's own `Signer` trait implemented over a C++ digest
  callback (`WalletSigner`, `include/dash/platform/signer.h`), so private keys
  never cross the FFI. Document assembly is `dash-platform-queries`' pure
  DPNS / DashPay builders, the same functions `dash-sdk` uses.

`cxx` rather than the workspace's usual cbindgen C ABI because the surface is
dominated by nested byte vectors and fallible calls: cxx gives typed
`rust::Vec` fields and `Result → rust::Error` without hand-written
length/ownership plumbing on either side.

## Trust boundary

Every response byte comes from an untrusted node, and the SDK's GroveDB
replay necessarily runs before the quorum signature check (the root hash only
exists after replay). On top of the SDK's own checks (proof replay, quorum
signature, signed-time window, monotonic Platform height, upward-only
protocol-version ratchet), the client refuses a verified response whose
Tenderdash chain id is not the network's, or whose signed core-chain-locked
height trails the embedder's own ChainLock by more than
`MAX_CORE_CHAINLOCK_LAG` core blocks. Responses are size-capped by the SDK's
decoder limit; every fallible bridge entry point runs under `catch_unwind`,
so a panic anywhere in the SDK is reported as a `rust::Error`, never a
process abort (the infallible ones swallow a panic themselves).
The crate must therefore be built with `panic = "unwind"` (the default).

TLS is verified against the system trust store plus the bundled Mozilla
roots (rustls). Integrity does not rest on it: every query result is bound to
a locally known Platform quorum key before the embedder sees it.

## Building and installing

The crate is an ordinary workspace member. Build systems vendor from the
workspace root and build only this package:

```sh
cargo vendor --locked vendored            # offline crate sources
cargo build -p dash-platform-cxx --locked --offline --release
```

`build.rs` stages the headers an embedder includes under
`target/<profile>/include/`:

```
include/dash/platform/ffi.h      # generated bridge (namespace platform_ffi)
include/dash/platform/signer.h   # WalletSigner callback type
include/rust/cxx.h               # cxx runtime header ffi.h includes
```

Install that `include/` tree and `target/<profile>/libdash_platform_cxx.a`.
Link with `-lpthread -lm`, plus `-ldl` on Linux and `-framework Security
-framework CoreFoundation` on macOS (rustls reads the system trust store
through Security.framework). `tests/cxx_smoke.cc` is a link-and-run check of
exactly that interface; CI runs it through `scripts/cxx-smoke.sh`.

## Runtime lifecycle

1. `new_platform_client()`.
2. `set_context(network, platform_quorum_type, tenderdash_chain_id,
   protocol_version, platform_activation_height)` once.
3. On every masternode-list / quorum update: `set_endpoints(...)` (rebuilds
   the SDK only when the set changed; the verified-height watermark survives)
   and `update_quorum_keys(...)` (replaces the set).
4. On every ChainLock: `set_core_chain_locked_height(...)`.
5. Queries and broadcasts from any thread.
6. `shutdown()` cancels in-flight requests and releases the runtime; blocked
   callers return with an interruption error.

## Tests

`tests/queries.rs` replays drive-proof-verifier's proof-vector corpus through
`dash-sdk`'s mock transport: only the socket is mocked, the SDK's `FromProof`
path runs the GroveDB replay and the BLS quorum check against the key the
test pushed through the client. `tests/signing.rs` pins every builder byte
for byte against rs-dpp-generated vectors; `tests/decoders.rs` covers the
stored-document decoders. Run with `cargo test -p dash-platform-cxx`.
