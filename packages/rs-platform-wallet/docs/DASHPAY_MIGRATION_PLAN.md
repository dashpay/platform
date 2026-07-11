# DashPay Migration Plan: Evo-Tool → Platform-Wallet

## Goal

Move ALL DashPay logic from evo-tool into platform-wallet's `DashPayWallet`.
Evo-tool becomes a thin UI layer that triggers operations and displays results.
No DashPay state flows from evo-tool into platform-wallet.

## Current State

### What DashPayWallet already owns (platform-wallet)
- Contact request send (ECDH key exchange, encrypted xpub, broadcast)
- Contact request sync (fetch received requests from Platform, auto-establish)
- Contact request accept/reject
- Contact account registration (DIP-14 address derivation, key-wallet account)
- Incoming payment address matching (DashpayReceivingFunds account pools)
- DIP-14/15 xpub derivation, account reference calculation
- Auto-accept proof generation/verification
- Encryption/decryption helpers (AES-256-CBC account labels)

### What evo-tool still owns (needs to move)

#### Profile Operations
| Operation | Evo-tool file | SDK calls | Moves to |
|-----------|--------------|-----------|----------|
| Load own profile | profile.rs:25-132 | Document::fetch_many (profile by $ownerId) | DashPayWallet::sync() |
| Create profile | profile.rs:134-374 (create path) | sdk.document_create + DocumentCreateTransitionBuilder | DashPayWallet::create_profile() |
| Update profile | profile.rs:134-374 (update path) | sdk.document_replace + DocumentReplaceTransitionBuilder | DashPayWallet::update_profile() |
| Fetch contact profile | profile.rs:432-459 | Document::fetch_many | DashPayWallet::sync() or on-demand |
| Search profiles | profile.rs:461-566 | DPNS query + profile fetch per identity | DashPayWallet::search_profiles() |
| Avatar processing | profile.rs:199-226 | SHA-256 hash + DHash fingerprint | Move to DashPayWallet (pure compute) |

#### Payment Operations
| Operation | Evo-tool file | SDK calls | Moves to |
|-----------|--------------|-----------|----------|
| Send payment to contact | payments.rs | derive_contact_payment_address + CoreWallet send | DashPayWallet::send_payment() |
| Record sent payment | payments.rs:334-340 via cache_payment | None (local) | Inside send_payment() |
| Record received payment | transaction_processing.rs:81-90 | None (local) | Inside match_incoming_dashpay_address() |
| Load payment history | payments.rs (local DB query) | None | Stays in evo-tool (UI reads from persister DB) |

#### Contact Operations (partially moved)
| Operation | Evo-tool file | Status |
|-----------|--------------|--------|
| Send contact request | contact_requests.rs | Already delegates to DashPayWallet |
| Accept contact request | contact_requests.rs | Already delegates to DashPayWallet |
| Reject contact request | contact_requests.rs | Already delegates to DashPayWallet |
| Load contacts (enriched) | contacts.rs | Decrypt contactInfo, fetch profiles — needs DashPayWallet |
| Load contact requests | contact_requests.rs | Query Platform — needs DashPayWallet |
| ContactInfo create/update | contact_info.rs | Encrypt + broadcast — needs DashPayWallet |

## DashPay Contract (DIP-15)

Three document types:

### `profile` (one per identity, mutable)
- `displayName` (string, max 25)
- `publicMessage` (string, max 140)
- `avatarUrl` (uri, max 2048)
- `avatarHash` (32-byte SHA-256)
- `avatarFingerprint` (8-byte dHash)
- Indexed by `$ownerId` (unique)
- Requires `$createdAt`, `$updatedAt`

### `contactRequest` (immutable, cannot be deleted)
- `toUserId` (32-byte Identifier)
- `encryptedPublicKey` (96 bytes: 16-byte IV + AES-256-CBC encrypted xpub)
- `senderKeyIndex`, `recipientKeyIndex` (u32)
- `accountReference` (u32, DIP-15 formula)
- `encryptedAccountLabel` (optional, 48-80 bytes)
- `autoAcceptProof` (optional, 38-102 bytes)
- Requires identity encryption/decryption bounded keys

### `contactInfo` (private metadata about contacts)
- `encToUserId` (32 bytes, encrypted)
- `rootEncryptionKeyIndex`, `derivationEncryptionKeyIndex`
- `privateData` (CBOR-encoded encrypted: aliasName, note, displayHidden, accounts)

Contract loaded via: `dpp::system_data_contracts::load_system_data_contract(SystemDataContract::Dashpay, PlatformVersion::latest())`

## SPV Integration

Payment address derivation path (DIP-15):
`m/9'/coin'/15'/account'/(sender_id)/(recipient_id)` → BIP32 non-hardened `/index` → P2PKH

Flow: SPV block → key-wallet `check_core_transaction` advances address pools →
`received_transaction_finality()` matches output via `try_match_incoming_dashpay_address()` →
record PaymentEntry on ManagedIdentity.

Key insight: key-wallet already manages DashPay address pools via `DashpayReceivingFunds`
account type. SPV detection is pre-computed during `register_contact_account()`.

## New DashPayWallet Public API

### Sync (replaces evo-tool's load/fetch operations)
```rust
/// Comprehensive DashPay sync: contact requests + profiles for all
/// managed identities. Call on wallet open and periodic refresh.
pub async fn sync(&self) -> Result<DashPaySyncResult, PlatformWalletError>
```
Internally:
1. Call existing `sync_contact_requests()` for each identity
2. Fetch profile documents for each identity (query by $ownerId)
3. Parse into DashPayProfile, cache on ManagedIdentity via set_dashpay_profile()
4. Fetch contact profiles for established contacts
5. Return summary (new contacts, profile updates, etc.)

### Data Models

```rust
/// Input for profile create/update. Only caller-provided fields.
/// Platform-wallet computes avatar_hash + avatar_fingerprint from
/// avatar_bytes internally, then drops the bytes.
pub struct ProfileUpdate {
    pub display_name: Option<String>,
    pub public_message: Option<String>,
    pub avatar_url: Option<String>,
    /// Raw image bytes pre-downloaded by the app layer (evo-tool).
    /// Platform-wallet computes SHA-256 hash + DHash fingerprint from
    /// these, includes them in the document, then drops the bytes.
    /// `None` = no avatar / remove avatar.
    pub avatar_bytes: Option<Vec<u8>>,
}

/// Persisted/displayed profile. Output of sync/create/update.
/// No raw bytes — only the computed hashes survive after processing.
pub struct DashPayProfile {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub avatar_hash: Option<[u8; 32]>,       // SHA-256 of image bytes
    pub avatar_fingerprint: Option<[u8; 8]>,  // DHash perceptual hash
    pub public_message: Option<String>,
}
```

NOTE: `avatar_bytes` is removed from `DashPayProfile` — it was transient
(only needed during document creation) and shouldn't be persisted or
carried in memory. Evo-tool's `dashpay_profiles` table column
`avatar_bytes` can be kept for local UI caching but the profile model
doesn't carry it.

### Profile Mutations (replace evo-tool's create/update)
```rust
/// Create a new DashPay profile on Platform.
/// Computes avatar hash/fingerprint from input.avatar_bytes if present.
/// Builds DocumentCreateTransition, broadcasts, caches result internally.
pub async fn create_profile(
    &self,
    identity_id: &Identifier,
    input: ProfileUpdate,
) -> Result<DashPayProfile, PlatformWalletError>

/// Update an existing DashPay profile on Platform.
/// Fetches current revision, bumps, broadcasts DocumentReplaceTransition.
/// Computes avatar hash/fingerprint from input.avatar_bytes if present.
pub async fn update_profile(
    &self,
    identity_id: &Identifier,
    input: ProfileUpdate,
) -> Result<DashPayProfile, PlatformWalletError>
```

### Search (replace evo-tool's DPNS + profile search)
```rust
/// Search for DashPay profiles by DPNS username prefix.
/// Queries DPNS contract, then fetches profiles for matching identities.
pub async fn search_profiles(
    &self,
    prefix: &str,
) -> Result<Vec<(Identifier, String, Option<DashPayProfile>)>, PlatformWalletError>
```
Needs DPNS contract: `load_system_data_contract(SystemDataContract::DPNS, ...)`

### Payments (replace evo-tool's send + receive paths)
```rust
/// Send a Core payment to a DashPay contact.
/// Resolves contact's receiving address from DIP-14 derivation,
/// builds and broadcasts Core tx via CoreWallet, records PaymentEntry.
pub async fn send_payment(
    &self,
    from_identity_id: &Identifier,
    to_contact_id: &Identifier,
    amount_duffs: u64,
    memo: Option<String>,
) -> Result<PaymentEntry, PlatformWalletError>
```

For received payments: extend `match_incoming_dashpay_address` variants
to also call `record_dashpay_payment()` internally when a match is found.
The caller passes `(txid, value)` and the method handles everything.

### ContactInfo (replace evo-tool's contact_info.rs)
```rust
/// Create or update encrypted contactInfo document on Platform.
pub async fn update_contact_info(
    &self,
    identity_id: &Identifier,
    contact_id: &Identifier,
    alias: Option<String>,
    note: Option<String>,
    hidden: bool,
) -> Result<(), PlatformWalletError>
```

### Load Contacts (replace evo-tool's contacts.rs enriched loading)
```rust
/// Load all established contacts with their profiles and DPNS names.
/// Decrypts contactInfo, fetches profiles for each contact.
pub async fn load_contacts(
    &self,
    identity_id: &Identifier,
) -> Result<Vec<EnrichedContact>, PlatformWalletError>
```

## Migration Sequence

### Phase 1: Profile Sync + Mutations (~200 LOC)
1. Add `fetch_profiles_for_identities()` internal helper to DashPayWallet
2. Add `sync()` that combines contact request sync + profile sync
3. Add `create_profile()` and `update_profile()` with avatar processing
4. Evo-tool profile.rs: delegate to DashPayWallet, remove Platform queries
5. Delete cache_profile from platform_wallet_cache.rs

### Phase 2: Payment Recording (~100 LOC)
1. Extend `try_match_incoming_dashpay_address` to accept txid+value and record internally
2. Add `send_payment()` to DashPayWallet
3. Evo-tool: payments.rs delegates send, transaction_processing.rs simplified
4. Delete cache_payment + cache_payment_with_pw_blocking

### Phase 3: Contact Enrichment + ContactInfo (~200 LOC)
1. Add `load_contacts()` with profile + DPNS enrichment
2. Add `update_contact_info()` for encrypted contact metadata
3. Add `search_profiles()` with DPNS integration
4. Evo-tool contacts.rs + contact_info.rs: delegate entirely

### Phase 4: Cleanup (~-400 LOC from evo-tool)
1. Delete `platform_wallet_cache.rs`
2. Simplify profile.rs, payments.rs, contacts.rs to thin delegation
3. Make `set_dashpay_profile` and `record_dashpay_payment` on ManagedIdentity `pub(crate)`
4. Remove DashPay contract from AppContext (platform-wallet loads it internally)

## Open Questions

1. **Avatar processing**: RESOLVED. Evo-tool (app layer) downloads avatar bytes via HTTP
   and passes them in `DashPayProfile.avatar_bytes`. Platform-wallet computes SHA-256 hash +
   DHash fingerprint from those bytes — it knows DIP-15 requires them. Hash/fingerprint
   functions move from evo-tool's `avatar_processing.rs` to platform-wallet. HTTP fetch
   (`fetch_image_bytes`) stays in evo-tool. No reqwest dependency in platform-wallet.

2. **State transition options**: evo-tool passes `app_context.state_transition_options()` for
   fee multiplier etc. Platform-wallet needs equivalent configuration.

3. **Load payment history**: Currently reads from evo-tool's persister DB directly.
   This should stay in evo-tool (UI reads persisted data) — no Platform query needed.

4. **DPNS contract for search**: search_profiles needs both DashPay + DPNS contracts.
   Both available via `load_system_data_contract`.
