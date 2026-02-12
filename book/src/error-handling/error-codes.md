# Error Codes

In the previous chapter we saw how `ConsensusError` organizes errors into a tree of enums and structs. But when a client -- whether a JavaScript SDK, a mobile app, or a third-party tool -- receives a validation failure from the platform, it does not receive a Rust enum. It receives bytes on a wire. To make those bytes useful, every consensus error maps to a **stable numeric code** through the `ErrorWithCode` trait.

These codes are the public API of the error system. They appear in gRPC responses, they are documented for client developers, and they must never change once assigned. A client that handles error code 40100 ("document already present") today must still be able to handle that same code a year from now, even after dozens of protocol upgrades.

## The `ErrorWithCode` trait

The trait lives in `packages/rs-dpp/src/errors/consensus/codes.rs` and is as simple as it gets:

```rust
pub trait ErrorWithCode {
    /// Returns the error code
    fn code(&self) -> u32;
}
```

One method, one number, no room for ambiguity. The trait is implemented for `ConsensusError` and for each of the four category enums.

## How `ConsensusError` delegates

The top-level implementation just forwards to the appropriate category:

```rust
impl ErrorWithCode for ConsensusError {
    fn code(&self) -> u32 {
        match self {
            Self::BasicError(e) => e.code(),
            Self::SignatureError(e) => e.code(),
            Self::StateError(e) => e.code(),
            Self::FeeError(e) => e.code(),

            #[cfg(test)]
            ConsensusError::TestConsensusError(_) => 1000,
            ConsensusError::DefaultError => 1, // this should never happen
        }
    }
}
```

The `DefaultError` variant returns code 1 -- a sentinel value that should never appear in practice. The `TestConsensusError` returns 1000, safely outside any production range.

## The code ranges

Error codes are organized into ranges that correspond to error categories and subcategories. Here is the complete map as it stands in the codebase:

### BasicError codes (10000-10899)

| Range | Category | Examples |
|-------|----------|----------|
| 10000-10099 | Versioning | `UnsupportedVersionError` (10000), `ProtocolVersionParsingError` (10001), `IncompatibleProtocolVersionError` (10004) |
| 10100-10199 | Structure | `JsonSchemaCompilationError` (10100), `InvalidIdentifierError` (10102), `ValueError` (10103) |
| 10200-10275 | Data Contract | `DataContractMaxDepthExceedError` (10200), `DuplicateIndexError` (10201), `InvalidDataContractIdError` (10204) |
| 10350-10359 | Groups | `GroupPositionDoesNotExistError` (10350), `GroupExceedsMaxMembersError` (10354) |
| 10400-10418 | Documents | `DataContractNotPresentError` (10400), `DuplicateDocumentTransitionsWithIdsError` (10401) |
| 10450-10460 | Tokens | `InvalidTokenIdError` (10450), `TokenTransferToOurselfError` (10456) |
| 10500-10533 | Identity | `DuplicatedIdentityPublicKeyBasicError` (10500), `InvalidIdentityPublicKeyDataError` (10511) |
| 10600-10603 | State Transition | `InvalidStateTransitionTypeError` (10600), `StateTransitionMaxSizeExceededError` (10602) |
| 10700-10700 | General | `OverflowError` (10700) |
| 10800-10818 | Address | `TransitionOverMaxInputsError` (10800), `WithdrawalBelowMinAmountError` (10818) |

### SignatureError codes (20000-20012)

```rust
impl ErrorWithCode for SignatureError {
    fn code(&self) -> u32 {
        match self {
            Self::IdentityNotFoundError { .. } => 20000,
            Self::InvalidIdentityPublicKeyTypeError { .. } => 20001,
            Self::InvalidStateTransitionSignatureError { .. } => 20002,
            Self::MissingPublicKeyError { .. } => 20003,
            Self::InvalidSignaturePublicKeySecurityLevelError { .. } => 20004,
            Self::WrongPublicKeyPurposeError { .. } => 20005,
            Self::PublicKeyIsDisabledError { .. } => 20006,
            Self::PublicKeySecurityLevelNotMetError { .. } => 20007,
            Self::SignatureShouldNotBePresentError(_) => 20008,
            Self::BasicECDSAError(_) => 20009,
            Self::BasicBLSError(_) => 20010,
            Self::InvalidSignaturePublicKeyPurposeError(_) => 20011,
            Self::UncompressedPublicKeyNotAllowedError(_) => 20012,
        }
    }
}
```

Signature errors occupy the 20000 range. There are only 13 of them -- cryptographic validation is relatively binary (the signature is valid or it is not), so there are fewer failure modes to distinguish.

### FeeError codes (30000)

```rust
impl ErrorWithCode for FeeError {
    fn code(&self) -> u32 {
        match self {
            Self::BalanceIsNotEnoughError { .. } => 30000,
        }
    }
}
```

The fee category currently has a single code. The 30000 range is reserved for future fee-related errors.

### StateError codes (40000-40899)

| Range | Category | Examples |
|-------|----------|----------|
| 40000-40009 | Data Contract | `DataContractAlreadyPresentError` (40000), `DataContractIsReadonlyError` (40001), `DataContractNotFoundError` (40008) |
| 40100-40117 | Documents | `DocumentAlreadyPresentError` (40100), `DocumentNotFoundError` (40101), `DuplicateUniqueIndexError` (40105) |
| 40200-40217 | Identity | `IdentityAlreadyExistsError` (40200), `InvalidIdentityRevisionError` (40203), `IdentityInsufficientBalanceError` (40210) |
| 40300-40306 | Voting | `MasternodeNotFoundError` (40300), `MasternodeVoteAlreadyPresentError` (40304) |
| 40400-40401 | Prefunded Balances | `PrefundedSpecializedBalanceInsufficientError` (40400) |
| 40500-40502 | Data Triggers | `DataTriggerConditionError` (40500), `DataTriggerExecutionError` (40501) |
| 40600-40603 | Addresses | `AddressDoesNotExistError` (40600), `AddressNotEnoughFundsError` (40601) |
| 40700-40721 | Tokens | `IdentityDoesNotHaveEnoughTokenBalanceError` (40700), `UnauthorizedTokenActionError` (40701) |
| 40800-40804 | Groups | `IdentityNotMemberOfGroupError` (40800), `GroupActionAlreadyCompletedError` (40802) |

Notice how the `DataTriggerError` sub-enum has its own `ErrorWithCode` implementation that the `StateError` delegates to:

```rust
impl ErrorWithCode for StateError {
    fn code(&self) -> u32 {
        match self {
            // ...
            // Data trigger errors: 40500-40699
            Self::DataTriggerError(ref e) => e.code(),
            // ...
        }
    }
}

impl ErrorWithCode for DataTriggerError {
    fn code(&self) -> u32 {
        match self {
            Self::DataTriggerConditionError { .. } => 40500,
            Self::DataTriggerExecutionError { .. } => 40501,
            Self::DataTriggerInvalidResultError { .. } => 40502,
        }
    }
}
```

This is the only case where `StateError` delegates to a sub-enum rather than mapping directly. All other state error variants map to their codes inline.

## The pattern: one variant, one code, one match arm

Look at how codes are assigned inside `BasicError`:

```rust
impl ErrorWithCode for BasicError {
    fn code(&self) -> u32 {
        match self {
            // Versioning Errors: 10000-10099
            Self::UnsupportedVersionError(_) => 10000,
            Self::ProtocolVersionParsingError { .. } => 10001,
            Self::SerializedObjectParsingError { .. } => 10002,
            Self::UnsupportedProtocolVersionError(_) => 10003,
            Self::IncompatibleProtocolVersionError(_) => 10004,
            Self::VersionError(_) => 10005,
            Self::UnsupportedFeatureError(_) => 10006,

            // Structure Errors: 10100-10199
            Self::JsonSchemaCompilationError(..) => 10100,
            Self::JsonSchemaError(_) => 10101,
            // ...
        }
    }
}
```

Every single variant gets its own match arm and its own code. There is no default case, no wildcard, no range-based mapping. This is deliberate -- if you add a new variant to the enum and forget to add a code for it, the Rust compiler will refuse to compile because the `match` is non-exhaustive. The type system enforces completeness.

## Nested `ContractError` patterns

One interesting detail is how `BasicError` handles `DataContractError` variants. The `ContractError` wrapper contains a nested enum, so the code mapping matches through two levels:

```rust
Self::ContractError(DataContractError::DocumentTypesAreMissingError { .. }) => 10214,
Self::ContractError(DataContractError::DecodingContractError { .. }) => 10222,
Self::ContractError(DataContractError::DecodingDocumentError { .. }) => 10223,
Self::ContractError(DataContractError::InvalidDocumentTypeError { .. }) => 10224,
Self::ContractError(DataContractError::MissingRequiredKey(_)) => 10225,
Self::ContractError(DataContractError::FieldRequirementUnmet(_)) => 10226,
// ... many more
```

Each specific `DataContractError` variant gets its own distinct code. This ensures that clients can distinguish between "the contract is missing a required key" (10225) and "the contract has a wrong type for a value" (10228) even though both come wrapped in `BasicError::ContractError`.

## Adding a new error code

Here is the procedure for adding a new consensus error:

1. **Choose the category.** Is it a structural validation issue (BasicError), a state conflict (StateError), a signature problem (SignatureError), or a fee issue (FeeError)?

2. **Choose the subcategory range.** Look at the existing ranges within that category. If your error is about tokens in `StateError`, you would use the 40700-40799 range.

3. **Pick the next available code.** Within the range, find the highest existing code and add 1. Never reuse a code, even if the original error was removed.

4. **Add the variant** to the appropriate enum (at the end -- remember the ordering rule).

5. **Add the match arm** to the `ErrorWithCode` implementation.

6. **Document the code** so client developers know what it means.

Here is what the diff would look like for adding a hypothetical new token state error:

```rust
// In state_error.rs -- add at the end of the enum:
#[error(transparent)]
TokenNewError(TokenNewError),

// In codes.rs -- add to the StateError match:
Self::TokenNewError(_) => 40722,  // next available in the 40700 range
```

## Why codes can never change

Consider what happens if code 40100 is changed from `DocumentAlreadyPresentError` to something else:

- A client SDK has a handler for code 40100 that displays "This document already exists" to the user.
- After the change, the platform sends 40100 for a completely different error.
- The user sees "This document already exists" when actually their token balance is insufficient.
- Trust in the platform erodes.

Error codes are part of the platform's **wire protocol**. They are as immutable as protobuf field numbers or HTTP status codes. Once assigned, they live forever.

## Rules

**Do:**
- Assign new codes at the end of the appropriate range
- Use the compiler's exhaustive match checking to ensure every variant has a code
- Keep codes within their designated range (do not put a document error in the identity range)
- Leave gaps between ranges for future expansion (this is already done)

**Do not:**
- Change an existing error code -- ever
- Reuse a code from a removed error variant
- Add a wildcard (`_`) match arm to `ErrorWithCode` implementations
- Assign codes outside the established ranges without allocating a new range first
- Skip codes within a range (assign sequentially within each subcategory)
