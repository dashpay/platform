# Shielded Pool Client Integration Guide

This guide explains how to build client applications that interact with Dash Platform's shielded credit pool. It covers key management, bundle construction, state transition creation, note tracking, and light client synchronization.

For the protocol specification, see [DIP-0040](../dip/dip-0040.md) (shielded credit pool), [DIP-0041](../dip/dip-0041.md) (L1 bridge), and [DIP-0042](../dip/dip-0042.md) (light client syncing).

## Table of Contents

1. [Overview](#1-overview)
2. [Dependencies](#2-dependencies)
3. [Key Management](#3-key-management)
4. [Commitment Tree and Note Tracking](#4-commitment-tree-and-note-tracking)
5. [Platform Sighash](#5-platform-sighash)
6. [Building Shielded State Transitions](#6-building-shielded-state-transitions)
   1. [Shield (Type 15)](#61-shield-type-15)
   2. [ShieldFromAssetLock (Type 18)](#62-shieldfromassetlock-type-18)
   3. [ShieldedTransfer (Type 16)](#63-shieldedtransfer-type-16)
   4. [Unshield (Type 17)](#64-unshield-type-17)
   5. [ShieldedWithdrawal (Type 19)](#65-shieldedwithdrawal-type-19)
7. [Bundle Serialization](#7-bundle-serialization)
8. [Light Client Syncing](#8-light-client-syncing)
9. [Trial Decryption](#9-trial-decryption)
10. [Fee Model](#10-fee-model)
11. [Security Considerations](#11-security-considerations)

---

## 1. Overview

The shielded credit pool enables private value transfers on Dash Platform using the Zcash Orchard protocol. Five state transition types move credits between transparent and shielded domains:

| Type ID | Name | Direction | When to Use |
|---------|------|-----------|-------------|
| 15 | Shield | Transparent -> Pool | Deposit platform credits into the shielded pool |
| 16 | ShieldedTransfer | Pool -> Pool | Transfer value privately within the pool |
| 17 | Unshield | Pool -> Transparent | Withdraw credits to a platform address |
| 18 | ShieldFromAssetLock | Core L1 -> Pool | Deposit directly from a Dash Core asset lock |
| 19 | ShieldedWithdrawal | Pool -> Core L1 | Withdraw credits to a Dash Core address |

All five types carry Orchard bundle data: serialized actions, a Halo 2 zero-knowledge proof, and RedPallas signatures. The client constructs these bundles using the Orchard builder API, then wraps them in the appropriate state transition struct.

### Cryptographic Primitives

| Primitive | Purpose |
|-----------|---------|
| Halo 2 | Zero-knowledge proof system (no trusted setup) |
| RedPallas | Re-randomizable Schnorr signatures on the Pallas curve |
| Sinsemilla | Hash-based commitment scheme for the Merkle tree |
| BLAKE2b-256 | Bundle commitment computation (per ZIP-244) |
| SHA-256 | Platform sighash computation |

---

## 2. Dependencies

The Dash Platform SDK (`dash-sdk`) re-exports all necessary Orchard and commitment tree types behind the `shielded` feature:

```toml
[dependencies]
dash-sdk = { version = "3", features = ["shielded"] }
```

The `shielded` feature enables `ClientCommitmentTree` for wallet-side note tracking and Merkle witness generation, as well as the Orchard builder for constructing shielded bundles. All Orchard types are re-exported from `dash_sdk::grovedb_commitment_tree`:

```rust
use dash_sdk::grovedb_commitment_tree::{
    // Builder
    Builder, BundleType,
    // Key management
    SpendingKey, FullViewingKey, IncomingViewingKey, OutgoingViewingKey,
    SpendAuthorizingKey, Scope,
    // Bundle types
    Bundle, Authorized, Flags, Action,
    // Memo types (Dash uses 36-byte memos, not Zcash 512-byte)
    DashMemo, NoteBytesData,
    // Proof creation/verification
    ProvingKey, VerifyingKey,
    // Note types
    Note, NoteValue, PaymentAddress,
    ExtractedNoteCommitment, Nullifier, Rho, TransmittedNoteCiphertext,
    // Tree types
    Anchor, MerklePath, MerkleHashOrchard,
    // Client tree
    ClientCommitmentTree, Position, Retention,
};
```

For the platform sighash, use the re-exported `dpp`:

```rust
use dash_sdk::dpp::shielded::compute_platform_sighash;
```

Alternatively, for projects that don't use the full SDK, the crate can be used directly:

```toml
[dependencies]
grovedb-commitment-tree = { version = "4", features = ["client"] }
```

---

## 3. Key Management

### Key Hierarchy

The Orchard key hierarchy derives all keys from a single 32-byte spending key:

```
SpendingKey (sk)
  |
  +-- SpendAuthorizingKey (ask)     -- signs spend actions
  |
  +-- FullViewingKey (fvk)          -- derives all viewing keys + addresses
       |
       +-- IncomingViewingKey (ivk) -- detects incoming notes (trial decryption)
       |
       +-- OutgoingViewingKey (ovk) -- recovers sent notes (wallet recovery)
       |
       +-- PaymentAddress           -- derived per-contact diversified address
```

### Creating Keys

```rust
use grovedb_commitment_tree::{
    SpendingKey, FullViewingKey, SpendAuthorizingKey,
    IncomingViewingKey, OutgoingViewingKey, Scope,
};

// Generate or load a 32-byte spending key seed
let sk = SpendingKey::from_bytes(seed_bytes)
    .expect("invalid spending key bytes");

// Derive all other keys
let fvk = FullViewingKey::from(&sk);
let ask = SpendAuthorizingKey::from(&sk);

// Derive payment addresses (use different indices for different contacts)
let default_address = fvk.address_at(0u32, Scope::External);
let contact_address = fvk.address_at(1u32, Scope::External);

// Viewing keys for note detection
let ivk: IncomingViewingKey = fvk.to_ivk(Scope::External);
let ovk: OutgoingViewingKey = fvk.to_ovk(Scope::External);
```

### Key Storage

- **SpendingKey**: Must be stored encrypted. This is the master secret -- anyone who obtains it can spend all shielded funds.
- **FullViewingKey**: Allows detecting all incoming and outgoing notes. Store securely but does not enable spending.
- **IncomingViewingKey**: Allows detecting only incoming notes. Safe to share with a watch-only server for filtered sync (DIP-0043).
- **PaymentAddress**: Safe to share publicly. Give a unique diversified address to each contact for privacy.

---

## 4. Commitment Tree and Note Tracking

### Server-Side Storage: BulkAppendTree

On the platform, encrypted notes are stored in a **CommitmentTree** element backed by a **BulkAppendTree** — a two-level append-only authenticated data structure:

```
CommitmentTree (epoch_size = 2048)
  |
  +-- Buffer (dense Merkle tree, up to 2048 entries)
  |     Entries 0..2047 of the current epoch
  |
  +-- MMR (Merkle Mountain Range of completed epochs)
        Epoch 0: entries 0..2047    (immutable blob, CDN-cacheable)
        Epoch 1: entries 2048..4095 (immutable blob, CDN-cacheable)
        ...
```

When the buffer fills (2048 notes), all entries are compacted into an immutable epoch blob and appended to the MMR. This gives:
- **O(1) append** for new notes
- **O(log n) authenticated reads** by global position
- **CDN-cacheable epoch blobs** for bulk syncing (completed epochs never change)

Each note is stored as `cmx (32 bytes) || encrypted_note (216 bytes)` = 248 bytes, accessed by its global position (0-indexed `u64`).

Separately, the CommitmentTree maintains a **Sinsemilla frontier** in auxiliary storage, used to compute the Orchard anchor (Merkle root) at the end of each block.

### Client-Side: ClientCommitmentTree

The `ClientCommitmentTree` maintains a local copy of the on-chain Sinsemilla Merkle tree (depth 32). It supports:

- Appending note commitments as they appear on-chain
- Checkpointing after each block
- Generating Merkle witnesses (authentication paths) for spending notes

```rust
use grovedb_commitment_tree::{ClientCommitmentTree, Retention, Position, Anchor, MerklePath};

// Create a new client tree (retain up to 1000 checkpoints)
let mut tree = ClientCommitmentTree::new(1000);

// Append notes as they appear on-chain (in global position order)
// Use Retention::Marked for notes belonging to this wallet (need witnesses later)
// Use Retention::Ephemeral for notes belonging to other wallets
tree.append(cmx_bytes, Retention::Marked)?;   // Our note
tree.append(other_cmx, Retention::Ephemeral)?; // Someone else's note

// Checkpoint after each block
tree.checkpoint(block_height)?;

// Get the current anchor (Merkle root)
let anchor: Anchor = tree.anchor()?;

// Generate a witness for spending a note at a known position
let merkle_path: MerklePath = tree.witness(position, 0)?
    .expect("witness should exist for marked leaf");
```

The `ClientCommitmentTree` tracks the Sinsemilla tree only — it does not replicate the BulkAppendTree structure. The server stores notes in the BulkAppendTree for efficient retrieval; the client appends cmx values to its Sinsemilla tree for witness generation.

### Wallet Note State

A wallet tracks each note through its lifecycle:

```
Created (cmx appended to tree)
  |
  +-- Unspent (nullifier not seen on-chain)
  |     |
  |     +-- Spendable (witness available at current anchor)
  |
  +-- Spent (nullifier published on-chain)
```

For each detected note, store:

| Field | Source | Purpose |
|-------|--------|---------|
| `Note` | Trial decryption | The note object (value, address, rho, rseed) |
| `Position` | Tree append order (= global position) | Location in commitment tree (for witness generation) |
| `cmx` | `ExtractedNoteCommitment::from(note.commitment())` | For tree tracking |
| `nullifier` | Known from note + spending key | To detect when the note is spent |
| `block_height` | Block where cmx appeared | For sync tracking |

---

## 5. Platform Sighash

The **platform sighash** cryptographically binds Orchard bundle data to platform-specific transparent fields. It is the hash that all Orchard signatures commit to.

```
sighash = SHA-256("DashPlatformSighash" || bundle_commitment || extra_data)
```

Where:
- `"DashPlatformSighash"` is a fixed 19-byte ASCII domain separator
- `bundle_commitment` is the 32-byte BLAKE2b-256 Orchard bundle commitment (per ZIP-244), covering: flags, value_balance, anchor, and all action fields (nullifier, rk, cmx, cv_net, encrypted_note) -- but NOT signatures or proof
- `extra_data` varies by transition type:

| Transition | extra_data | Rationale |
|------------|------------|-----------|
| Shield | empty (`&[]`) | Witness signatures already authenticate inputs |
| ShieldFromAssetLock | empty (`&[]`) | Asset lock proof authenticates the source |
| ShieldedTransfer | empty (`&[]`) | No transparent fields exist |
| Unshield | `output_address.to_bytes() \|\| amount.to_le_bytes()` | Binds destination and amount to the proof |
| ShieldedWithdrawal | `output_script \|\| amount.to_le_bytes()` | Binds Core script and amount to the proof |

### Computing the Sighash

```rust
use dpp::shielded::compute_platform_sighash;

// After building the bundle but before signing:
let bundle_commitment: [u8; 32] = unauthorized_bundle.commitment().into();

// For Shield, ShieldFromAssetLock, or ShieldedTransfer (no extra data):
let sighash = compute_platform_sighash(&bundle_commitment, &[]);

// For Unshield (bind output_address and amount):
let mut extra_data = output_address.to_bytes();
extra_data.extend_from_slice(&amount.to_le_bytes());
let sighash = compute_platform_sighash(&bundle_commitment, &extra_data);

// For ShieldedWithdrawal (bind output_script and amount):
let mut extra_data = output_script.to_bytes();
extra_data.extend_from_slice(&amount.to_le_bytes());
let sighash = compute_platform_sighash(&bundle_commitment, &extra_data);
```

The same sighash must be computed identically on both the signing (client) and verification (platform) sides. If any transparent field is modified after signing, verification will fail.

---

## 6. Building Shielded State Transitions

### Common Pattern

All shielded transitions follow the same five-step pattern:

1. **Create an Orchard builder** with the appropriate flags and anchor
2. **Add spends and/or outputs** to the builder
3. **Build, prove, and sign** the bundle using the platform sighash
4. **Serialize the bundle** into platform format (`SerializedAction` structs)
5. **Wrap in a state transition** and broadcast

### ProvingKey Caching

The `ProvingKey` takes approximately 30 seconds to build. Cache it for the lifetime of the application:

```rust
use std::sync::OnceLock;
use grovedb_commitment_tree::ProvingKey;

static PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();

fn get_proving_key() -> &'static ProvingKey {
    PROVING_KEY.get_or_init(ProvingKey::build)
}
```

### 6.1 Shield (Type 15)

Deposits credits from transparent platform addresses into the shielded pool. This is an **output-only** bundle (no spends).

```rust
use grovedb_commitment_tree::{
    Builder, BundleType, Flags as OrchardFlags, Anchor,
    SpendingKey, FullViewingKey, NoteValue, Scope,
};
use dpp::shielded::compute_platform_sighash;
use dpp::state_transition::state_transitions::shielded::shield_transition::ShieldTransition;

// 1. Setup keys and recipient
let sk = SpendingKey::from_bytes(seed)?;
let fvk = FullViewingKey::from(&sk);
let recipient = fvk.address_at(0u32, Scope::External);

// 2. Build output-only bundle (spends disabled for shielding)
let anchor = Anchor::empty_tree(); // No spends, so anchor is unused
let mut builder = Builder::<DashMemo>::new(
    BundleType::Transactional {
        flags: OrchardFlags::SPENDS_DISABLED,
        bundle_required: false,
    },
    anchor,
);

let shield_amount: u64 = 100_000; // Credits to shield
builder.add_output(
    None,           // No outgoing viewing key needed
    recipient,
    NoteValue::from_raw(shield_amount),
    [0u8; 36],     // 36-byte structured memo
)?;

// 3. Build -> prove -> sign
let pk = get_proving_key();
let mut rng = rand::rngs::OsRng;
let (unauthorized, _) = builder
    .build::<i64>(&mut rng)?
    .expect("bundle should be present");

let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
let sighash = compute_platform_sighash(&bundle_commitment, &[]);
let proven = unauthorized.create_proof(pk, &mut rng)?;
let bundle = proven.apply_signatures(rng, sighash, &[])?; // No spend auth keys

// 4. Serialize bundle (see Section 7)
let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
    serialize_authorized_bundle(&bundle);

// 5. Create state transition
// `inputs` = map of platform addresses contributing credits
// `signer` = signs the address witnesses
let transition = ShieldTransition::try_from_bundle_with_signer(
    inputs,           // BTreeMap<PlatformAddress, (AddressNonce, Credits)>
    actions,
    flags,
    value_balance,    // Negative (credits flow INTO pool)
    anchor_bytes,
    proof_bytes,
    binding_sig,
    fee_strategy,     // Which inputs pay fees
    signer,           // Signs address witnesses
    user_fee_increase,
    platform_version,
)?;
```

**Key points:**
- `value_balance` will be **negative** (credits flow into the pool)
- The shield amount equals `|value_balance|`
- Fees are paid from the transparent platform address inputs
- No `SpendAuthorizingKey` needed (empty `&[]` for signatures)

### 6.2 ShieldFromAssetLock (Type 18)

Deposits credits directly from a Dash Core asset lock proof. Identical bundle construction to Shield, but the funding source is a core asset lock instead of platform address balances.

```rust
use dpp::state_transition::state_transitions::shielded::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;

// Bundle construction is identical to Shield (output-only, empty sighash)
// ... (same builder/prove/sign steps as Shield) ...

// Create state transition with asset lock proof
let transition = ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle(
    asset_lock_proof,              // From Dash Core
    asset_lock_private_key_bytes,  // Signs the asset lock
    actions,
    flags,
    value_balance,
    anchor_bytes,
    proof_bytes,
    binding_sig,
    user_fee_increase,
    platform_version,
)?;
```

### 6.3 ShieldedTransfer (Type 16)

Transfers value privately within the shielded pool. **Spends** an existing note and creates a new output note. The ZK proof is the sole authorization.

```rust
use dpp::state_transition::state_transitions::shielded::shielded_transfer_transition::ShieldedTransferTransition;

// 1. Get a spendable note with its Merkle witness
let (note, merkle_path, anchor) = wallet.take_spendable_note()?;
let note_value = note.value().inner();

// 2. Build spend + output bundle
let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
builder.add_spend(fvk.clone(), note, merkle_path)?;
builder.add_output(
    None,
    recipient_address,  // The recipient's payment address
    NoteValue::from_raw(note_value), // Transfer full value (no fee from pool)
    memo_bytes,         // [u8; 36] structured memo
)?;

// 3. Build -> prove -> sign (needs SpendAuthorizingKey for the spend)
let pk = get_proving_key();
let mut rng = rand::rngs::OsRng;
let (unauthorized, _) = builder.build::<i64>(&mut rng)?.expect("bundle present");

let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
let sighash = compute_platform_sighash(&bundle_commitment, &[]); // No extra_data
let proven = unauthorized.create_proof(pk, &mut rng)?;
let bundle = proven.apply_signatures(rng, sighash, &[ask])?; // Spend auth key required

// 4. Serialize and create transition
let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
    serialize_authorized_bundle(&bundle);

let transition = ShieldedTransferTransition::try_from_bundle(
    actions,
    flags,
    value_balance as u64, // 0 for pure transfer, >0 if paying fees from pool
    anchor_bytes,
    proof_bytes,
    binding_sig,
    platform_version,
)?;
```

**Key points:**
- The `anchor` must match a historical anchor stored on-chain (not `Anchor::empty_tree()`)
- `value_balance` is 0 for a pure private transfer (all value stays in the pool)
- `value_balance` > 0 means that amount is extracted from the pool as a fee
- The `SpendAuthorizingKey` (`ask`) must be provided to `apply_signatures`

### 6.4 Unshield (Type 17)

Withdraws credits from the shielded pool to a transparent platform address. Spends a note and delivers part of the value to a transparent address.

```rust
use dpp::state_transition::state_transitions::shielded::unshield_transition::UnshieldTransition;

// 1. Get a spendable note
let (note, merkle_path, anchor) = wallet.take_spendable_note()?;
let note_value = note.value().inner();

// 2. Decide amounts
let unshield_amount = note_value / 2;   // Amount going to transparent address
let change_amount = note_value - unshield_amount; // Change staying in pool

// 3. Build bundle: spend note, output change back to self
let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
builder.add_spend(fvk.clone(), note, merkle_path)?;
builder.add_output(
    None,
    self_address, // Change goes back to our shielded address
    NoteValue::from_raw(change_amount),
    [0u8; 36],   // 36-byte structured memo
)?;

// 4. Build -> prove -> sign WITH extra_data binding
let pk = get_proving_key();
let mut rng = rand::rngs::OsRng;
let (unauthorized, _) = builder.build::<i64>(&mut rng)?.expect("bundle present");

let output_address = PlatformAddress::P2pkh(recipient_hash);
let mut extra_data = output_address.to_bytes();
extra_data.extend_from_slice(&unshield_amount.to_le_bytes());

let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
let sighash = compute_platform_sighash(&bundle_commitment, &extra_data);
let proven = unauthorized.create_proof(pk, &mut rng)?;
let bundle = proven.apply_signatures(rng, sighash, &[ask])?;

// 5. Serialize and create transition
let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
    serialize_authorized_bundle(&bundle);

let transition = UnshieldTransition::try_from_bundle(
    output_address,
    unshield_amount,
    actions,
    flags,
    value_balance,  // Positive (credits flow OUT of pool)
    anchor_bytes,
    proof_bytes,
    binding_sig,
    platform_version,
)?;
```

**Key points:**
- `value_balance` must be **positive** (credits flow out of the pool)
- `value_balance >= amount` (the difference is the fee paid from the pool)
- `output_address` and `amount` are bound to the sighash -- they cannot be modified after signing
- The `output_address` is a `PlatformAddress` (P2pkh or P2sh, not a Core address)

### 6.5 ShieldedWithdrawal (Type 19)

Withdraws credits from the shielded pool to a Dash Core L1 address. Similar to Unshield but targets a Core script instead of a platform address.

```rust
use dpp::state_transition::state_transitions::shielded::shielded_withdrawal_transition::ShieldedWithdrawalTransition;

// Bundle construction similar to Unshield, but with output_script in extra_data
let mut extra_data = output_script.to_bytes();
extra_data.extend_from_slice(&withdrawal_amount.to_le_bytes());

let sighash = compute_platform_sighash(&bundle_commitment, &extra_data);
// ... prove and sign as usual ...

let transition = ShieldedWithdrawalTransition::try_from_bundle(
    withdrawal_amount,
    actions,
    flags,
    value_balance,
    anchor_bytes,
    proof_bytes,
    binding_sig,
    core_fee_per_byte,  // Core transaction fee rate
    pooling,            // Pooling strategy (Never, Standard, etc.)
    output_script,      // Dash Core output script (e.g., P2PKH)
    platform_version,
)?;
```

---

## 7. Bundle Serialization

After building and signing an Orchard bundle, decompose it into the platform serialization format:

```rust
use dpp::shielded::SerializedAction;
use grovedb_commitment_tree::{Bundle, Authorized, DashMemo};

fn serialize_authorized_bundle(
    bundle: &Bundle<Authorized, i64, DashMemo>,
) -> (Vec<SerializedAction>, u8, i64, [u8; 32], Vec<u8>, [u8; 64]) {
    let actions: Vec<SerializedAction> = bundle.actions().iter().map(|action| {
        let enc = action.encrypted_note();
        let mut encrypted_note = Vec::with_capacity(216); // 32 + 104 + 80
        encrypted_note.extend_from_slice(&enc.epk_bytes);
        encrypted_note.extend_from_slice(enc.enc_ciphertext.as_ref());
        encrypted_note.extend_from_slice(&enc.out_ciphertext);

        SerializedAction {
            nullifier: action.nullifier().to_bytes(),
            rk: <[u8; 32]>::from(action.rk()),
            cmx: action.cmx().to_bytes(),
            encrypted_note,
            cv_net: action.cv_net().to_bytes(),
            spend_auth_sig: <[u8; 64]>::from(action.authorization()),
        }
    }).collect();

    let flags = bundle.flags().to_byte();
    let value_balance = *bundle.value_balance();
    let anchor = bundle.anchor().to_bytes();
    let proof = bundle.authorization().proof().as_ref().to_vec();
    let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());

    (actions, flags, value_balance, anchor, proof, binding_sig)
}
```

### SerializedAction Fields

| Field | Size | Description |
|-------|------|-------------|
| `nullifier` | 32 bytes | Unique tag preventing double-spends |
| `rk` | 32 bytes | Randomized spend validating key (RedPallas) |
| `cmx` | 32 bytes | Extracted note commitment for the new output |
| `encrypted_note` | 216 bytes | `epk` (32) + `enc_ciphertext` (104) + `out_ciphertext` (80) |
| `cv_net` | 32 bytes | Pedersen value commitment |
| `spend_auth_sig` | 64 bytes | RedPallas spend authorization signature |

---

## 8. Light Client Syncing

Light clients must synchronize with the on-chain shielded pool state. The protocol is specified in [DIP-0042](../dip/dip-0042.md). The key gRPC queries are:

### Available Queries

| Query | Returns | Purpose |
|-------|---------|---------|
| `GetShieldedPoolState` | Pool parameters, total balance, note count | Initial state check |
| `GetShieldedEncryptedNotes` | Encrypted notes by global position range | Note discovery via trial decryption |
| `GetShieldedAnchors` | Historical anchors by block height | Verify spend witnesses |
| `GetShieldedNullifiers` | Published nullifiers | Detect spent notes |

### GetShieldedEncryptedNotes

Notes are indexed by **global position** (a monotonically increasing `u64`). The request takes:

| Field | Type | Description |
|-------|------|-------------|
| `start_index` | `u64` | First global position to fetch (0-based) |
| `count` | `u32` | Maximum number of notes to return |
| `prove` | `bool` | Whether to return a GroveDB proof (V1) instead of raw data |

**Non-proved response** (`prove = false`): Returns a list of `EncryptedNote { cmx, encrypted_note }` for each position in the requested range. The response stops early if the position is past the end of the tree.

**Proved response** (`prove = true`): Returns a GroveDB **V1 proof** (supports BulkAppendTree subqueries). The client verifies using:

```rust
use grovedb::GroveDb;
use grovedb::VerifyOptions;

let (root_hash, result_set) = GroveDb::verify_query_with_options(
    &proof_bytes,
    &path_query,  // Same PathQuery structure as the server used
    VerifyOptions {
        absence_proofs_for_non_existing_searched_keys: false,
        verify_proof_succinctness: false,
        include_empty_trees_in_result: false,
    },
    grove_version,
)?;
```

V1 proofs authenticate BulkAppendTree entries by global position range. A single proof covers all requested positions efficiently (epoch blobs + buffer entries).

### Sync Flow

```
1. Query pool state to get current note count and latest block height
2. Determine the last synced position (wallet state)
3. Fetch notes in batches by position range:
   a. GetShieldedEncryptedNotes(start_index = last_synced + 1, count = batch_size)
   b. Trial-decrypt each note with IncomingViewingKey
   c. For decrypted notes: record (Note, Position, cmx) in wallet
   d. Append ALL cmx values to ClientCommitmentTree (Marked for ours, Ephemeral for others)
   e. Repeat until fewer than batch_size notes returned (caught up)
4. Checkpoint the ClientCommitmentTree at the current sync point
5. Query nullifiers to detect which of our notes have been spent
6. Remove spent notes from the spendable set
```

### Epoch-Based Bulk Syncing

The BulkAppendTree's epoch structure (epoch_size = 2048) enables efficient bulk syncing:

- **Completed epochs** (positions 0..2047, 2048..4095, ...) are immutable blobs that never change
- These can be served from CDN/cache without re-querying the state tree
- A client syncing from scratch can download completed epoch blobs in parallel
- Only the current (partial) buffer needs fresh queries from platform nodes

### Sync Strategies

| Strategy | Description | Best For |
|----------|-------------|----------|
| Sequential | Process every position in order | Simple implementation, full history |
| Warp Sync | Scan notes first, compute witnesses later | Fast initial sync (10-100x faster) |
| Spend-Before-Sync | Use server-provided witness for immediate spending | Spending before full sync completes |
| Epoch-Parallel | Download completed epoch blobs concurrently | Initial sync on fast connections |

---

## 9. Trial Decryption

Light clients discover their notes by attempting to decrypt every encrypted note on-chain. The Orchard protocol uses standard `try_note_decryption` from `zcash_note_encryption`, parameterized with `OrchardDomain<DashMemo>`.

### 9.1 Decrypting from Bundle Actions

When you have a full `Bundle` (e.g., from a state transition you submitted or received via P2P), use `try_note_decryption` directly on each action:

```rust
use dash_sdk::grovedb_commitment_tree::{
    OrchardDomain, DashMemo, IncomingViewingKey, Note, PaymentAddress,
    try_note_decryption, Bundle, Authorized, Action,
};

fn scan_bundle_for_owned_notes(
    ivk: &IncomingViewingKey,
    bundle: &Bundle<Authorized, i64, DashMemo>,
) -> Vec<(Note, PaymentAddress, [u8; 36])> {
    let mut found = Vec::new();
    for action in bundle.actions() {
        // OrchardDomain binds the decryption to this action's Rho (nullifier-derived)
        let domain = OrchardDomain::<DashMemo>::for_action(action);
        // Action implements ShieldedOutput<OrchardDomain<DashMemo>>,
        // so it can be passed directly to try_note_decryption
        if let Some((note, address, memo)) = try_note_decryption(&domain, ivk, action) {
            found.push((note, address, memo));
        }
    }
    found
}
```

### 9.2 Decrypting from RPC Encrypted Notes

When syncing from the `GetShieldedEncryptedNotes` RPC, each entry includes:
- `nullifier` (32 bytes) -- the nullifier from the action that created this note (needed for Rho derivation)
- `cmx` (32 bytes) -- the extracted note commitment
- `encrypted_note` (216 bytes) -- `epk (32) || enc_ciphertext (104) || out_ciphertext (80)`

The nullifier is essential because `OrchardDomain` uses `Rho::from_nf_old(nullifier)` to validate `RandomSeed` and construct the `Note` during decryption.

#### Compact Trial Decryption (Fast Scanning)

Compact decryption only uses the first 52 bytes of the enc_ciphertext (version + diversifier + value + rseed). It's faster for scanning but does not recover the memo:

```rust
use dash_sdk::grovedb_commitment_tree::{
    OrchardDomain, DashMemo, IncomingViewingKey, Note,
    CompactAction, try_compact_note_decryption,
    ExtractedNoteCommitment, Nullifier, EphemeralKeyBytes,
    COMPACT_NOTE_SIZE,
};

/// Attempt compact trial decryption on an entry from GetShieldedEncryptedNotes.
fn try_compact_decrypt(
    ivk: &IncomingViewingKey,
    nullifier_bytes: &[u8; 32],
    cmx_bytes: &[u8; 32],
    encrypted_note: &[u8],
) -> Option<Note> {
    let nf = Nullifier::from_bytes(nullifier_bytes).into()?;
    let cmx = ExtractedNoteCommitment::from_bytes(cmx_bytes).into()?;
    let epk_bytes: [u8; 32] = encrypted_note[0..32].try_into().ok()?;

    let enc_compact: [u8; COMPACT_NOTE_SIZE] =
        encrypted_note[32..32 + COMPACT_NOTE_SIZE].try_into().ok()?;

    let compact = CompactAction::from_parts(nf, cmx, EphemeralKeyBytes(epk_bytes), enc_compact);
    let domain = OrchardDomain::<DashMemo>::for_compact_action(&compact);
    let (note, _address) = try_compact_note_decryption(&domain, ivk, &compact)?;
    Some(note)
}
```

#### Full Sync Loop

```rust
// Fetch notes from the RPC
let response = client.get_shielded_encrypted_notes(start_index, count, false).await?;
for (pos, entry) in response.entries.iter().enumerate() {
    let position = start_index + pos as u64;
    let nf: [u8; 32] = entry.nullifier.as_slice().try_into()?;
    let cmx: [u8; 32] = entry.cmx.as_slice().try_into()?;

    if let Some(note) = try_compact_decrypt(&ivk, &nf, &cmx, &entry.encrypted_note) {
        // This note belongs to us -- mark position in commitment tree for future spending
        tree.mark_position(position);
        wallet.add_note(note, position);
    }

    // Always append the cmx to the commitment tree (even for non-owned notes)
    tree.append(cmx);
}
```

### 9.3 Integration with ClientCommitmentTree

After detecting an owned note via trial decryption, mark it in the `ClientCommitmentTree`:

```rust
// After successful decryption at position `pos`:
tree.mark_position(pos);
```

This ensures the tree retains the witness (Merkle path) for this note, enabling future spend proofs.

Trial decryption is the core privacy guarantee: the server cannot determine which notes belong to which client. The client downloads all encrypted notes and tests each one locally.

---

## 10. Fee Model

Fees vary by transition type:

| Transition | Fee Source | Calculation |
|------------|-----------|-------------|
| Shield | Platform address inputs | Standard fee model (deducted from input addresses) |
| ShieldFromAssetLock | Asset lock value | `asset_lock_value - shield_amount` |
| ShieldedTransfer | Shielded pool | `value_balance` (extracted from pool, can be 0) |
| Unshield | Shielded pool | `value_balance - amount` (extracted from pool) |
| ShieldedWithdrawal | Shielded pool | `value_balance - amount` (extracted from pool) |

For Shield, fees are deducted from the transparent platform address inputs using the standard fee model with `user_fee_increase` as a multiplier (0 = 100% of base fee, 1 = 101%, etc.). For ShieldFromAssetLock, the fee is `asset_lock_value - shield_amount`, validated against the minimum fee with `user_fee_increase` applied. For ShieldedTransfer, Unshield, and ShieldedWithdrawal, fees are cryptographically locked by the Orchard binding signature -- the client chooses the fee at bundle construction time by setting the `value_balance` appropriately.

---

## 11. Security Considerations

### Sighash Binding

The platform sighash cryptographically binds transparent fields to the Orchard proof. For Unshield and ShieldedWithdrawal, the `output_address`/`output_script` and `amount` are included in `extra_data`. If an attacker modifies these fields, the binding signature and spend authorization signatures will fail verification.

**Always compute the sighash correctly.** Using wrong `extra_data` will produce a valid-looking bundle that the platform will reject.

### Anchor Freshness

Spend-based transitions (ShieldedTransfer, Unshield, ShieldedWithdrawal) must reference a **historical anchor** stored on-chain. The platform rejects:
- `Anchor::empty_tree()` (all zeros) for spend transitions
- Anchors that don't match any recorded on-chain anchor

Build spend bundles using the anchor from your `ClientCommitmentTree`, which must be in sync with the on-chain state.

### Nullifier Uniqueness

Each nullifier can only be published once. If a client submits a transition containing a nullifier that already exists in the on-chain nullifier set, the transition will be rejected (double-spend prevention).

### Key Security

- Never transmit the `SpendingKey` or `SpendAuthorizingKey` over the network
- The `ProvingKey` and `VerifyingKey` are deterministic and public -- safe to share
- Diversified addresses (different `address_at` indices) are unlinkable to each other without the `FullViewingKey`

### Value Conservation

The Orchard binding signature mathematically guarantees that no credits are created or destroyed:

```
sum(input_values) = sum(output_values) + value_balance
```

The platform verifies this constraint via the binding signature without learning any individual values.
