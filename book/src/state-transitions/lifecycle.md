# The State Transition Lifecycle

Every change to Dash Platform -- creating an identity, registering a data contract,
storing a document, casting a masternode vote -- follows the same fundamental pattern:
a **state transition**. If you understand this one concept, you understand the
heartbeat of the entire platform.

## What Is a State Transition?

A state transition is the **atomic unit of state change** on Dash Platform. Think of
the platform's state as a database. You cannot write to that database directly. Instead,
you construct a state transition object that describes what you want to change, sign it
with your private key, serialize it to bytes, and broadcast it to the network. Validators
receive it, validate it through a multi-stage pipeline, and -- if everything checks out --
apply it to their copy of the state.

This is fundamentally different from a smart contract model. There is no arbitrary code
execution. Every possible mutation is one of a fixed set of state transition types, each
with its own validation rules hardcoded into the platform. The benefit is predictability:
you can reason about fees, security, and correctness without worrying about
Turing-complete execution.

## The StateTransition Enum

At the Rust level, every state transition is a variant of a single enum. This is defined
in `packages/rs-dpp/src/state_transition/mod.rs`:

```rust
#[derive(Debug, Clone, Encode, Decode, PlatformSerialize,
         PlatformDeserialize, PlatformSignable, From, PartialEq)]
#[platform_serialize(unversioned)]
#[platform_serialize(limit = 100000)]
pub enum StateTransition {
    DataContractCreate(DataContractCreateTransition),
    DataContractUpdate(DataContractUpdateTransition),
    Batch(BatchTransition),
    IdentityCreate(IdentityCreateTransition),
    IdentityTopUp(IdentityTopUpTransition),
    IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition),
    IdentityUpdate(IdentityUpdateTransition),
    IdentityCreditTransfer(IdentityCreditTransferTransition),
    MasternodeVote(MasternodeVoteTransition),
    IdentityCreditTransferToAddresses(IdentityCreditTransferToAddressesTransition),
    IdentityCreateFromAddresses(IdentityCreateFromAddressesTransition),
    IdentityTopUpFromAddresses(IdentityTopUpFromAddressesTransition),
    AddressFundsTransfer(AddressFundsTransferTransition),
    AddressFundingFromAssetLock(AddressFundingFromAssetLockTransition),
    AddressCreditWithdrawal(AddressCreditWithdrawalTransition),
}
```

Each variant wraps a dedicated struct. Notice the derive macros: `Encode` and `Decode`
for bincode serialization, `PlatformSerialize` and `PlatformDeserialize` for the
platform's own serialization layer, and `PlatformSignable` for generating the
"signable bytes" that get signed.

These variants fall into natural groups:

**Identity lifecycle:**
- `IdentityCreate` -- Register a new identity (funded by an asset lock on the Dash core chain)
- `IdentityTopUp` -- Add credits to an existing identity (also asset-lock funded)
- `IdentityUpdate` -- Add or disable public keys on an identity
- `IdentityCreditWithdrawal` -- Withdraw credits back to the core chain
- `IdentityCreditTransfer` -- Transfer credits between identities

**Data contracts and documents:**
- `DataContractCreate` -- Register a new data contract (schema)
- `DataContractUpdate` -- Update an existing data contract
- `Batch` -- Create, replace, delete, or transfer documents; mint, burn, transfer, or freeze tokens

**Governance:**
- `MasternodeVote` -- Cast a vote in a contested resource election

**Address-based (newer):**
- `IdentityCreateFromAddresses`, `IdentityTopUpFromAddresses`, `AddressFundsTransfer`,
  `AddressFundingFromAssetLock`, `AddressCreditWithdrawal` -- Operations that use
  platform addresses instead of (or in addition to) identity-based authentication

Each variant has its own numeric discriminant, defined in
`packages/rs-dpp/src/state_transition/state_transition_types.rs`:

```rust
#[repr(u8)]
pub enum StateTransitionType {
    DataContractCreate = 0,
    Batch = 1,
    IdentityCreate = 2,
    IdentityTopUp = 3,
    DataContractUpdate = 4,
    IdentityUpdate = 5,
    IdentityCreditWithdrawal = 6,
    IdentityCreditTransfer = 7,
    MasternodeVote = 8,
    IdentityCreditTransferToAddresses = 9,
    IdentityCreateFromAddresses = 10,
    IdentityTopUpFromAddresses = 11,
    AddressFundsTransfer = 12,
    AddressFundingFromAssetLock = 13,
    AddressCreditWithdrawal = 14,
}
```

This type tag is what allows the platform to deserialize a raw byte buffer into the
correct variant. When bytes arrive over the wire, the first byte tells the deserializer
which struct to decode into.

## The Batch Transition: A Swiss Army Knife

The `Batch` variant deserves special attention because it is the most complex. A single
`BatchTransition` can contain multiple sub-transitions, each operating on a different
document or token. The sub-transitions include:

- **Document operations:** Create, Replace, Delete, Transfer, UpdatePrice, Purchase
- **Token operations:** Transfer, Mint, Burn, Freeze, Unfreeze, DestroyFrozenFunds,
  EmergencyAction, ConfigUpdate, Claim, DirectPurchase, SetPriceForDirectPurchase

This batching is important for atomicity: either all operations in the batch succeed
or none of them do. It also means a single identity nonce covers the entire batch,
preventing partial replay attacks.

## Signatures and Authentication

State transitions carry cryptographic signatures that prove authorization. There are
two fundamentally different authentication models:

**Identity-signed transitions** -- The majority of transition types. The signer is an
identity that already exists on the platform. The transition carries a
`signature_public_key_id` referencing a key in the identity's key set, plus a `signature`
over the signable bytes.

**Asset-lock transitions** -- `IdentityCreate` and `IdentityTopUp` are special because
the identity may not exist yet (or the transition does not require an identity signature).
Instead, they carry an `AssetLockProof` that references a transaction on the Dash core
chain. The signature proves ownership of the funds being locked.

**Address-based transitions** -- Newer transition types like `IdentityCreateFromAddresses`
use platform address inputs with their own nonces and balances, rather than identity-based
authentication.

The `sign` method on `StateTransition` handles this:

```rust
pub fn sign(
    &mut self,
    identity_public_key: &IdentityPublicKey,
    private_key: &[u8],
    bls: &impl BlsModule,
) -> Result<(), ProtocolError> { ... }
```

It first verifies that the key's purpose and security level are appropriate for
this transition type, then signs the `signable_bytes()` with the private key.
Supported key types are `ECDSA_SECP256K1`, `ECDSA_HASH160`, and `BLS12_381`.

The signable bytes are produced by the `PlatformSignable` derive macro. It serializes
the transition with the signature field zeroed out, producing a deterministic byte
sequence that both client and platform can independently compute.

## Serialization

The platform uses **bincode** for wire serialization of state transitions, wrapped
in the `PlatformSerialize` / `PlatformDeserialize` layer. This gives us:

- Compact binary encoding (no field names, no JSON overhead)
- Deterministic output (critical for signature verification)
- Size limits (`#[platform_serialize(limit = 100000)]` -- 100KB max)
- Version awareness through the `#[platform_serialize(unversioned)]` annotation

Deserialization includes version-range checks:

```rust
pub fn deserialize_from_bytes_in_version(
    bytes: &[u8],
    platform_version: &PlatformVersion,
) -> Result<Self, ProtocolError> {
    let state_transition = StateTransition::deserialize_from_bytes(bytes)?;
    let active_version_range = state_transition.active_version_range();
    if active_version_range.contains(&platform_version.protocol_version) {
        Ok(state_transition)
    } else {
        Err(ProtocolError::StateTransitionError(
            StateTransitionIsNotActiveError { ... },
        ))
    }
}
```

This ensures that a transition type introduced in protocol version 9 cannot be
submitted to a node running protocol version 8.

### Size caps and decode budgets per family

Two different numbers bound a serialized state transition, and they are
enforced at different points:

- **The wire cap** is a plain comparison on the raw byte length, made before
  any decoding. Every ordinary family shares
  `SystemLimits::max_state_transition_size` (20 KiB). From protocol version 17
  the contract create and update transitions, in the generation that can carry
  a code bundle, get their own cap,
  `SystemLimits::max_contract_code_state_transition_size` (32 MiB,
  provisional).
- **The decode budget** is bincode's `with_limit`. It counts the bytes read
  *and* the allocation claims the `Value` decoder makes for every map and
  array while decoding a contract schema, so it has to sit well above the wire
  cap. The contract-code envelopes decode under
  `SystemLimits::max_contract_code_state_transition_decode_budget` (64 MiB,
  provisional). The ordinary families keep the decode they shipped with: the
  `limit = 100000` on the enum sits in a second `platform_serialize` attribute
  the derive never reads, so `deserialize_from_bytes` applies no bincode
  budget and the wire cap is the only bound on them. That decode is not
  changed retroactively; the contract-code envelopes are the first family
  decoded under an explicit budget.

The family is read from the wire prefix before anything is decoded.
`StateTransition::peek_envelope_kind` looks at the first ten bytes at most
(the outer variant index of `StateTransition`, then the inner variant index
of the contract transition enum) and returns `Ordinary` or
`ContractCodeCapable { family }`; an unknown or truncated prefix is
`Ordinary`, so it is bounded by the small cap and fails decode as it always
did. `family_max_size` and `family_decode_budget` map the kind onto the
tables of the active version, and
`deserialize_from_bytes_in_version_bounded` decodes under that budget and
reports a not-yet-active variant as `StateTransitionNotActiveError` (an
unpaid consensus rejection) rather than as a protocol error. The block
decoder (`decode_raw_state_transitions` v1), the `getProofs` query (v1) and
the DAPI broadcast pre-filter all go through these helpers so every ingress
path applies the same cap; the frozen v0 decoder keeps calling
`deserialize_from_bytes_in_version` unchanged.

Because the block byte limit is a Tenderdash consensus parameter, raising it
for the large envelopes is protocol state as well:
`ConsensusVersions::block_max_bytes` and `block_max_gas` are pushed by
`consensus_params_update` v2 from both proposal paths at the activation
boundary, so every validator adopts them at the same height. The node-local
Tenderdash mempool (`max-tx-bytes`) and RPC (`max-body-bytes`) limits are
dashmate options sized to match.

## The Full Journey

Here is the end-to-end lifecycle of a state transition, from a client's perspective
all the way through to state application:

**1. Client constructs the transition.** Using the Rust SDK (`rs-sdk`), JavaScript
SDK (`js-dash-sdk`), or any client library, the application builds a concrete transition
struct -- for example, a `DataContractCreateTransition` containing the new contract's schema.

**2. Client signs the transition.** The client calls `sign()` or `sign_external()`,
which computes signable bytes, signs them with the appropriate private key, and attaches
the signature and key ID to the transition struct.

**3. Client serializes and broadcasts.** The signed `StateTransition` is serialized
to bytes via `serialize_to_bytes()` and sent to the network through DAPI (the
Decentralized API).

**4. Platform receives the bytes in `check_tx`.** Before a state transition enters the
mempool, it goes through a lighter validation pass. The platform deserializes the bytes,
checks the signature, verifies the identity has sufficient balance, and validates basic
structure. This is a gatekeeper -- it rejects obviously invalid transitions cheaply.

**5. Platform processes during `process_proposal`.** When a block is proposed, each
state transition goes through the full validation pipeline: is_allowed, signature
verification, nonce validation, basic structure, balance checks, advanced structure,
transform_into_action, and state validation. (We will cover this pipeline in detail in
the next chapter.)

**6. Platform applies the action.** If validation succeeds, the resulting
`StateTransitionAction` is converted into `DriveOperation`s, which are converted
into `LowLevelDriveOperation`s, which are applied atomically to GroveDB. Fees are
calculated and deducted. The state has changed.

**7. Result is returned.** The client receives confirmation (or rejection) through DAPI.

## The Versioning Pattern

You will notice that almost every method on `StateTransition` dispatches through
a version table. For example:

```rust
match platform_version
    .drive_abci
    .validation_and_processing
    .process_state_transition
{
    0 => v0::process_state_transition_v0(...),
    version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch { ... })),
}
```

This is the **protocol versioning** pattern. Every behavior that could conceivably change
between protocol versions is gated behind a version number looked up from
`PlatformVersion`. This means a node running protocol version 9 and a node running
protocol version 10 can both validate the same block correctly, each using its own
version's logic. It is how the platform achieves hard-fork-free upgrades.

## The call_method Macro

Since `StateTransition` is an enum with 15 variants, dispatching a method call to
the inner type would require writing out a 15-arm match statement every time. The
codebase solves this with a family of macros:

```rust
macro_rules! call_method {
    ($state_transition:expr, $method:ident) => {
        match $state_transition {
            StateTransition::DataContractCreate(st) => st.$method(),
            StateTransition::DataContractUpdate(st) => st.$method(),
            StateTransition::Batch(st) => st.$method(),
            // ... all 15 variants
        }
    };
}
```

There are several variants: `call_method` for universal methods,
`call_getter_method_identity_signed` for methods that return `Option` (returning `None`
for non-identity-signed transitions like `IdentityCreate`), and
`call_errorable_method_identity_signed` for methods that return `Result` (returning
an error for inapplicable variants).

## Rules and Guidelines

**Do:**
- Always deserialize with `deserialize_from_bytes_in_version` in production code, to
  enforce version-range checks.
- Use `signable_bytes()` when computing what to sign or verify -- never serialize the
  full transition (which includes the signature itself).
- Check `active_version_range()` before processing a transition to ensure it is valid
  for the current protocol version.

**Do not:**
- Assume all transitions have signatures. `IdentityCreateFromAddresses` and
  `AddressFundsTransfer` return `None` from `signature()`.
- Assume all transitions have an `owner_id`. Address-based transitions do not.
- Modify the `StateTransitionType` discriminant values -- they are part of the
  wire format and changing them would break all existing serialized data.
- Add new variants without also updating every `call_method` macro and every
  match statement in the validation pipeline.
