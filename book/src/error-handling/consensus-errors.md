# Consensus Errors

When a Dash Platform node processes a state transition -- a document creation, an identity update, a credit withdrawal -- things can go wrong. The data contract might reference an invalid schema. The signature might not match. The identity might not have enough balance. These are not internal bugs; they are **protocol-level validation failures** that every node on the network must agree on. If one node rejects a transition for reason X and another rejects it for reason Y, the chain forks. If one node returns error code 40100 and another returns 40101, clients break.

This is why consensus errors in Dash Platform are not ordinary Rust errors. They are **serializable, code-stable, network-transmitted data structures** that must produce identical bytes on every node and remain decodable by every client version. The `ConsensusError` enum is the heart of this system.

## The top-level enum

Open `packages/rs-dpp/src/errors/consensus/consensus_error.rs` and you will find the root of the hierarchy:

```rust
#[derive(
    thiserror::Error,
    Debug,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    Clone,
    PartialEq,
)]
#[platform_serialize(limit = 2000)]
#[error(transparent)]
#[allow(clippy::large_enum_variant)]
pub enum ConsensusError {
    /*

    DO NOT CHANGE ORDER OF VARIANTS WITHOUT INTRODUCING OF NEW VERSION

    */
    #[error("default error")]
    DefaultError,

    #[error(transparent)]
    BasicError(BasicError),

    #[error(transparent)]
    StateError(StateError),

    #[error(transparent)]
    SignatureError(SignatureError),

    #[error(transparent)]
    FeeError(FeeError),

    #[cfg(test)]
    #[cfg_attr(test, error(transparent))]
    TestConsensusError(TestConsensusError),
}
```

There are five things worth understanding here.

## The four categories

Every consensus error falls into one of four categories:

- **BasicError** -- structural and syntactic validation failures. The state transition itself is malformed, references a nonexistent document type, has an invalid identifier, exceeds size limits, or fails schema validation. These are caught before the node ever checks persistent state. The `BasicError` enum in `packages/rs-dpp/src/errors/consensus/basic/basic_error.rs` contains over 130 variants organized into sub-groups: versioning errors, structure errors, data contract errors, group errors, document errors, token errors, identity errors, state transition errors, and address errors.

- **StateError** -- the transition is structurally valid but conflicts with the current platform state. A document already exists, an identity nonce is wrong, a token account is frozen, a group action was already completed. The `StateError` enum in `packages/rs-dpp/src/errors/consensus/state/state_error.rs` contains roughly 80 variants covering data contracts, documents, identities, voting, tokens, groups, and address balances.

- **SignatureError** -- the cryptographic signature on the transition is invalid. The identity was not found, the key type is wrong, the key is disabled, the security level is insufficient, or the raw signature verification failed.

- **FeeError** -- the transition is valid in every other way, but the identity cannot pay for it. Currently this category has a single variant: `BalanceIsNotEnoughError`.

This layered structure mirrors the validation pipeline. Platform checks basic structure first, then state, then signatures, then fees. Each layer produces errors from its own category.

## The DO NOT CHANGE ORDER rule

The comment at the top of every consensus error enum is not a suggestion:

```rust
/*

DO NOT CHANGE ORDER OF VARIANTS WITHOUT INTRODUCING OF NEW VERSION

*/
```

Why? Because `ConsensusError` derives `Encode` and `Decode` from bincode. Bincode serializes enum variants by their **ordinal position** -- the first variant is 0, the second is 1, and so on. If you reorder variants, the same bytes now decode to a different error on nodes running different code versions. This is a consensus failure.

The same rule applies to every nested error enum. Here is `FeeError` in `packages/rs-dpp/src/errors/consensus/fee/fee_error.rs`:

```rust
#[derive(
    Error, Debug, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize, Clone,
)]
pub enum FeeError {
    /*

    DO NOT CHANGE ORDER OF VARIANTS WITHOUT INTRODUCING OF NEW VERSION

    */
    #[error(transparent)]
    BalanceIsNotEnoughError(BalanceIsNotEnoughError),
}
```

And the same rule applies to individual error structs. Here is `DocumentAlreadyPresentError` in `packages/rs-dpp/src/errors/consensus/state/document/document_already_present_error.rs`:

```rust
#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document {document_id} is already present")]
#[platform_serialize(unversioned)]
pub struct DocumentAlreadyPresentError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
}
```

Notice the comment says fields too, not just variants. Bincode serializes struct fields in declaration order. Swap two fields and the bytes change.

## The derive stack

Every consensus error type carries the same set of derives:

```rust
#[derive(
    Error,              // thiserror -- provides Display and Error trait
    Debug,              // standard Rust debug formatting
    Clone,              // errors need to be cloneable
    PartialEq,          // errors need to be comparable
    Encode,             // bincode encoding (field-level)
    Decode,             // bincode decoding (field-level)
    PlatformSerialize,  // platform wrapper around bincode (size limits, etc.)
    PlatformDeserialize,// platform wrapper around bincode
)]
```

The `thiserror::Error` derive gives each variant a `Display` implementation. For enums, `#[error(transparent)]` delegates to the inner type's `Display`. For leaf structs, you write the message directly:

```rust
#[error("Document {document_id} is already present")]
pub struct DocumentAlreadyPresentError {
    document_id: Identifier,
}
```

The `Encode` and `Decode` derives come from bincode and handle the actual byte-level serialization of each field. The `PlatformSerialize` and `PlatformDeserialize` derives are the platform's own wrappers that add size limits and error type conversion on top of bincode. We will cover those in the serialization chapters.

## The `#[platform_serialize]` attribute

On the top-level `ConsensusError`, you will notice:

```rust
#[platform_serialize(limit = 2000)]
```

This sets a maximum serialized size of 2000 bytes. If a consensus error somehow serializes to more than 2000 bytes, the platform will return a `MaxEncodedBytesReachedError` instead. This is a defense against oversized error payloads being transmitted across the network.

On individual leaf error structs, you will see a different attribute:

```rust
#[platform_serialize(unversioned)]
```

The `unversioned` flag means the struct does not need a `PlatformVersion` parameter for serialization. Most leaf error structs are simple enough that they do not change between protocol versions -- their serialization format is stable. The top-level `ConsensusError` enum does not use `unversioned` because it may need version-aware behavior as the protocol evolves.

## How errors nest

The nesting is three levels deep:

1. **ConsensusError** -- the top level, with four category variants
2. **Category enums** (BasicError, StateError, SignatureError, FeeError) -- each holds dozens of specific error variants
3. **Leaf error structs** -- the actual error data (identifiers, messages, amounts)

Each level has `From` implementations to make conversion ergonomic:

```rust
// Leaf -> Category
impl From<DocumentAlreadyPresentError> for ConsensusError {
    fn from(err: DocumentAlreadyPresentError) -> Self {
        Self::StateError(StateError::DocumentAlreadyPresentError(err))
    }
}

// Category -> Top-level
impl From<StateError> for ConsensusError {
    fn from(error: StateError) -> Self {
        Self::StateError(error)
    }
}
```

This means you can use `?` to propagate any leaf error up to a `ConsensusError`:

```rust
fn validate_document(doc: &Document) -> Result<(), ConsensusError> {
    if document_exists(doc.id()) {
        return Err(DocumentAlreadyPresentError::new(doc.id()).into());
    }
    Ok(())
}
```

The `.into()` call walks the `From` chain: `DocumentAlreadyPresentError` -> `ConsensusError::StateError(StateError::DocumentAlreadyPresentError(...))`.

## Anatomy of a leaf error

Every leaf error follows the same pattern. Let us look at the full `DocumentAlreadyPresentError`:

```rust
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document {document_id} is already present")]
#[platform_serialize(unversioned)]
pub struct DocumentAlreadyPresentError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
}

impl DocumentAlreadyPresentError {
    pub fn new(document_id: Identifier) -> Self {
        Self { document_id }
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }
}

impl From<DocumentAlreadyPresentError> for ConsensusError {
    fn from(err: DocumentAlreadyPresentError) -> Self {
        Self::StateError(StateError::DocumentAlreadyPresentError(err))
    }
}
```

The pattern is:
1. Fields are private with getter methods
2. A `new()` constructor
3. A `From` impl that chains through the category enum to `ConsensusError`
4. The full derive stack including `PlatformSerialize` and `PlatformDeserialize`
5. The `#[platform_serialize(unversioned)]` attribute
6. The ordering warning comment

## The test-only variant

You may have noticed this in `ConsensusError`:

```rust
#[cfg(test)]
#[cfg_attr(test, error(transparent))]
TestConsensusError(TestConsensusError),
```

This variant only exists in test builds. It allows tests to create synthetic consensus errors without depending on real validation logic. The `#[cfg(test)]` attribute ensures it is completely stripped from production builds and does not affect the binary encoding of the other variants.

## Rules

**Do:**
- Always add new variants at the **end** of the enum
- Always add new fields at the **end** of the struct
- Always include the ordering warning comment in new error types
- Always implement `From<YourError> for ConsensusError` through the appropriate category
- Always derive the full stack: `Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize`
- Use `#[platform_serialize(unversioned)]` on leaf error structs
- Keep fields private with getter methods

**Do not:**
- Reorder existing variants or fields -- this breaks network serialization
- Remove existing variants -- old nodes might still produce them
- Change the `#[error(...)]` message without considering client-side parsing
- Add large fields to error structs -- the 2000-byte limit on `ConsensusError` applies to the entire serialized tree
- Use `#[platform_serialize(limit = ...)]` on leaf structs -- the limit is enforced at the `ConsensusError` level
