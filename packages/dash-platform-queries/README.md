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
  [`ContextProvider`], and verifies every response proof with
  [`drive-proof-verifier`].
- Block explorers, Electrum-style servers, hardware-wallet tooling — anything
  that talks to DAPI its own way but must not trust responses.

If you want networking, retries, and a managed connection pool, use
`dash-sdk` — it consumes this crate internally.

## What's here

- [`documents::DocumentQuery`] — rich document query builder, wire
  encoding for both request versions, and decoding **from** the wire request
  (`DocumentQuery::try_from_request`) using the same proto conversions the
  server (`drive-abci`) uses, so client and server cannot drift.
- `documents::verify_documents_response` — request-driven proof verification
  for document queries, delegating to `drive-proof-verifier`'s `FromProof`.
- Aggregate proof helpers (count/sum/average/ranked) shared with `dash-sdk`.
- Pure DPNS builders — `build_dpns_preorder_and_domain_documents`, label
  normalization/validation — and pure DashPay contact-request document
  assembly (`dashpay::build_contact_request_document`); crypto material is
  supplied by the caller, keys never enter this crate.
- `transition::validation` and document-transition helpers
  (`ensure_entropy_matches_document_id`, `prepare_document_for_transition`).

## Feature flags

- `mocks` — serde support for the types used in dump/replay test vectors
  (forwarded by `dash-sdk`'s `mocks`).

The dependency tree is checked in CI to stay free of the transport stack
(`hyper`, `rustls`, `tower`); see the "Check transport-free feature cuts"
step in `.github/workflows/tests-rs-workspace.yml`.
