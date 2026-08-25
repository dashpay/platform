# dash-platform-queries

Transport-free query core of the Dash Platform SDK.

This crate carries the pieces of `dash-sdk` that build queries, encode them
onto the wire format, and decode/verify proved responses — with **no
transport implementation**: no `rs-dapi-client` and no tonic native
channel/TLS stack. Shared generated types and context-provider utilities
remain dependencies. `dash-sdk` depends on it and re-exports every moved
item at its historical path, so most imports keep compiling unchanged — but
the extraction is not fully source-compatible. Depending on what it uses,
downstream code may need to:

- import the `dash_sdk::platform::DocumentQuerySdk` extension trait to keep
  calling `DocumentQuery::new_with_data_contract_id`, which fetches the
  contract and therefore needs `&Sdk`;
- handle `dash_platform_queries::Error` — `DocumentQuery`'s fallible
  methods now return it instead of `dash_sdk::Error`. `?` call sites keep
  compiling via `From`; explicit return types and direct variant matches
  need a conversion;
- implement the `dash_sdk::platform::WireQuery` marker trait
  (`impl WireQuery for MyCustomRequest {}`) for custom `TransportRequest`
  types used with the blanket `Query` impl.

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

- `DocumentQuery` — rich document query builder, wire encoding for both
  request versions, and decoding **from** the wire request
  (`DocumentQuery::try_from_request`) via decoders that mirror the server's
  (`drive-abci`'s `v1/conversions.rs`) and are kept in lockstep with them.
- `verify_documents_response` — request-driven proof verification for document
  queries, delegating to `drive-proof-verifier`'s `FromProof`.
- Aggregate proof helpers (count/sum/average/ranked) shared with `dash-sdk`.
- Pure DPNS builders — `build_dpns_preorder_and_domain_documents`, label
  normalization/validation — and pure DashPay contact-request document
  assembly (`dashpay::build_contact_request_document`); crypto material is
  supplied by the caller, keys never enter this crate.
- `transition::validation` helpers.

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
