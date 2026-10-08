# Drive Errors

The previous two chapters covered consensus errors -- the carefully serialized, code-stable errors that get sent across the network. Drive errors are a different beast entirely. They are **internal** errors that arise from the storage layer, the database, and the logic that sits between state transitions and GroveDB. Their Rust enum is not serialized as a public consensus error and does not need stable numeric codes. Public queries map selected errors to transport statuses, as described below.

But they do need to be well-organized, because Drive is where most of the platform's complexity lives. When something goes wrong in Drive, you need to know immediately whether it is a corrupted database, a protocol-level validation failure, a fee calculation error, or a bug in your own code.

## Public query errors

The gRPC `QueryService` classifies a typed `QuerySyntaxError` as
`InvalidArgument`, including when it is wrapped in Drive or ABCI errors.
Database corruption, execution failures, and arbitrary GroveDB or proof errors
retain their server-error classification. This distinction lets the existing
client reject a caller's invalid request without retrying it or banning the
responding node. Internal node failures and invalid cryptographic proofs still
trigger the existing retry and ban policy.

Some request shapes must be checked before constructing a storage proof:

| Request | Public contract |
|---|---|
| Empty contract, identity-balance, contract-key, token-balance/info/status, evonode-ID, or address-info selections | Unproved empty results are retained; proved empty selections return `InvalidArgument`. |
| Identity keys with explicit limit zero, or epoch information with count zero | The unproved behavior is retained; requesting a zero-limit proof returns `InvalidArgument`. |
| Contract history with explicit limit zero, or protocol-version vote status with count zero | Both proof modes return `InvalidArgument`. |
| Document request V0 limit zero, or shielded encrypted-note count zero | Zero retains its existing default meaning. |
| Root path-element query with no keys, or specific identity keys with no IDs and no limit | Successful empty results and verifiable proofs are retained. |

An empty proved identity-balance selection previously returned a proof whose
query could not be verified by the client. Rejecting that request avoids treating
an unusable success as a proof failure from an honest node. These checks belong
to the public request boundary; they do not change persisted data, consensus
execution, fees, or proof generations. Clients connected to older servers still
receive those servers' previous statuses or unusable proofs.

The unproved identity `SpecificKeys` query currently fails inside query lowering
when an explicit limit is smaller than the number of distinct requested keys,
including missing keys and a zero limit. Its return-cap contract permits that
request. The proved form does not take this lowering path, and its proofs for
an identity with no keys verify successfully. This remains a separate query
defect that can produce `Internal` and a node ban for raw unproved callers; it
is not reclassified as invalid caller input.

## The Drive `Error` enum

The top-level error type lives in `packages/rs-drive/src/error/mod.rs`:

```rust
/// Errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Query error
    #[error("query: {0}")]
    Query(#[from] QuerySyntaxError),
    /// Storage Flags error
    #[error("storage flags: {0}")]
    StorageFlags(#[from] StorageFlagsError),
    /// Drive error
    #[error("drive: {0}")]
    Drive(#[from] DriveError),
    /// Proof error
    #[error("proof: {0}")]
    Proof(#[from] ProofError),
    /// GroveDB error
    #[error("grovedb: {0}")]
    GroveDB(Box<grovedb::Error>),
    /// Protocol error
    #[error("protocol: {0}")]
    Protocol(Box<ProtocolError>),
    /// Identity error
    #[error("identity: {0}")]
    Identity(#[from] IdentityError),
    /// Fee error
    #[error("fee: {0}")]
    Fee(#[from] FeeError),
    /// Document error
    #[error("document: {0}")]
    Document(#[from] DocumentError),
    /// Value error
    #[error("value: {0}")]
    Value(#[from] ValueError),
    /// DataContract error
    #[error("contract: {0}")]
    DataContract(#[from] DataContractError),
    /// Cache error
    #[error("contract: {0}")]
    Cache(#[from] CacheError),
    /// Protocol error with info string
    #[error("protocol: {0} ({1})")]
    ProtocolWithInfoString(Box<ProtocolError>, String),
    /// IO error with info string
    #[error("io: {0} ({1})")]
    IOErrorWithInfoString(Box<io::Error>, String),
}
```

This enum is a classic Rust error aggregator. It collects errors from every subsystem that Drive interacts with -- the query parser, the storage flags system, GroveDB, the protocol layer, identities, fees, documents, data contracts, and the cache. Each variant wraps a specific error type from that subsystem.

Notice the organizational difference from `ConsensusError`:
- `ConsensusError` is organized by **validation phase** (basic, state, signature, fee)
- Drive's `Error` is organized by **subsystem** (query, storage, grovedb, protocol, identity, etc.)

This makes sense. Consensus errors are about "what rule was violated." Drive errors are about "what component failed."

## `Box<ProtocolError>` -- avoiding large enum variants

Two variants wrap their inner error in a `Box`:

```rust
/// GroveDB error
#[error("grovedb: {0}")]
GroveDB(Box<grovedb::Error>),

/// Protocol error
#[error("protocol: {0}")]
Protocol(Box<ProtocolError>),
```

Why? Because `ProtocolError` and `grovedb::Error` are large types. Without the `Box`, the entire `Error` enum would be as large as its largest variant, which could be hundreds of bytes. Since most Drive operations return `Result<T, Error>`, you would be paying that size cost on every `Ok` path too -- the `Result` itself is as large as the larger of `T` and `Error`.

Boxing the large variants means the `Error` enum stores only an 8-byte pointer for those cases, keeping the overall size reasonable. This is a common Rust pattern, and Clippy will warn you about it via the `clippy::large_enum_variant` lint.

## The `From` trait chain

The `#[from]` attribute on most variants is a `thiserror` feature that automatically generates `From` implementations. For example, `#[from] DriveError` generates:

```rust
impl From<DriveError> for Error {
    fn from(value: DriveError) -> Self {
        Self::Drive(value)
    }
}
```

But look at the manually written `From` implementations at the bottom of the file:

```rust
impl From<ProtocolError> for Error {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(Box::new(value))
    }
}

impl From<grovedb::Error> for Error {
    fn from(value: grovedb::Error) -> Self {
        Self::GroveDB(Box::new(value))
    }
}

impl From<grovedb::element::error::ElementError> for Error {
    fn from(value: grovedb::element::error::ElementError) -> Self {
        Self::GroveDB(Box::new(grovedb::Error::ElementError(value)))
    }
}

impl From<ProtocolDataContractError> for Error {
    fn from(value: ProtocolDataContractError) -> Self {
        Self::Protocol(Box::new(ProtocolError::DataContractError(value)))
    }
}
```

These cannot use `#[from]` because they involve boxing or wrapping through an intermediate type. The `ProtocolError` conversion boxes the value. The `grovedb::element::error::ElementError` conversion wraps the error inside `grovedb::Error::ElementError` first, then boxes it. The `ProtocolDataContractError` conversion wraps through `ProtocolError::DataContractError` and then boxes.

These manual `From` implementations create **multi-hop error conversion chains**. When a function deep in Drive's GroveDB interaction layer returns a `grovedb::element::error::ElementError`, the `?` operator can propagate it all the way up to a `drive::Error` in a single step, with the conversion chain handling the wrapping automatically.

## The `DriveError` enum -- internal errors

The `DriveError` enum in `packages/rs-drive/src/error/drive.rs` is where Drive reports its own internal problems:

```rust
/// Drive errors
#[derive(Debug, thiserror::Error)]
pub enum DriveError {
    /// This error should never occur, it is the equivalent of a panic.
    #[error("corrupted code execution error: {0}")]
    CorruptedCodeExecution(&'static str),

    /// Platform expected some specific versions
    #[error("drive unknown version on {method}, received: {received}")]
    UnknownVersionMismatch {
        method: String,
        known_versions: Vec<FeatureVersion>,
        received: FeatureVersion,
    },

    /// A critical corrupted state should stall the chain.
    #[error("critical corrupted state error: {0}")]
    CriticalCorruptedState(&'static str),

    /// Error
    #[error("not supported error: {0}")]
    NotSupported(&'static str),

    // ... many more variants
}
```

Notice how `DriveError` uses `&'static str` for most of its error messages rather than `String`. This is deliberate -- these are internal error messages that should be known at compile time. A `CorruptedCodeExecution("document tree missing expected root")` is a fixed message that describes a specific bug or data corruption scenario. Using `&'static str` makes it clear that these are not user-facing messages and avoids allocations on error paths.

There are a few variants that do use `String` -- these are cases where the error message needs to include runtime data:

```rust
#[error("corrupted contract indexes error: {0}")]
CorruptedContractIndexes(String),

#[error("corrupted drive state error: {0}")]
CorruptedDriveState(String),
```

## The severity spectrum

`DriveError` variants implicitly encode severity through naming conventions:

- **`Corrupted*`** variants indicate data corruption. The database is in an unexpected state. These are serious problems that may indicate bugs or hardware failures:
  ```rust
  CorruptedCodeExecution(&'static str),
  CriticalCorruptedState(&'static str),
  CorruptedContractPath(&'static str),
  CorruptedDocumentPath(&'static str),
  CorruptedBalancePath(&'static str),
  CorruptedSerialization(String),
  CorruptedElementType(&'static str),
  CorruptedDriveState(String),
  ```

- **`Invalid*`** and **`NotSupported`** variants indicate logic errors -- the code is trying to do something that should not be possible:
  ```rust
  InvalidDeletionOfDocumentThatKeepsHistory(&'static str),
  InvalidContractHistoryFetchLimit(u16),
  NotSupported(&'static str),
  ```

- **`*NotFound`** and **`*DoesNotExist`** variants indicate missing data that was expected:
  ```rust
  DataContractNotFound(String),
  ElementNotFound(&'static str),
  PrefundedSpecializedBalanceDoesNotExist(String),
  ```

- **Version mismatch** variants indicate protocol version problems:
  ```rust
  UnknownVersionMismatch { method, known_versions, received },
  VersionNotActive { method, known_versions },
  ```

## When to use consensus errors vs drive errors

This is the key design decision you face when adding error handling to Drive code:

**Use a consensus error** when:
- The error is caused by invalid user input (a malformed state transition)
- The error needs to be communicated back to the client
- The error needs a stable numeric code
- Other nodes must produce the same error for the same input

**Use a drive error** when:
- The error is caused by internal state (database corruption, missing paths)
- The error indicates a bug in the platform code
- The error is about version mismatches or unsupported features
- The error involves the storage layer (GroveDB problems)

In practice, Drive functions often work with both. A typical pattern is a function that validates input (producing consensus errors) and then performs storage operations (which might produce drive errors). The return type is usually `Result<T, Error>` where `Error` is the Drive error, and consensus errors are embedded inside `ProtocolError` which is itself a variant of the Drive error.

## The `?` propagation pattern

Here is how error propagation typically works in Drive code:

```rust
fn apply_document_create(
    &self,
    document: &Document,
    contract: &DataContract,
    // ...
) -> Result<(), Error> {
    // This might return a grovedb::Error, which auto-converts via From
    let existing = self.grove_get(path, key, transaction)?;

    // This might return a DriveError
    if existing.is_some() {
        return Err(DriveError::CorruptedDocumentAlreadyExists(
            "document should not exist at this point"
        ).into());
    }

    // This might return a ProtocolError, which auto-converts via From + Box
    let serialized = document.serialize(platform_version)?;

    // This might return a grovedb::Error
    self.grove_insert(path, key, element, transaction)?;

    Ok(())
}
```

The `?` operator handles all the conversions transparently. A `grovedb::Error` becomes `Error::GroveDB(Box::new(...))`. A `DriveError` becomes `Error::Drive(...)`. A `ProtocolError` becomes `Error::Protocol(Box::new(...))`. The developer does not need to think about which `From` implementation is being invoked -- the type system figures it out.

## Sub-module error types

Drive organizes its errors into submodules, each with its own error enum:

```rust
pub mod cache;      // CacheError
pub mod contract;   // DataContractError (Drive's own, distinct from DPP's)
pub mod document;   // DocumentError
pub mod drive;      // DriveError
pub mod fee;        // FeeError (Drive's own, distinct from consensus FeeError)
pub mod identity;   // IdentityError
pub mod proof;      // ProofError
pub mod query;      // QuerySyntaxError
```

Each of these has a `#[from]` conversion to the top-level `Error`, creating a clean hierarchy where subsystem-specific code can work with its own error type and callers can propagate through `?` to the unified type.

## Rules

**Do:**
- Use `Box` for large error types (ProtocolError, grovedb::Error) to keep the enum small
- Use `&'static str` for internal error messages that are known at compile time
- Use `String` only when the message needs runtime data
- Let `#[from]` generate `From` implementations where possible
- Write manual `From` implementations when boxing or intermediate wrapping is needed
- Follow the naming conventions: `Corrupted*` for data corruption, `Invalid*` for logic errors, `*NotFound` for missing data

**Do not:**
- Use consensus errors for internal Drive problems
- Use drive errors for user-facing validation failures
- Forget to add a `From` implementation when introducing a new error type
- Return a `String` error message when a typed error variant would be more informative
- Panic in Drive code -- return a `CorruptedCodeExecution` or `CriticalCorruptedState` error instead
