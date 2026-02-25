# Identities

Before a user can do anything on Dash Platform -- register a name, send a contact request, create a data contract -- they need an **identity**. An identity is the platform-level representation of a user. It is the anchor for everything: documents are owned by identities, state transitions are signed by identity keys, and fees are paid from identity balances.

If you are coming from Ethereum, think of an identity as an account. But unlike Ethereum's single-key accounts, a Dash Platform identity can have multiple public keys with different purposes and security levels, making it more flexible and more secure.

## The Identity Enum

Following the standard versioning pattern, `Identity` is defined in `packages/rs-dpp/src/identity/identity.rs`:

```rust
#[derive(Debug, Clone, PartialEq, From)]
pub enum Identity {
    V0(IdentityV0),
}
```

Currently only V0 exists. The `IdentityV0` struct lives in `packages/rs-dpp/src/identity/v0/mod.rs`:

```rust
pub struct IdentityV0 {
    pub id: Identifier,
    pub public_keys: BTreeMap<KeyID, IdentityPublicKey>,
    pub balance: u64,
    pub revision: Revision,
}
```

Four fields. That is it. An identity is remarkably simple:

- **`id`**: A 32-byte unique identifier. For identities created via asset locks, this is derived from the locking transaction. For identities created via address-based funding, it is derived from the input addresses and nonces.

- **`public_keys`**: A `BTreeMap` mapping key IDs (simple integers) to `IdentityPublicKey` objects. Each public key has a purpose (authentication, encryption, decryption, transfer, voting, owner), a security level (master, critical, high, medium), and the actual key data. An identity can have many keys for different scenarios.

- **`balance`**: The identity's credit balance, measured in platform credits. Credits are the unit of account for fee payment on the platform. Users convert Dash into credits through a process called "topping up."

- **`revision`**: A monotonically increasing counter that increments with every identity update (adding keys, disabling keys, etc.). This prevents replay attacks -- each update must reference the current revision.

## The Credit System

Platform credits are the fuel that powers everything on the network. Every state transition (creating a document, updating a contract, transferring tokens) costs credits. The credit system is how the platform measures and charges for computational and storage resources.

The balance field is a simple `u64` representing the number of credits an identity holds. When an operation is performed, the fee system calculates the cost (as we will see in the [Cost Tracking](../drive/cost-tracking.md) chapter) and deducts it from the identity's balance. If the balance is insufficient, the operation is rejected.

## Public Keys and Key Types

An identity's public keys are not just random cryptographic keys -- they are structured with purpose and security level. Each `IdentityPublicKey` carries:

- **Key ID** (`KeyID`): A numeric identifier unique within the identity, used to reference the key.
- **Purpose**: What the key is for -- authentication, encryption, decryption, transfer, voting, or owner operations.
- **Security Level**: How sensitive operations signed by this key are -- master, critical, high, or medium.
- **Key Type**: The cryptographic algorithm -- ECDSA_SECP256K1, BLS12_381, ECDSA_HASH160, BIP13_SCRIPT_HASH, or EDDSA_25519_HASH160.
- **Data**: The actual public key bytes.
- **Disabled At**: An optional timestamp marking when the key was disabled.

This multi-key design means an identity can have a master key stored in cold storage, a critical key for important operations, and a high-level key for everyday use -- all belonging to the same identity. If a day-to-day key is compromised, the master key can disable it and add a replacement without losing the identity.

## The PartialIdentity Pattern

Here is where the codebase reveals a practical optimization. Loading a full identity from storage is expensive -- you need to fetch the balance, all the keys, the revision, and potentially more. But most operations do not need all of that. A balance transfer only needs the balance. A document creation only needs to verify one key.

Enter `PartialIdentity`, defined alongside `Identity` in `packages/rs-dpp/src/identity/identity.rs`:

```rust
pub struct PartialIdentity {
    pub id: Identifier,
    pub loaded_public_keys: BTreeMap<KeyID, IdentityPublicKey>,
    pub balance: Option<Credits>,
    pub revision: Option<Revision>,
    pub not_found_public_keys: BTreeSet<KeyID>,
}
```

A `PartialIdentity` is exactly what it sounds like -- a partially-loaded identity. Notice the differences from `Identity`:

- `balance` is `Option<Credits>` rather than a bare `u64`. It might not have been loaded.
- `revision` is `Option<Revision>`. Same story.
- `loaded_public_keys` might only contain the specific keys that were requested.
- `not_found_public_keys` tracks which keys were requested but did not exist on the identity.

You can convert a full `Identity` into a `PartialIdentity`:

```rust
impl IdentityV0 {
    pub fn into_partial_identity_info(self) -> PartialIdentity {
        let Self { id, public_keys, balance, revision, .. } = self;
        PartialIdentity {
            id,
            loaded_public_keys: public_keys,
            balance: Some(balance),
            revision: Some(revision),
            not_found_public_keys: Default::default(),
        }
    }

    pub fn into_partial_identity_info_no_balance(self) -> PartialIdentity {
        let Self { id, public_keys, revision, .. } = self;
        PartialIdentity {
            id,
            loaded_public_keys: public_keys,
            balance: None,  // explicitly not loaded
            revision: Some(revision),
            not_found_public_keys: Default::default(),
        }
    }
}
```

The `PartialIdentity` pattern is used extensively in Drive's query and validation code. When processing a state transition, Drive fetches only the identity fields it actually needs, wraps them in a `PartialIdentity`, and passes that through the validation pipeline. This avoids unnecessary storage reads and keeps things efficient.

## Identity Nonces

Replay protection on Dash Platform uses **nonces** rather than sequential transaction counters. There are two kinds:

1. **Identity nonce**: A per-identity counter used for identity-level operations (like key updates).
2. **Identity-contract nonce**: A per-identity-per-contract counter used for document operations. This allows operations on different contracts to be submitted in parallel without conflicting.

The nonce system is defined in `packages/rs-dpp/src/identity/identity_nonce.rs` and is more sophisticated than a simple incrementing counter. The nonce value is actually a packed `u64` that contains both the counter value and a bitfield tracking recently-used nonces:

```rust
pub const IDENTITY_NONCE_VALUE_FILTER: u64 = 0xFFFFFFFFFF;
pub const MISSING_IDENTITY_REVISIONS_FILTER: u64 = 0xFFFFFF0000000000;
pub const MAX_MISSING_IDENTITY_REVISIONS: u64 = 24;
```

The lower 40 bits hold the current nonce tip. The upper 24 bits form a bitfield that tracks which of the last 24 nonce values have been seen. This allows out-of-order submission within a window: if a user submits nonces 5, 7, and 6 in that order, all three are accepted. But nonce 5 cannot be submitted again because it is already marked in the bitfield.

The validation function checks several conditions:

```rust
pub fn validate_identity_nonce_update(
    existing_nonce: IdentityNonce,
    new_revision_nonce: IdentityNonce,
    identity_id: Identifier,
) -> SimpleConsensusValidationResult {
    let actual_existing_revision = existing_nonce & IDENTITY_NONCE_VALUE_FILTER;
    match actual_existing_revision.cmp(&new_revision_nonce) {
        std::cmp::Ordering::Equal => {
            // Nonce already used at the tip
            // -> NonceAlreadyPresentAtTip error
        }
        std::cmp::Ordering::Less => {
            // Nonce is in the future -- check it's within window
            // -> NonceTooFarInFuture if gap > 24
        }
        std::cmp::Ordering::Greater => {
            // Nonce is in the past -- check bitfield
            // -> NonceTooFarInPast if gap > 24
            // -> NonceAlreadyPresentInPast if bit is already set
        }
    }
}
```

This design balances several concerns:
- **Replay protection**: A nonce cannot be reused.
- **Out-of-order tolerance**: Within a 24-nonce window, transactions can arrive in any order.
- **Bounded storage**: Only 8 bytes are needed to track the full nonce state (the packed `u64`).
- **Parallel submission**: Identity-contract nonces let different contracts have independent nonce spaces.

## Creating an Identity

The `Identity` enum provides versioned constructors:

```rust
impl Identity {
    pub fn new_with_id_and_keys(
        id: Identifier,
        public_keys: BTreeMap<KeyID, IdentityPublicKey>,
        platform_version: &PlatformVersion,
    ) -> Result<Identity, ProtocolError> {
        match platform_version
            .dpp
            .identity_versions
            .identity_structure_version
        {
            0 => {
                let identity_v0 = IdentityV0 {
                    id,
                    public_keys,
                    balance: 0,
                    revision: 0,
                };
                Ok(identity_v0.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Identity::new_with_id_and_keys".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
```

New identities start with a balance of 0 and a revision of 0. The balance is filled by the identity creation state transition (which includes an asset lock or address-based funding), and the revision increments from there.

## How Identity Differs from Other Types

One important thing to note: the identity is **not stored as a single blob in Drive**. Unlike documents and data contracts (which are serialized and stored as items), identity fields are stored in separate locations within GroveDB's tree structure. The balance is in one place, each key is in another, the revision somewhere else. This is because different operations need to update different parts of the identity independently and atomically.

The `Identity` struct is primarily used for:
- Creating new identities (assembling all fields for the creation state transition)
- Client-side representation (what the SDK returns when you query an identity)
- Transport (serialized for gRPC responses)

Inside Drive and ABCI, you will more commonly see `PartialIdentity` or direct field access through Drive's identity methods.

## Rules and Guidelines

**Do:**
- Use `PartialIdentity` when you only need a subset of identity fields. It avoids unnecessary storage reads.
- Validate nonces through the provided `validate_identity_nonce_update` function -- the bitfield logic is subtle.
- Always go through `PlatformVersion` when constructing identities to ensure the correct structure version.

**Do not:**
- Assume an identity has only one key. Identities commonly have multiple keys with different purposes and security levels.
- Manually pack or unpack nonce bitfields. Use the provided constants and validation functions.
- Store or cache full `Identity` objects when a `PartialIdentity` would suffice. The full identity can be large if it has many keys.
- Treat identity balance as Dash amounts. Credits are the unit of account on the platform; the conversion to/from Dash happens at the protocol level.
