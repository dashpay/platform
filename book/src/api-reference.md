# API Reference

Auto-generated API documentation for the Dash Platform developer ecosystem.
Each section is built from source and updated by CI when relevant source files change on `v*-dev` branches.

---

## [Rust Crate Docs](api/rust/dash_sdk/index.html)

Full [rustdoc](https://doc.rust-lang.org/rustdoc/) for the Dash Platform workspace — every public type, trait, function, and module across all crates.

Key crates:

| Crate | Description |
|-------|-------------|
| `dash-sdk` | High-level client SDK with builder pattern and fetch traits |
| `dpp` | Dash Platform Protocol — data contracts, documents, identities, state transitions |
| `drive` | Decentralized storage engine built on GroveDB |
| `drive-abci` | ABCI application connecting Tenderdash to Drive |
| `dapi-grpc` | Rust types generated from the gRPC protocol definitions |
| `rs-dapi-client` | Low-level DAPI client with retries and load balancing |
| `platform-value` | Cross-language value representation |
| `platform-version` | Protocol versioning and feature version dispatch |

## [gRPC API](api/grpc/index.html)

Protocol Buffer service definitions for the three gRPC endpoints:

- **Platform** — identity, data contract, document, and token operations
- **Core** — block, transaction, and masternode queries
- **Drive** — internal node-to-node replication

Documents every RPC method, request/response message, and field type.

## [JavaScript / TypeScript API](api/js/index.html)

[TypeDoc](https://typedoc.org/) for the JavaScript and TypeScript client libraries:

- **dash** (js-dash-sdk) — main SDK for Node.js and browser applications
- **wasm-dpp** — WebAssembly bindings for Dash Platform Protocol
- **wasm-sdk** — Browser-facing WASM SDK with TypeScript type definitions
