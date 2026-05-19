# Identity Keys Deep Dive

Every identity on Dash Platform is controlled by a set of **identity public keys**.
These keys determine what the identity can do: sign state transitions, encrypt messages,
transfer credits, vote, or prove masternode ownership. The key system is designed around
three axes -- **purpose**, **security level**, and **key type** -- that together define
what a key is for, how sensitive it is, and what cryptographic algorithm it uses.

This chapter covers the full key lifecycle: structure, creation, storage, validation,
rotation, and the GroveDB tree layout that makes lookups efficient.

## Key Structure

An identity public key is represented by `IdentityPublicKeyV0`:

```rust
pub struct IdentityPublicKeyV0 {
    pub id: KeyID,                            // u32, unique within this identity
    pub purpose: Purpose,                      // what the key is used for
    pub security_level: SecurityLevel,         // how sensitive the key is
    pub key_type: KeyType,                     // cryptographic algorithm
    pub read_only: bool,                       // if true, cannot sign state transitions
    pub data: BinaryData,                      // the public key bytes
    pub disabled_at: Option<TimestampMillis>,  // None = active, Some = disabled
    pub contract_bounds: Option<ContractBounds>, // restrict to specific contract
}
```

Each field serves a specific role:

- **`id`** (`KeyID = u32`): Sequential identifier assigned at creation. Key IDs are
  unique within a single identity but not globally. The ID is used to reference the
  key in state transitions and storage.

- **`data`** (`BinaryData`): The raw public key bytes. Size depends on `key_type`:
  33 bytes for ECDSA, 48 bytes for BLS, 20 bytes for hash-based types.

- **`disabled_at`**: When set, the key can no longer be used to sign anything. This
  is a timestamp (milliseconds since epoch), not a boolean, so you know exactly when
  the key was disabled.

- **`read_only`**: A read-only key can verify signatures but cannot be used to sign
  new state transitions. This is enforced at validation time.

## Purpose

The `Purpose` enum defines what a key is authorized to do:

| Purpose | Value | Description |
|---------|-------|-------------|
| `AUTHENTICATION` | 0 | General-purpose signing. Every identity must have at least one MASTER-level authentication key. |
| `ENCRYPTION` | 1 | Encrypt data. Cannot sign documents or state transitions. |
| `DECRYPTION` | 2 | Decrypt data. Cannot sign documents or state transitions. |
| `TRANSFER` | 3 | Sign credit transfers, withdrawals, and token operations. Required at CRITICAL security level. |
| `SYSTEM` | 4 | System operations. Cannot sign documents. |
| `VOTING` | 5 | Cast masternode votes. Cannot sign documents. |
| `OWNER` | 6 | Prove ownership of a masternode or evonode. |

Purposes are grouped by searchability in the storage layer:

- **Searchable**: `AUTHENTICATION`, `TRANSFER`, `VOTING` -- these get indexed in the
  key reference tree so they can be looked up by purpose and security level.
- **Non-searchable**: `ENCRYPTION`, `DECRYPTION`, `SYSTEM`, `OWNER` -- stored but not
  indexed for search.

The practical effect: if a Platform node needs to find "the TRANSFER key for identity X",
it can do a direct tree lookup. But finding "the ENCRYPTION key for identity X" requires
fetching all keys and filtering client-side.

## Security Level

Security levels form a strict hierarchy:

```
MASTER (0)  >  CRITICAL (1)  >  HIGH (2)  >  MEDIUM (3)
  strongest                                    weakest
```

The numeric value is inverted from what you might expect: lower value = stronger
security. This matters because many operations check
`key.security_level().stronger_or_equal_security_than(required_level)`.

### What Security Level Controls

1. **What the key can sign.** A data contract can require that documents be signed
   with at least a certain security level. A key at MEDIUM cannot sign a document that
   requires HIGH or above.

2. **What operations the key can perform.** Some state transitions require specific
   security levels:
   - Adding/disabling other keys requires MASTER
   - Credit transfers require CRITICAL (enforced via the TRANSFER purpose)
   - Document operations accept HIGH or MEDIUM depending on the contract

3. **Which purposes allow which levels.** Not all combinations are valid for
   externally added keys (i.e., keys added via identity create/update transitions):

   | Purpose | Allowed Security Levels |
   |---------|------------------------|
   | AUTHENTICATION | MASTER, CRITICAL, HIGH, MEDIUM |
   | ENCRYPTION | MEDIUM only |
   | DECRYPTION | MEDIUM only |
   | TRANSFER | CRITICAL only |
   | SYSTEM | Not externally addable (platform-managed) |
   | VOTING | Not externally addable (platform-managed) |
   | OWNER | Not externally addable (platform-managed) |

   SYSTEM, VOTING, and OWNER keys are created automatically by the platform
   (e.g., during masternode registration) and cannot be added through state
   transitions. Attempting to add a key with one of these purposes will fail
   validation. Similarly, attempting to create a TRANSFER key at HIGH security
   level will fail because only CRITICAL is allowed for that purpose.

### The Master Key Requirement

Every identity must have exactly one MASTER-level AUTHENTICATION key at creation time.
This key is the identity's root of trust -- it can add new keys, disable other keys,
and perform any operation. Losing access to the master key means losing the ability to
manage the identity's key set.

## Key Type

The `KeyType` enum determines the cryptographic algorithm and key size:

| Key Type | Value | Size | Unique | Description |
|----------|-------|------|--------|-------------|
| `ECDSA_SECP256K1` | 0 | 33 bytes | Yes | Standard Bitcoin/Dash curve. Default. |
| `BLS12_381` | 1 | 48 bytes | Yes | BLS signatures, used by masternodes. |
| `ECDSA_HASH160` | 2 | 20 bytes | No | RIPEMD160(SHA256) of an ECDSA public key. Core address type. |
| `BIP13_SCRIPT_HASH` | 3 | 20 bytes | No | Script hash. Core address type. |
| `EDDSA_25519_HASH160` | 4 | 20 bytes | No | RIPEMD160(SHA256) of an Ed25519 public key. |

### Unique vs Non-Unique Keys

This distinction is critical for understanding how keys are stored and enforced:

- **Unique key types** (ECDSA_SECP256K1, BLS12_381): The full public key is stored, and
  Platform enforces that no two identities can register the same public key. This is
  checked in both the unique and non-unique hash tables during insertion. If identity A
  registers an ECDSA key, identity B cannot register the same key bytes.

- **Non-unique key types** (ECDSA_HASH160, BIP13_SCRIPT_HASH, EDDSA_25519_HASH160):
  Only a 20-byte hash is stored. Multiple identities can share the same hash. This
  makes sense for address-based key types where the same Dash address might legitimately
  be associated with multiple identities (e.g., through asset lock transactions).

### Core Address Key Types

`ECDSA_HASH160` and `BIP13_SCRIPT_HASH` are specifically for linking Platform identities
to Layer 1 (Core) Dash addresses. They store the same 20-byte hash used in Core
addresses, enabling cross-layer identity verification without revealing the full public
key.

## Contract Bounds

A key can optionally be restricted to operations within a specific data contract:

```rust
pub enum ContractBounds {
    /// Key can only be used within a specific contract
    SingleContract { id: Identifier },

    /// Key can only be used within a specific contract and document type
    SingleContractDocumentType {
        id: Identifier,
        document_type_name: String,
    },
}
```

When `contract_bounds` is set:
- The key can only sign state transitions that target the specified contract.
- With `SingleContractDocumentType`, it is further restricted to a specific document
  type within that contract.
- The key cannot be used for general-purpose operations outside the bound contract.

This enables fine-grained delegation: an identity owner can create a key that is only
allowed to interact with one specific dApp, limiting exposure if that key is compromised.

## Storage in GroveDB

Identity keys are stored across multiple trees in GroveDB for efficient access patterns.

### Identity-Level Trees

Each identity has its own subtree under the root `Identities` tree:

```
Identities [RootTree::Identities]
└── {identity_id (32 bytes)}
    ├── IdentityTreeKeys [128]
    │   └── {key_id (varint)} → serialized IdentityPublicKey
    │
    ├── IdentityTreeKeyReferences [160]
    │   ├── AUTHENTICATION [0]
    │   │   ├── MASTER [0]
    │   │   │   └── {key_id} → reference to IdentityTreeKeys
    │   │   ├── CRITICAL [1]
    │   │   │   └── ...
    │   │   ├── HIGH [2]
    │   │   │   └── ...
    │   │   └── MEDIUM [3]    ← pre-created at identity creation
    │   │       └── ...
    │   ├── TRANSFER [3]
    │   │   └── {key_id} → reference
    │   └── VOTING [5]
    │       └── {key_id} → reference
    │
    ├── IdentityTreeRevision [192]
    ├── IdentityTreeNonce [64]
    └── IdentityContractInfo [32]
```

**IdentityTreeKeys** stores the actual serialized key data, keyed by the key ID
encoded as a varint.

**IdentityTreeKeyReferences** provides a searchable index organized by purpose and
(for AUTHENTICATION) security level. Each entry is a GroveDB reference pointing back
to the actual key in IdentityTreeKeys.

The MEDIUM security level subtree under AUTHENTICATION is pre-created during identity
initialization, even if no MEDIUM keys exist yet. Other security level subtrees are
created on-demand when a key with that level is first added.

### Global Key Hash Tables

Two root-level trees provide reverse lookups from key hashes to identity IDs:

```
UniquePublicKeyHashesToIdentities [24]
└── {key_hash (20 bytes)} → identity_id (32 bytes)

NonUniquePublicKeyKeyHashesToIdentities [8]
└── {key_hash (20 bytes)}
    └── {identity_id (32 bytes)} → empty item
```

The **unique** table is a flat mapping: one hash to one identity. Insertion fails if
the hash already exists in either table.

The **non-unique** table uses a nested structure: each key hash has a subtree containing
identity IDs as keys. This allows multiple identities to share the same key hash.

### Key Hash Computation

All key types are hashed to 20 bytes for storage in the hash tables:

- **ECDSA_SECP256K1** (33 bytes): `RIPEMD160(SHA256(pubkey))`
- **BLS12_381** (48 bytes): `RIPEMD160(SHA256(pubkey))`
- **ECDSA_HASH160** (20 bytes): stored as-is (already a hash)
- **BIP13_SCRIPT_HASH** (20 bytes): stored as-is
- **EDDSA_25519_HASH160** (20 bytes): stored as-is

## Key Lifecycle

### Creation

Keys are added to an identity either at identity creation or via an IdentityUpdate
state transition.

**At Identity Creation:**
- The state transition includes `IdentityPublicKeyInCreation` objects
- Validation enforces exactly one MASTER-level AUTHENTICATION key
- Each key also carries a `signature` field (proving the creator holds the private key)
- After validation, keys are converted to `IdentityPublicKey` with `disabled_at = None`

**Via IdentityUpdate:**
- The `add_public_keys` field carries new `IdentityPublicKeyInCreation` objects
- Multiple keys can be added in a single transition
- The transition must be signed by a key with sufficient security level
- New key IDs must not collide with existing keys on the identity

The insertion process differs by key type:

1. **Unique keys**: Check both hash tables for conflicts, insert into
   `UniquePublicKeyHashesToIdentities`, insert key data, create references.
2. **Non-unique keys**: Create subtree under hash if needed, insert identity ID,
   insert key data, create references.

### Disabling

Keys are disabled (not deleted) via the `disable_public_keys` field of IdentityUpdate:

```rust
// IdentityUpdateTransition fields (simplified)
pub add_public_keys: Vec<IdentityPublicKeyInCreation>,
pub disable_public_keys: Vec<KeyID>,
```

When a key is disabled:
1. The key is fetched from storage.
2. `disabled_at` is set to the current block timestamp (milliseconds).
3. The serialized key is replaced in `IdentityTreeKeys`.
4. Key references in `IdentityTreeKeyReferences` are refreshed.

A disabled key:
- Cannot be used to sign any state transition (checked during signature verification)
- Remains in storage (can still be read)
- Can be re-enabled in the future

### Re-enabling

Keys can be re-enabled by clearing the `disabled_at` field:
1. The key is fetched from storage.
2. `disabled_at` is set to `None`.
3. The serialized key is replaced.
4. References are refreshed.

### Masternode Keys

Masternode identities have a special rule: all their keys are registered as non-unique,
regardless of key type. This allows the same BLS key to be used across multiple
masternode identities (e.g., during key rotation or when the same operator runs
multiple masternodes).

## Signing and Verification

When a state transition is signed:

1. The transition specifies which key ID it was signed with.
2. The key is looked up on the signing identity.
3. Validation checks:
   - The key exists and is not disabled (`disabled_at` must be `None`)
   - The key's purpose allows this type of state transition
   - The key's security level meets the minimum required
   - If the key has `contract_bounds`, the transition targets the bound contract
   - If the key is `read_only`, it cannot sign
4. The signature is verified using the appropriate algorithm:
   - ECDSA_SECP256K1: standard secp256k1 signature verification
   - BLS12_381: BLS signature verification
   - ECDSA_HASH160: ECDSA verification (key data is a hash, so verification uses
     the hash comparison path)

## Validation Rules Summary

**At identity creation:**
- Exactly 1 MASTER-level AUTHENTICATION key required
- No duplicate key IDs in the transition
- No duplicate key data for unique key types
- Key count must not exceed `max_public_keys_in_creation` (platform config)
- Each key's purpose/security level combination must be in the allowed set

**At key addition (IdentityUpdate):**
- New key IDs must not conflict with existing keys
- Unique key hashes must not exist in either the unique or non-unique global tables
- The signing key must have sufficient security level to add keys
- All purpose/security level constraints apply

**At signing time:**
- Key must not be disabled
- Key purpose must match the operation
- Key security level must be >= the required level
- Contract bounds must match (if set)
- Key must not be read-only

## Querying Keys

**Find identity by public key hash:**
```rust
// Returns the identity ID that owns this unique public key
let identity_id = drive.fetch_identity_id_by_unique_public_key_hash(
    key_hash, transaction, platform_version
)?;
```

**Find all identities sharing a non-unique key hash:**
```rust
// Returns all identity IDs registered under this non-unique key hash
let identity_ids = drive.fetch_identity_ids_by_non_unique_public_key_hash(
    key_hash, transaction, platform_version
)?;
```

**Fetch all keys for an identity:**
```rust
let keys = identity.public_keys();  // BTreeMap<KeyID, IdentityPublicKey>
```

**Fetch by purpose and security level:**
Uses the `IdentityTreeKeyReferences` tree to efficiently look up keys without
scanning all keys on the identity.

## Design Rationale

**Why separate purpose and security level?** Purpose defines *what* a key can do;
security level defines *how sensitive* it is. An AUTHENTICATION key at MEDIUM can sign
low-sensitivity documents. An AUTHENTICATION key at MASTER can manage the identity
itself. This separation lets identities create keys with exactly the right
capabilities -- not too much, not too little.

**Why disable instead of delete?** Disabled keys remain in storage so that historical
signatures can still be verified. If a key were deleted, past state transitions signed
by that key would become unverifiable.

**Why unique vs non-unique hash tables?** Full public keys (ECDSA, BLS) must be globally
unique to prevent impersonation. But hash-based keys (ECDSA_HASH160, BIP13_SCRIPT_HASH)
represent Dash addresses that may legitimately appear in multiple identities -- for
example, when the same address is used in multiple asset lock transactions.

**Why pre-create the MEDIUM AUTHENTICATION subtree?** This is the most commonly used
security level for document signing. Pre-creating its tree at identity creation avoids
the cost of creating it on the first document submission.

**Why contract bounds?** They enable the principle of least privilege. An identity can
create a key specifically for interacting with one dApp. If that key is compromised,
the damage is limited to that single contract -- the attacker cannot use it to transfer
credits or interact with other contracts.
