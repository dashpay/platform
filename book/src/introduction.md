# Introduction

Welcome to *The Dash Platform Book*. This is a guide to the design philosophy,
architectural patterns, and engineering conventions that shape the Dash Platform
Rust codebase. It is not a user manual or an API reference -- it is the book you
read *before* you open your editor, so that the thousands of files in the
monorepo start to make sense.

## Who This Book Is For

- **Contributors** who want to add a new state transition, query endpoint, or
  Drive operation and need to know where things go and why.
- **Auditors** reviewing the codebase for correctness, who need a map of the
  security-critical paths (validation pipelines, fee calculations, proof
  verification).
- **Architects** evaluating the system design, particularly the versioning
  strategy, the ABCI integration, and the GroveDB storage model.

If you have written Rust before and understand traits, enums, and feature flags,
you are ready.

## The Design Philosophy

Four principles recur throughout the codebase. Understanding them up front will
save you hours of "why is it done this way?" confusion.

### 1. Version Everything

Every method that touches consensus has a version number. The version is not
embedded in the code path -- it lives in a central `PlatformVersion` struct that
is threaded through the entire call stack:

```rust
// From packages/rs-platform-version/src/version/protocol_version.rs
#[derive(Clone, Debug)]
pub struct PlatformVersion {
    pub protocol_version: ProtocolVersion,
    pub dpp: DPPVersion,
    pub drive: DriveVersion,
    pub drive_abci: DriveAbciVersion,
    pub consensus: ConsensusVersions,
    pub fee_version: FeeVersion,
    pub system_data_contracts: SystemDataContractVersions,
    pub system_limits: SystemLimits,
}
```

When you see a method that dispatches on a version number, this is the
canonical pattern:

```rust
// From packages/rs-drive-abci/src/execution/engine/finalize_block_proposal/mod.rs
pub(crate) fn finalize_block_proposal(
    &self,
    request_finalize_block: FinalizeBlockCleanedRequest,
    block_execution_context: BlockExecutionContext,
    transaction: &Transaction,
    platform_version: &PlatformVersion,
) -> Result<block_execution_outcome::v0::BlockFinalizationOutcome, Error> {
    match platform_version
        .drive_abci
        .methods
        .engine
        .finalize_block_proposal
    {
        0 => self.finalize_block_proposal_v0(
            request_finalize_block,
            block_execution_context,
            transaction,
            platform_version,
        ),
        version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
            method: "finalize_block_proposal".to_string(),
            known_versions: vec![0],
            received: version,
        })),
    }
}
```

This pattern allows the network to upgrade without hard forks. Every node on the
network agrees on which version to run for each method at each block height, and
the version table is the single source of truth. The [Versioning](versioning/platform-version.md)
section covers this in depth.

### 2. Explicit Costs

Platform is a metered system. Every operation -- inserting a document, updating a
contract, creating an identity -- has a fee expressed in credits. Costs are not
estimated after the fact; they are tracked as operations execute, through
GroveDB's cost accounting layer. The `FeeResult` type flows alongside the data,
and sum trees in GroveDB let the platform verify fee totals against the
Merkle-proven state.

The fee version itself is part of `PlatformVersion`, which means the network can
adjust fee schedules across protocol upgrades without any node disagreeing on
how much an operation costs at a given block height.

### 3. Clear Transformation Stages

A state transition does not go from "bytes on the wire" to "committed to disk"
in one step. It passes through well-defined stages:

1. **Decode** -- raw bytes become a `StateTransition` enum.
2. **Structure validation** -- syntactic checks (field lengths, required fields).
3. **State validation** -- checks against current platform state (does this
   identity exist? does the nonce match?).
4. **Transform into action** -- the validated transition becomes a
   `StateTransitionAction`, a representation of *what to do* rather than *what
   was requested*.
5. **Convert to operations** -- the action becomes a list of `DriveOperation`
   values (GroveDB inserts, deletes, replacements).
6. **Apply** -- the operations execute inside a GroveDB transaction.

Each stage is a separate module with its own versioned dispatch. This separation
means you can audit validation independently from execution, and you can test
each stage in isolation.

### 4. Trait-Based Polymorphism

The codebase avoids dynamic dispatch where performance matters and embraces it
where extensibility matters. ABCI handlers are defined against traits like
`PlatformApplication`, `TransactionalApplication`, and
`BlockExecutionApplication`:

```rust
// From packages/rs-drive-abci/src/abci/app/mod.rs
pub trait PlatformApplication<C = DefaultCoreRPC> {
    fn platform(&self) -> &Platform<C>;
}

pub trait TransactionalApplication<'a> {
    fn start_transaction(&self);
    fn transaction(&self) -> &RwLock<Option<Transaction<'a>>>;
    fn commit_transaction(&self, platform_version: &PlatformVersion)
        -> Result<(), Error>;
}

pub trait BlockExecutionApplication {
    fn block_execution_context(&self) -> &RwLock<Option<BlockExecutionContext>>;
}
```

This makes it possible to swap in mock implementations for testing while keeping
the production path zero-cost.

## How This Book Is Organized

The book follows the data as it flows through the system, from the outside in:

| Section | What You Will Learn |
|---------|-------------------|
| **Architecture** | The monorepo layout, crate responsibilities, and the request pipeline from client to GroveDB. |
| **Versioning** | How `PlatformVersion` controls every consensus-critical code path, and how upgrades propagate. |
| **State Transitions** | The lifecycle of a state transition: validation, transformation, operation generation, and application. |
| **Error Handling** | The split between consensus errors (returned to users) and execution errors (node-level panics). |
| **Serialization** | The `platform-serialization` crate and its derive macros for versioned binary encoding. |
| **Data Model** | Data contracts, documents, identities, and tokens as Rust types. |
| **Drive** | GroveDB operations, batch processing, cost tracking, and finalize tasks. |
| **Testing** | Unit test patterns, strategy tests for randomized multi-block scenarios, and test configuration. |
| **SDK** | The client-side `dash-sdk` crate: builder patterns, fetch traits, and proof verification. |
| **WASM** | Binding patterns for the browser-facing `wasm-dpp` and `wasm-sdk` crates. |

Each chapter follows the same arc: **why** the pattern exists (the problem it
solves), **what** the pattern is (the types and modules involved), **how** it
works (real code from the repository), and **rules** (the do's and don'ts that
keep the codebase consistent).

## Coding Conventions

A few conventions appear consistently across the codebase and are worth calling
out early:

- **`#![forbid(unsafe_code)]`** is set in both `dpp` and `drive`. The platform
  avoids unsafe Rust entirely in its core logic. Unsafe operations are confined
  to external dependencies (RocksDB, cryptographic libraries).
- **`#![deny(missing_docs)]`** is enabled in `drive`, enforcing doc comments on
  every public item. DPP has this commented out but is moving toward it.
- **Feature-gated compilation** is pervasive. A typical crate has 20-80 Cargo
  features controlling which modules, serialization formats, and integrations
  are compiled. This keeps binary sizes small and compile times manageable for
  downstream consumers that only need a subset of functionality.
- **Versioned module layout:** When a function has multiple versions, they live
  in sibling directories named `v0/`, `v1/`, etc., with a parent `mod.rs` that
  dispatches based on `PlatformVersion`. This is the dominant structural pattern
  in Drive-ABCI.

## A Note on Reading the Source

The codebase lives in a monorepo at `packages/`. The Rust crates are prefixed
with `rs-` on disk but have shorter names in `Cargo.toml`:

| Disk path | Crate name |
|-----------|-----------|
| `packages/rs-dpp` | `dpp` |
| `packages/rs-drive` | `drive` |
| `packages/rs-drive-abci` | `drive-abci` |
| `packages/rs-sdk` | `dash-sdk` |
| `packages/rs-platform-version` | `platform-version` |
| `packages/rs-platform-value` | `platform-value` |
| `packages/rs-platform-serialization` | `platform-serialization` |
| `packages/rs-drive-proof-verifier` | `drive-proof-verifier` |

The workspace currently targets Rust 1.92 and protocol version 12 (as of
v3.0.1). The workspace `Cargo.toml` lists 44 member crates, but the core
platform logic lives in the first eight listed above.

Let's begin with the architecture.
