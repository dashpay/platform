# dash-platform-queries

Transport-free query core of the Dash Platform SDK.

This crate carries the pieces of `dash-sdk` that build queries, encode them
onto the wire format, and decode/verify proved responses — with **no
transport implementation**: no `rs-dapi-client` and no tonic native
channel/TLS stack. Shared generated types and context-provider utilities
remain dependencies. `dash-sdk` depends on it and re-exports everything at
the historical paths, so SDK users need no changes.

## Who this is for

Embedders that bring their own transport and trust context and only need the
verification/query layer:

- **Dash Core's platform GUI** — fetches over its own gRPC-Web transport,
  serves quorum keys from its locally synced LLMQ state via a
  [`ContextProvider`](../rs-context-provider), and verifies every response
  proof with [`drive-proof-verifier`](../rs-drive-proof-verifier).
- Block explorers, Electrum-style servers, hardware-wallet tooling — anything
  that talks to DAPI its own way but must not trust responses.

If you want networking, retries, and a managed connection pool, use
`dash-sdk` — it consumes this crate internally.

## What's here

- [`documents::DocumentQuery`] — rich document query builder with wire
  encoding for both request versions.
- Aggregate proof helpers (count/sum/average/ranked) shared with `dash-sdk`.
- DPNS username helpers — label normalization/validation and the
  convertibility/contested checks shared with `dash-sdk`.
- `transition::validation` — structural validation for state transitions
  ahead of signing.

Wire-request decoding (`DocumentQuery::try_from_request`), request-driven
proof verification, and pure DPNS/DashPay document builders arrive in the
next slice of this series.

## Feature flags

- `mocks` — serde support for the types used in dump/replay test vectors
  (forwarded by `dash-sdk`'s `mocks`).

The dependency tree is checked in CI to stay free of the transport stack
(`hyper`, `rustls`, `tower`); see the "Check transport-free feature cuts"
step in `.github/workflows/tests-rs-workspace.yml`.

"Transport-free" means no networking stack, not an async-runtime-free graph:
`tokio` is still reachable on native targets through
`dash-context-provider` → `dash-async`, exactly as it already was for
`drive-proof-verifier` before this crate existed. `tonic` is present too, but
only for `dapi-grpc`'s generated message/client types — its transport feature
stays off, which is what the `hyper`/`rustls`/`tower` assertions prove. On
`wasm32-unknown-unknown` none of that is pulled in; the wasm assertions in the
same CI step also ban `mio`.
