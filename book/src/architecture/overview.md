# Monorepo Overview

Dash Platform ships as a single Git repository containing 44 Rust crates, a
handful of JavaScript/TypeScript packages, and supporting tooling. This chapter
maps the territory: what each crate owns, how they depend on one another, and
where the boundaries are drawn.

## The Workspace

The top-level `Cargo.toml` declares a workspace with `resolver = "2"`. It pins
a few critical external dependencies at the workspace level -- most notably
`dashcore` (the Rust Dash Core library) and the GroveDB family of crates:

```toml
# From Cargo.toml (workspace root)
[workspace.dependencies]
dashcore = { git = "https://github.com/dashpay/rust-dashcore", rev = "53d699c..." }
```

The workspace version (`3.0.1` at time of writing, Rust edition 2021, MSRV
1.92) is shared by all member crates through `version.workspace = true`.

## The Core Dependency Chain

Four crates form the spine of the platform. Understanding their dependency order
is the single most important thing for navigating the codebase:

```text
    dpp (rs-dpp)
      |
      v
    drive (rs-drive)
      |
      v
    drive-abci (rs-drive-abci)
      |
      v
    dash-sdk (rs-sdk)
```

Each layer adds a concern:

### dpp -- Dash Platform Protocol

**Crate name:** `dpp`
**Path:** `packages/rs-dpp`

DPP defines the *data model* of the platform. This is where you will find:

- `Identity`, `IdentityPublicKey`, and identity state transitions
- `DataContract` and the JSON Schema validation logic
- `Document` and document state transitions
- `StateTransition` -- the enum that unifies all transition types
- Prelude types used everywhere: `Identifier`, `BlockHeight`, `IdentityNonce`
- Token, voting, and withdrawal types

```rust
// From packages/rs-dpp/src/lib.rs
pub mod data_contract;
pub mod document;
pub mod identifier;
pub mod identity;
pub mod state_transition;
pub mod tokens;
pub mod voting;
pub mod fee;
pub mod validation;
```

DPP is deliberately *storage-agnostic*. It knows nothing about GroveDB, ABCI,
or gRPC. It depends on `platform-version`, `platform-value`, and
`platform-serialization` for versioned encoding, but it never opens a database.

A key design choice: DPP uses **feature flags extensively**. The `Cargo.toml`
lists over 80 features that control which serialization formats, validation
paths, and system contracts are compiled in. The `abci` feature, for instance,
pulls in the subset needed by Drive-ABCI without dragging in client-side
concerns:

```toml
# From packages/rs-dpp/Cargo.toml
[features]
abci = [
  "state-transitions",
  "state-transition-validation",
  "validation",
  "random-public-keys",
  "identity-serialization",
  "vote-serialization",
  "platform-value-cbor",
  "core-types",
  "core-types-serialization",
  "core-types-serde-conversion",
  "core_rpc_client",
]
```

**Rule:** If you are adding a new type that both the server and the SDK need,
put it in DPP. If it needs GroveDB, it belongs in Drive.

### drive -- Dash Drive

**Crate name:** `drive`
**Path:** `packages/rs-drive`

Drive is the *storage engine*. It wraps GroveDB (a Merkle-tree-based key-value
store) and provides domain-specific operations: inserting documents, managing
identity balances, tracking fee pools, building cryptographic proofs.

```rust
// From packages/rs-drive/src/lib.rs
#[cfg(any(feature = "server", feature = "verify"))]
pub mod drive;
#[cfg(any(feature = "server", feature = "verify"))]
pub mod query;
#[cfg(feature = "server")]
pub mod state_transition_action;
#[cfg(any(feature = "server", feature = "verify"))]
pub mod verify;
#[cfg(feature = "server")]
pub mod fees;
```

Notice the feature split: `server` includes everything needed to *write* to the
database, while `verify` includes only what is needed to *prove* and *verify*
queries. The SDK only enables `verify`, which means it can check GroveDB proofs
without linking in the full storage engine.

Drive depends on DPP for type definitions and on the GroveDB family of crates
for storage:

```toml
# From packages/rs-drive/Cargo.toml
dpp = { path = "../rs-dpp", features = ["state-transitions"], default-features = false }
grovedb = { git = "https://github.com/dashpay/grovedb", rev = "33dfd48...", optional = true }
grovedb-path = { git = "https://github.com/dashpay/grovedb", rev = "33dfd48..." }
grovedb-costs = { git = "https://github.com/dashpay/grovedb", rev = "33dfd48...", optional = true }
```

**Rule:** Never import `drive` with the `server` feature from a client-side
crate. Use `verify` only.

### drive-abci -- The ABCI Application

**Crate name:** `drive-abci`
**Path:** `packages/rs-drive-abci`

This is the *application server*. It implements the Tenderdash ABCI interface,
orchestrates block processing, validates state transitions, manages platform
state, and serves gRPC queries. It is the binary that masternodes run.

Drive-ABCI depends on both DPP and Drive, plus the Tenderdash ABCI library and
the dapi-grpc protobuf definitions:

```toml
# From packages/rs-drive-abci/Cargo.toml
drive = { path = "../rs-drive", default-features = false, features = ["server"] }
dpp = { path = "../rs-dpp", default-features = false, features = ["abci"] }
tenderdash-abci = { git = "https://github.com/dashpay/rs-tenderdash-abci", tag = "v1.5.0" }
dapi-grpc = { path = "../dapi-grpc", default-features = false, features = ["server", "platform"] }
```

The crate is organized into three major subsystems:

1. **`abci/`** -- Tenderdash handler functions (`prepare_proposal`,
   `process_proposal`, `finalize_block`, `check_tx`, etc.)
2. **`execution/`** -- Block processing engine, state transition validation,
   platform events (epoch changes, withdrawals, voting)
3. **`query/`** -- gRPC query service implementing the Platform API

**Rule:** Business logic goes in `execution/`. The `abci/` handlers should be
thin wrappers that delegate to the execution engine.

### dash-sdk -- The Client SDK

**Crate name:** `dash-sdk`
**Path:** `packages/rs-sdk`

The SDK is what application developers use. It provides high-level methods for
fetching documents, creating identities, broadcasting state transitions, and
verifying proofs. It depends on DPP for types, Drive (with `verify` only) for
proof verification, and `rs-dapi-client` for network communication:

```toml
# From packages/rs-sdk/Cargo.toml
dpp = { path = "../rs-dpp", default-features = false, features = ["dash-sdk-features"] }
drive = { path = "../rs-drive", default-features = false, features = ["verify"] }
drive-proof-verifier = { path = "../rs-drive-proof-verifier", default-features = false }
rs-dapi-client = { path = "../rs-dapi-client", default-features = false }
```

## Supporting Crates

Several smaller crates provide cross-cutting infrastructure:

### platform-version

**Path:** `packages/rs-platform-version`

The versioning backbone. Defines `PlatformVersion`, `ProtocolVersion`, and the
version tables for every consensus-critical method across DPP, Drive, and
Drive-ABCI. Currently tracks 12 protocol versions (v1 through v12).

```rust
// From packages/rs-platform-version/src/version/mod.rs
pub type ProtocolVersion = u32;
pub const LATEST_VERSION: ProtocolVersion = PROTOCOL_VERSION_12;
pub const INITIAL_PROTOCOL_VERSION: ProtocolVersion = 1;
```

Every crate in the dependency chain depends on `platform-version`. It is the
root of the version tree.

### platform-serialization

**Path:** `packages/rs-platform-serialization`

A thin wrapper around `bincode` that adds platform-version-aware serialization.
Paired with `platform-serialization-derive` for derive macros that generate
versioned `Encode`/`Decode` implementations.

### platform-value

**Path:** `packages/rs-platform-value`

A dynamically-typed value type (think `serde_json::Value` but with binary
support). Used as the interchange format when converting between JSON, CBOR, and
Rust types, especially in data contract and document processing.

### drive-proof-verifier

**Path:** `packages/rs-drive-proof-verifier`

Client-side proof verification. Takes a GroveDB proof returned by a platform
query and verifies it against a known root hash. Used by the SDK and the WASM
bindings.

### dapi-grpc

**Path:** `packages/dapi-grpc`

Protobuf definitions and generated Rust code for the Platform gRPC API. Both
server-side (Drive-ABCI) and client-side (SDK, DAPI client) depend on this
crate, using the `server` and `client` features respectively.

## What Is GroveDB?

GroveDB is an external dependency -- a Merkle-tree-based authenticated data
structure built on RocksDB. It is not part of this repository but is central to
understanding Drive.

Key properties:

- **Authenticated:** Every read can produce a cryptographic proof that the data
  (or its absence) is consistent with the root hash stored in the block header.
- **Hierarchical:** Data is organized into nested trees (subtrees), addressed by
  paths. A document lives at a path like
  `[contract_id, document_type, document_id]`.
- **Sum trees:** Some subtrees track the sum of their leaf values, used for
  balance accounting and fee verification.
- **Transactional:** All writes happen inside a transaction that can be committed
  or rolled back atomically.
- **Cost-tracking:** Every operation returns a `CostResult` that records storage
  and processing costs.

GroveDB is pinned to a specific Git revision in the workspace `Cargo.toml` and
referenced by five sub-crates: `grovedb`, `grovedb-path`, `grovedb-costs`,
`grovedb-storage`, and `grovedb-version`.

## The Full Crate Map

Here is a simplified view of every Rust workspace member, grouped by role:

| Role | Crates |
|------|--------|
| **Protocol types** | `dpp`, `platform-value`, `platform-serialization`, `platform-serialization-derive`, `platform-versioning`, `platform-value-convertible` |
| **Storage** | `drive` |
| **Application server** | `drive-abci` |
| **Client SDK** | `dash-sdk`, `rs-dapi-client`, `dash-context-provider`, `rs-sdk-trusted-context-provider` |
| **Proof verification** | `drive-proof-verifier` |
| **gRPC definitions** | `dapi-grpc` |
| **WASM bindings** | `wasm-dpp`, `wasm-dpp2`, `wasm-sdk`, `wasm-drive-verify` |
| **iOS/FFI** | `rs-sdk-ffi` |
| **System contracts** | `dpns-contract`, `dashpay-contract`, `withdrawals-contract`, `masternode-reward-shares-contract`, `wallet-utils-contract`, `token-history-contract`, `keyword-search-contract`, `data-contracts` |
| **Tooling** | `dashmate` (JS), `strategy-tests`, `simple-signer`, `check-features`, `json-schema-compatibility-validator` |
| **Other** | `dash-platform-macros`, `rs-dash-event-bus`, `rs-platform-wallet`, `dash-platform-balance-checker`, `rs-dapi` |

## Rules

**Do:**
- Follow the dependency direction. DPP never imports Drive. Drive never imports
  Drive-ABCI.
- Use feature flags to keep compilation lean. The SDK should never compile
  server-side code.
- Put new domain types in DPP. Put new storage operations in Drive. Put new
  validation logic in Drive-ABCI.

**Don't:**
- Add GroveDB as a dependency to DPP. If you need tree structure knowledge in a
  type definition, use a path abstraction.
- Enable the `server` feature of Drive in client-facing crates. This pulls in
  RocksDB and doubles compile times.
- Create new top-level crates without updating the workspace `Cargo.toml` and
  ensuring the dependency direction is maintained.
