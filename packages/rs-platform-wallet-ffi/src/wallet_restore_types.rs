//! C-compatible types for external-signable wallet restore via the
//! load-side callbacks on
//! [`PersistenceCallbacks`](crate::persistence::PersistenceCallbacks).
//!
//! On write: `on_persist_account_registrations_fn` fires with the
//! `AccountSpecFFI` shape so Swift can store accounts in SwiftData.
//! On load: `on_load_wallet_list_fn` returns an array of
//! `WalletRestoreEntryFFI` which Rust assembles into an
//! external-signable `Wallet` via `Wallet::new_external_signable` +
//! per-account `Account::from_xpub`. (The mnemonic stays in the
//! host's keychain; signing routes back through the configured
//! signer surface. Earlier revisions reconstructed a `WatchOnly`
//! wallet — that path has been replaced.)
//!
//! All `*const u8` pointers must stay valid for the duration of the
//! load callback. Swift owns the allocation and is asked to free it
//! via the paired free callback.
//!
//! Xpubs are bincode-encoded (`RootExtendedPubKey` / `ExtendedPubKey`
//! use the `Encode` / `Decode` impls from the `bincode` feature of
//! `key-wallet`). Swift treats the bytes as opaque.
//!
//! The `AccountSpecFFI` struct is flat — it carries every field any
//! variant of `key_wallet::account::AccountType` might need. Fields
//! irrelevant to a given `type_tag` are ignored.

use std::os::raw::{c_char, c_void};

use crate::asset_lock_persistence::AssetLockEntryFFI;
use crate::platform_address_types::AddressBalanceEntryFFI;
use crate::types::FFINetwork;
use crate::wallet_registration_persistence::AccountAddressPoolFFI;

/// Discriminant for [`key_wallet::account::AccountType`].
///
/// Keep the integer values stable across releases — they end up in
/// SwiftData rows on the client. Carried across the FFI boundary as
/// a plain `u8` (see `AccountSpecFFI.type_tag`); validated via
/// [`AccountTypeTagFFI::try_from_u8`] before any `match`. Reading a
/// foreign `u8` directly into a `repr(u8)` enum field would be UB
/// for out-of-range values *before* the match runs.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountTypeTagFFI {
    Standard = 0,
    CoinJoin = 1,
    IdentityRegistration = 2,
    IdentityTopUp = 3,
    IdentityTopUpNotBoundToIdentity = 4,
    IdentityInvitation = 5,
    AssetLockAddressTopUp = 6,
    AssetLockShieldedAddressTopUp = 7,
    ProviderVotingKeys = 8,
    ProviderOwnerKeys = 9,
    ProviderOperatorKeys = 10,
    ProviderPlatformKeys = 11,
    DashpayReceivingFunds = 12,
    DashpayExternalAccount = 13,
    PlatformPayment = 14,
    /// DIP-13 per-identity ECDSA authentication key account.
    /// `index` carries the identity index; the master key lives at
    /// `.../identity_index'/0'` in the DIP-13 path.
    IdentityAuthenticationEcdsa = 15,
    /// DIP-13 per-identity BLS authentication key account. Shape
    /// mirrors `IdentityAuthenticationEcdsa`; the BLS-typed storage
    /// it maps into is gated on the `bls` feature on the Rust side.
    IdentityAuthenticationBls = 16,
}

impl AccountTypeTagFFI {
    /// Validating constructor for an FFI byte. Out-of-range bytes
    /// (corrupt SwiftData row, forward-versioned tag, malformed
    /// host buffer) return `None` so callers surface a recoverable
    /// validation error rather than triggering UB on an enum match.
    pub fn try_from_u8(b: u8) -> Option<Self> {
        Some(match b {
            0 => Self::Standard,
            1 => Self::CoinJoin,
            2 => Self::IdentityRegistration,
            3 => Self::IdentityTopUp,
            4 => Self::IdentityTopUpNotBoundToIdentity,
            5 => Self::IdentityInvitation,
            6 => Self::AssetLockAddressTopUp,
            7 => Self::AssetLockShieldedAddressTopUp,
            8 => Self::ProviderVotingKeys,
            9 => Self::ProviderOwnerKeys,
            10 => Self::ProviderOperatorKeys,
            11 => Self::ProviderPlatformKeys,
            12 => Self::DashpayReceivingFunds,
            13 => Self::DashpayExternalAccount,
            14 => Self::PlatformPayment,
            15 => Self::IdentityAuthenticationEcdsa,
            16 => Self::IdentityAuthenticationBls,
            _ => return None,
        })
    }
}

/// Discriminant for [`key_wallet::account::StandardAccountType`].
/// Only meaningful when the parent `type_tag` is
/// [`AccountTypeTagFFI::Standard`]. Same FFI-`u8`-with-validating-ctor
/// shape as `AccountTypeTagFFI` for the same UB-avoidance reason.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardAccountTypeTagFFI {
    Bip44 = 0,
    Bip32 = 1,
}

impl StandardAccountTypeTagFFI {
    pub fn try_from_u8(b: u8) -> Option<Self> {
        Some(match b {
            0 => Self::Bip44,
            1 => Self::Bip32,
            _ => return None,
        })
    }
}

/// Flat account spec carried in `WalletRestoreEntryFFI.accounts`.
///
/// Field relevance per `type_tag`:
///   * `Standard`                            — `standard_tag`, `index`
///   * `CoinJoin`                            — `index`
///   * `IdentityRegistration`                — (none)
///   * `IdentityTopUp`                       — `registration_index`
///   * `IdentityTopUpNotBoundToIdentity`     — (none)
///   * `IdentityInvitation`                  — (none)
///   * `AssetLockAddressTopUp`               — (none)
///   * `AssetLockShieldedAddressTopUp`       — (none)
///   * `ProviderVotingKeys`                  — (none)
///   * `ProviderOwnerKeys`                   — (none)
///   * `ProviderOperatorKeys`                — (none); `account_xpub_bytes`
///     carries a bincode-encoded extended **BLS** public key, not a
///     secp256k1 `ExtendedPubKey`
///   * `ProviderPlatformKeys`                — (none); `account_xpub_bytes`
///     carries a bincode-encoded extended **Ed25519** public key, not a
///     secp256k1 `ExtendedPubKey`
///   * `DashpayReceivingFunds`               — `index`, `user_identity_id`, `friend_identity_id`
///   * `DashpayExternalAccount`              — `index`, `user_identity_id`, `friend_identity_id`
///   * `PlatformPayment`                     — `index` (as `account`), `key_class`
///   * `IdentityAuthenticationEcdsa`         — `index` (as `identity_index`)
///   * `IdentityAuthenticationBls`           — `index` (as `identity_index`)
#[repr(C)]
pub struct AccountSpecFFI {
    /// Raw byte projection of [`AccountTypeTagFFI`]. Validated via
    /// [`AccountTypeTagFFI::try_from_u8`] on the Rust side before any
    /// `match` — reading a foreign byte directly into a `repr(u8)`
    /// enum field would be UB for out-of-range values.
    pub type_tag: u8,
    /// Raw byte projection of [`StandardAccountTypeTagFFI`]. Same
    /// validation pattern as `type_tag`.
    pub standard_tag: u8,
    pub index: u32,
    pub registration_index: u32,
    pub key_class: u32,
    pub user_identity_id: [u8; 32],
    pub friend_identity_id: [u8; 32],
    /// Bincode-encoded [`key_wallet::bip32::ExtendedPubKey`] for ECDSA
    /// accounts. For the two provider key-material accounts the bytes
    /// are instead a bincode-encoded extended BLS
    /// (`ProviderOperatorKeys`) or Ed25519 (`ProviderPlatformKeys`)
    /// public key — the `type_tag` selects the decode. Valid for
    /// callback duration only; Swift owns the allocation.
    pub account_xpub_bytes: *const u8,
    pub account_xpub_bytes_len: usize,
}

/// Per-identity public-key row carried on
/// [`IdentityRestoreEntryFFI::keys`].
///
/// Mirrors the persisted `PersistentPublicKey` columns Swift writes
/// during the `on_persist_identity_keys_fn` round. Carrying them on the
/// load path means each restored `Identity` enters the in-memory
/// `IdentityManager` with a populated `public_keys` `BTreeMap` instead
/// of an empty one — the original gap that left
/// `Identity::public_keys()` empty after cold-start until the next sync
/// round repopulated it.
///
/// Field discriminants match the DPP `repr(u8)` enum layouts (same
/// convention used by [`crate::identity_registration_with_signer::IdentityPubkeyFFI`]):
/// - `key_type`: [`dpp::identity::KeyType`] discriminant
///   (0 = ECDSA_SECP256K1, …).
/// - `purpose`: [`dpp::identity::Purpose`] discriminant
///   (0 = AUTHENTICATION, …).
/// - `security_level`: [`dpp::identity::SecurityLevel`] discriminant
///   (0 = MASTER, 1 = CRITICAL, 2 = HIGH, 3 = MEDIUM).
///
/// `data` is the public-key bytes (compressed secp256k1 → 33 bytes;
/// BLS → 48; etc.). The pointer is Swift-owned and valid only for the
/// duration of the load callback.
///
/// `contract_bounds_*` mirror the [`IdentityKeyEntryFFI`]
/// projection of DPP's `ContractBounds` enum (kind tag: 0=none,
/// 1=SingleContract, 2=SingleContractDocumentType). Including them
/// here closes the persist↔restore round-trip — without it, scoped
/// DashPay keys (registered with `SingleContractDocumentType`) come
/// back as unbounded on cold restart.
///
/// Disabled-at and other non-essential fields remain omitted —
/// they're either always `None` for newly derived identity-auth
/// keys or get re-populated by the next identity sync round.
#[repr(C)]
pub struct IdentityKeyRestoreFFI {
    pub key_id: u32,
    pub key_type: u8,
    pub purpose: u8,
    pub security_level: u8,
    pub read_only: bool,
    /// Public-key bytes (33 for ECDSA_SECP256K1; 48 for BLS; etc.).
    /// Valid for callback duration only; Swift owns the allocation.
    pub data: *const u8,
    pub data_len: usize,
    /// ContractBounds discriminant: 0=none, 1=SingleContract,
    /// 2=SingleContractDocumentType. Mirrors the encoding in
    /// [`crate::identity_persistence::IdentityKeyEntryFFI`].
    pub contract_bounds_kind: u8,
    /// 32-byte contract identifier. Zeroed when
    /// `contract_bounds_kind == 0`; otherwise the contract id the
    /// key is bound to.
    pub contract_bounds_id: [u8; 32],
    /// NUL-terminated UTF-8 doc-type name. Non-null iff
    /// `contract_bounds_kind == 2`. Swift-owned (released by the
    /// same load-callback allocation arena that frees the public-
    /// key data buffer).
    pub contract_bounds_document_type: *const c_char,
}

/// Per-identity entry attached to a [`WalletRestoreEntryFFI`].
///
/// Carries the scalar fields needed to rebuild a `ManagedIdentity`
/// inside the wallet's `IdentityManager` on startup. Bucket placement
/// is implicit: every identity carried on a `WalletRestoreEntryFFI`
/// belongs to the wallet that owns the entry (lands in
/// `wallet_identities[wallet_id][identity_index]`). Out-of-wallet /
/// observed identities don't ride on the wallet entry restore path —
/// they have no associated wallet, so there's no bucket to attach
/// them to (see report).
///
/// The DPP `Identity` is reconstructed from the scalars via the
/// `IdentityV0` shape on the Rust side — matching what
/// `IdentityManager::apply_identity_entry` does on the changeset
/// replay path — so no full `Identity` blob crosses the FFI.
///
/// Public keys ride along on `keys` / `keys_count` as
/// `IdentityKeyRestoreFFI` rows assembled from the per-identity
/// `PersistentPublicKey` rows on the Swift side. Each row is converted
/// into an `IdentityPublicKey::V0` and inserted into the
/// reconstructed `Identity.public_keys` map keyed by `key_id`. When
/// `keys_count == 0` the identity loads with an empty `public_keys`
/// map (e.g. an in-flight registration whose key persist round
/// hasn't completed); a subsequent sync round refreshes it.
///
/// All pointer fields (`dpns_names`, `contested_dpns_names`, `keys`)
/// are Swift-owned and valid only for the duration of the load
/// callback. The matching free callback releases them.
#[repr(C)]
pub struct IdentityRestoreEntryFFI {
    /// 32-byte identifier.
    pub identity_id: [u8; 32],
    /// Identity balance (credits).
    pub balance: u64,
    /// On-chain identity revision.
    pub revision: u64,
    /// HD identity index (`m/9'/coin'/5'/0'/ECDSA'/N'/...`).
    ///
    /// Always meaningful on this struct: every identity carried on a
    /// `WalletRestoreEntryFFI` is wallet-owned (lands in
    /// `wallet_identities[wallet_id][identity_index]`), so the
    /// derivation index is always known. Out-of-wallet identities don't
    /// ride on the wallet-restore path today (no SwiftData rows for
    /// them), so the optionality that exists on
    /// `ManagedIdentity.identity_index` doesn't surface here. If/when
    /// out-of-wallet identities start being persisted, they need their
    /// own restore array on the load callback rather than reusing this
    /// field with a sentinel.
    pub identity_index: u32,
    /// `IdentityStatus` discriminant — same encoding as
    /// `IdentityEntryFFI::status` (0 = Unknown, 1 = PendingCreation,
    /// 2 = Active, 3 = FailedCreation, 4 = NotFound).
    pub status: u8,
    /// Optional DPNS name labels owned by this identity, as a flat
    /// `*const *const c_char` array of UTF-8 c-strings. `null` when
    /// no names are cached.
    pub dpns_names: *const *const c_char,
    pub dpns_names_count: usize,
    /// Optional contested DPNS labels currently in voting period.
    /// Same array shape as `dpns_names`. `null` when none.
    pub contested_dpns_names: *const *const c_char,
    pub contested_dpns_names_count: usize,
    /// Identity public-key rows assembled from the per-identity
    /// `PersistentPublicKey` SwiftData rows. Each row is folded into
    /// the reconstructed `Identity.public_keys` map keyed by
    /// `key_id`. `null` / `0` when the identity has no persisted
    /// keys (e.g. an in-flight registration whose key-persist round
    /// hasn't completed).
    pub keys: *const IdentityKeyRestoreFFI,
    pub keys_count: usize,
    /// DashPay contact rows owned by this identity, assembled from the
    /// per-identity `PersistentDashpayContactRequest` SwiftData rows.
    /// Reuses the persist-side [`crate::contact_persistence::ContactRequestFFI`]
    /// shape (Swift-owned for the callback window — the byte buffers
    /// and metadata strings ride the load allocation, NOT the Rust
    /// destructors). Restores pending sent / incoming requests and
    /// established contacts (pairs of rows, both directions) with
    /// their owner-private metadata — without this, contacts only
    /// re-derive from chain on the first sync sweep and the
    /// contactInfo metadata is wiped during the deferred-publish
    /// window (the relaunch-durability gap in contact-info persistence).
    /// `null` / `0` when the identity has no persisted contact rows.
    pub contacts: *const crate::contact_persistence::ContactRequestFFI,
    pub contacts_count: usize,
    /// DashPay payment-history rows owned by this identity, assembled
    /// from the per-identity `PersistentDashpayPayment` SwiftData rows.
    /// Restores the `dashpay_payments` map at load — without this the
    /// in-memory map starts empty and only *Received* entries are
    /// re-derived from UTXOs by the reconcile sweep, so *Sent* entries
    /// (with their user-entered memos) silently vanish from the
    /// authoritative model on every relaunch (H1). Swift-owned for the
    /// callback window; the strings ride the load allocation, NOT the
    /// Rust destructors. `null` / `0` when the identity has no payments.
    pub payments: *const PaymentRestoreEntryFFI,
    pub payments_count: usize,
    /// DashPay ignored senders (per-sender mute, local-only) owned by this
    /// identity, assembled from the persisted ignored-sender rows. Restores
    /// the managed identity's ignored-senders set at load — **without this the ignore
    /// set starts empty on every relaunch, so the still-on-platform
    /// immutable `contactRequest` documents of a previously-ignored sender
    /// re-ingest on the next sync sweep and the ignored sender resurfaces**
    /// (the relaunch-durability gap that mirrors the contacts/payments
    /// restore arrays above). Each entry is a bare 32-byte sender id (the
    /// host persists only currently-ignored senders, so an un-ignored one
    /// simply doesn't appear) — a flat POD array, so nothing rides the load
    /// allocation here. `null` / `0` when the identity has ignored no one.
    pub ignored_senders: *const [u8; 32],
    pub ignored_senders_count: usize,
    /// DashPay cached **contact** profiles owned by this identity,
    /// assembled from the per-identity `PersistentDashpayContactProfile`
    /// SwiftData rows. Restores the managed identity's contact-profile cache
    /// (present entries only) at load — without this the contact-profile
    /// cache starts empty on every relaunch and the requests/contacts UI
    /// shows raw identity ids until the next profile sweep re-fetches
    /// every contact (write amplification + a visible cold-start flicker).
    /// Only **present** profiles are persisted/restored; the
    /// confirmed-absent negative cache rebuilds harmlessly on the next
    /// sweep. Swift-owned for the callback window; the strings ride the
    /// load allocation, NOT the Rust destructors. `null` / `0` when the
    /// identity has no cached contact profiles.
    pub contact_profiles: *const ContactProfileRestoreEntryFFI,
    pub contact_profiles_count: usize,
}

/// One DashPay payment-history row to rehydrate into
/// the managed identity's payments map (keyed by `txid`) at load.
///
/// `direction_raw` / `status_raw` mirror the `PaymentDirection` /
/// `PaymentStatus` discriminants (direction: 0=Sent, 1=Received;
/// status: 0=Pending, 1=Confirmed, 2=Failed). Swift owns `txid`
/// (always non-null) and the optional `memo` for the callback window.
#[repr(C)]
pub struct PaymentRestoreEntryFFI {
    /// NUL-terminated transaction id (hex) — the `dashpay_payments`
    /// map key.
    pub txid: *const std::os::raw::c_char,
    /// The other identity in this payment.
    pub counterparty_id: [u8; 32],
    /// Amount in duffs (always positive; `direction_raw` carries sign).
    pub amount_duffs: u64,
    /// `PaymentDirection` discriminant: 0=Sent, 1=Received.
    pub direction_raw: u8,
    /// `PaymentStatus` discriminant: 0=Pending, 1=Confirmed, 2=Failed.
    pub status_raw: u8,
    /// NUL-terminated memo, or null when the source `Option` was `None`.
    pub memo: *const std::os::raw::c_char,
}

/// One cached **contact** profile row to rehydrate into
/// the managed identity's contact-profile cache (keyed by the contact's identity
/// id) at load. Mirrors the persist-side
/// [`crate::identity_persistence::ContactProfileRowFFI`] field-for-field
/// (the leading `contact_id` key, the five public profile fields with
/// their `_present` byte-array flags, and the trailing `checked_at_ms`
/// self-heal timestamp).
///
/// Only **present** profiles ride this struct — the confirmed-absent
/// negative cache is never persisted, so every restored entry rebuilds
/// as `ContactProfileEntry { profile: Some(..), checked_at_ms }`. Swift
/// owns the four optional c-strings for the callback window; gate the
/// byte-array fields on their paired `_present` flag rather than
/// checking for all-zero (a valid hash/fingerprint value).
#[repr(C)]
pub struct ContactProfileRestoreEntryFFI {
    /// The contact's 32-byte identity id — the `contact_profiles` map
    /// key.
    pub contact_id: [u8; 32],
    /// NUL-terminated `displayName`, or null when the source `Option`
    /// was `None`.
    pub display_name: *const std::os::raw::c_char,
    /// NUL-terminated `bio`, or null when `None`.
    pub bio: *const std::os::raw::c_char,
    /// NUL-terminated `avatarUrl`, or null when `None`.
    pub avatar_url: *const std::os::raw::c_char,
    /// SHA-256 avatar hash; meaningful only when
    /// [`Self::avatar_hash_present`] is `true`.
    pub avatar_hash: [u8; 32],
    /// `true` iff the source `avatar_hash` was `Some(_)`.
    pub avatar_hash_present: bool,
    /// DHash avatar fingerprint; meaningful only when
    /// [`Self::avatar_fingerprint_present`] is `true`.
    pub avatar_fingerprint: [u8; 8],
    /// `true` iff the source `avatar_fingerprint` was `Some(_)`.
    pub avatar_fingerprint_present: bool,
    /// NUL-terminated `publicMessage`, or null when `None`.
    pub public_message: *const std::os::raw::c_char,
    /// Wall-clock ms of the last fetch attempt — the
    /// `ContactProfileEntry::checked_at_ms` self-heal timestamp.
    pub checked_at_ms: u64,
}

/// One unspent UTXO row to rehydrate into a funds-bearing account's
/// `ManagedCoreFundsAccount.utxos` map at startup.
///
/// The leading account-tag block is the same `(type_tag, standard_tag,
/// index, registration_index, key_class, user_identity_id,
/// friend_identity_id)` shape `AccountSpecFFI` uses, so the loader can
/// reuse `account_type_from_spec` for routing. Keys-only and
/// PlatformPayment variants are skipped on the receive side — they
/// don't carry UTXOs.
///
/// `script_pubkey` is a Swift-owned byte buffer; the address string is
/// reconstructed from `(script_pubkey, network)` on the Rust side, so
/// no C-string field is needed here.
#[repr(C)]
pub struct UtxoRestoreEntryFFI {
    /// Raw byte projection of [`AccountTypeTagFFI`]. Validated via
    /// [`AccountTypeTagFFI::try_from_u8`] on the Rust side. See
    /// `AccountSpecFFI.type_tag` for the UB-avoidance rationale.
    pub type_tag: u8,
    pub standard_tag: u8,
    pub account_index: u32,
    pub registration_index: u32,
    pub key_class: u32,
    pub user_identity_id: [u8; 32],
    pub friend_identity_id: [u8; 32],
    pub prev_txid: [u8; 32],
    pub vout: u32,
    pub value_duffs: u64,
    pub script_pubkey: *const u8,
    pub script_pubkey_len: usize,
    pub height: u32,
    pub is_coinbase: bool,
    pub is_confirmed: bool,
    pub is_instantlocked: bool,
    pub is_locked: bool,
}

/// One persisted transaction record carried back at load time so the
/// in-memory `transactions()` map can be selectively repopulated for
/// the small subset of records that matter for chain-lock cascade —
/// today, the funding transactions of tracked asset locks still at
/// `Built` / `Broadcast` (`statusRaw < 2`).
///
/// Why selectively rather than wholesale: the wallet's own load path
/// only bulk-restores UTXOs, not tx records, by design — most tx
/// history is consumed reactively through SwiftData `@Query`s, not
/// from the in-memory map. The exception is asset locks waiting for
/// IS-lock / chain-lock proofs: their funding tx must live in the
/// in-memory map at the moment the next chain-lock event fires, or
/// `WalletManager::apply_chain_lock` finds nothing to promote and
/// the bridge has no `chain_lock_promotions` to emit. Restoring
/// these specific records closes that gap without breaking the rest
/// of the lazy-load model.
///
/// `context_raw` matches `TransactionContext` discriminants:
/// 0 = Mempool, 1 = InstantSend, 2 = InBlock, 3 = InChainLockedBlock.
/// Only `2` and `3` are reconstructible from these scalar fields;
/// `0` / `1` need either no block info (Mempool) or an IS-lock blob
/// we don't carry (InstantSend), so the Rust load path treats them
/// as `Mempool` — defensive code for an edge that shouldn't occur in
/// practice (an asset lock at `Built` / `Broadcast` has by definition
/// not yet observed IS-lock or block confirmation).
#[repr(C)]
pub struct UnresolvedAssetLockTxRecordFFI {
    /// Family-independent source index the funding tx spent UTXOs
    /// from — the same `account_index` the Rust `TrackedAssetLock`
    /// carries. A pooled lock can be funded from BIP44, BIP32 or
    /// DashPay receiving accounts (CoinJoin backs only the drain
    /// flow), so load-time routing tries BIP44, then BIP32, then
    /// CoinJoin at this index, and finally falls back to a receival
    /// account (searched by txid, index-independent) — see
    /// `restore_unresolved_asset_lock_tx_records`.
    pub account_index: u32,
    /// Consensus-encoded asset-lock transaction body. Same wire
    /// format `dashcore::consensus::encode::serialize` produces, so
    /// `Transaction::consensus_decode` round-trips. Swift-owned for
    /// the callback window; freed by `LoadWalletListFreeFn`.
    pub tx_bytes: *mut u8,
    pub tx_bytes_len: usize,
    /// `TransactionContext` discriminant; see struct docstring for
    /// values. Anything other than `2` / `3` is treated as `Mempool`
    /// by the load path.
    pub context_raw: u32,
    /// Block height (meaningful only when `context_raw` is `2` or
    /// `3`; zero placeholder otherwise).
    pub block_height: u32,
    /// Block hash (wire-orientation 32 bytes; meaningful only when
    /// `context_raw` is `2` or `3`; zeros otherwise).
    pub block_hash: [u8; 32],
    /// Block timestamp (Unix seconds; same meaningfulness rule as
    /// `block_height`).
    pub block_timestamp: u64,
    /// Persisted "first seen" Unix-second timestamp. Carried so the
    /// rebuilt `TransactionRecord` mirrors what Swift had on disk;
    /// `wait_for_proof` itself reads only `context` + `height()`, so
    /// a zero here is benign for the chain-lock cascade path.
    pub first_seen: u64,
}

/// A persisted provider special transaction (ProRegTx / ProUpServTx /
/// ProUpRegTx / ProUpRevTx) staged back into the wallet at load so its
/// DIP-3 payload record is resident on the provider-key accounts again.
///
/// Without this, key-wallet's rust-dashcore #876 retention has nothing to
/// retain after a restart (the wallet is rebuilt from staging, which
/// otherwise stages only UTXOs + asset-lock funding txs), so the
/// masternode-list aggregation comes back empty until a rescan
/// re-processes the blocks.
///
/// Same raw-tx / height shape as [`UnresolvedAssetLockTxRecordFFI`] but
/// with NO `account_index`: provider involvement is payload-based (owner
/// / voting key hashes), not a known BIP44 index, so the load path routes
/// the record onto the wallet's provider-key accounts directly. `tx_bytes`
/// is Swift-owned for the callback window and freed by the load
/// allocation's `release()`.
#[repr(C)]
pub struct ProviderSpecialTxRestoreEntryFFI {
    /// Consensus-encoded transaction body (`Transaction::consensus_decode`
    /// round-trips). Carries the DIP-3 special-transaction payload.
    pub tx_bytes: *mut u8,
    pub tx_bytes_len: usize,
    /// `TransactionContext` discriminant: `2` = InBlock, `3` =
    /// InChainLockedBlock; anything else is treated as `Mempool`.
    pub context_raw: u32,
    /// Block height (meaningful only when `context_raw` is `2` / `3`).
    pub block_height: u32,
    /// Block hash (wire-orientation; meaningful with `context_raw` `2`/`3`).
    pub block_hash: [u8; 32],
    /// Block timestamp (Unix seconds; same meaningfulness rule).
    pub block_timestamp: u64,
    /// The transaction's index within its block (`block.vtx` order),
    /// meaningful only when `has_block_position`. Restored onto the
    /// rebuilt record's `BlockInfo` so the masternode aggregation keeps
    /// Core's same-block apply order across restarts (rust-dashcore#891).
    /// `false` for rows persisted before the field existed.
    pub block_position: u32,
    pub has_block_position: bool,
    /// Persisted "first seen" Unix-second timestamp (mirrors on-disk).
    pub first_seen: u64,
}

/// Per-wallet entry returned by `on_load_wallet_list_fn`.
///
/// `accounts` points to a contiguous array of length `accounts_count`.
/// Swift allocates it, the load callback stashes the pointer here, and
/// the matching free callback releases it after Rust is done reading.
#[repr(C)]
pub struct WalletRestoreEntryFFI {
    pub wallet_id: [u8; 32],
    /// Network this wallet was created on. Mirrors what was supplied to
    /// `platform_wallet_manager_create_wallet_from_seed`.
    pub network: FFINetwork,
    pub accounts: *const AccountSpecFFI,
    pub accounts_count: usize,
    /// Cached platform-address balances for this wallet. The pointer is
    /// Swift-owned and valid only for the duration of the callback.
    pub platform_address_balances: *const AddressBalanceEntryFFI,
    pub platform_address_balances_count: usize,
    /// Network-scoped incremental BLAST sync watermark, repeated on
    /// each wallet entry for that network during restore.
    pub platform_sync_height: u64,
    pub platform_sync_timestamp: u64,
    pub platform_last_known_recent_block: u64,
    /// Per-wallet identities to rehydrate into the wallet's
    /// `IdentityManager` (the `wallet_identities[wallet_id]` bucket).
    /// `null` / `0` when the wallet has no persisted identities.
    pub identities: *const IdentityRestoreEntryFFI,
    pub identities_count: usize,
    /// Core-chain sync metadata stamped onto the rebuilt
    /// `ManagedWalletInfo.metadata` at load time. Zero is treated as
    /// "unknown" — the snapshot leaves the field at its default in
    /// that case (which `from_wallet` already seeds from
    /// `birth_height - 1`). `last_synced` is Unix seconds.
    pub birth_height: u32,
    pub synced_height: u32,
    pub last_processed_height: u32,
    pub last_synced: u64,
    /// Persisted unspent UTXOs to repopulate funds-bearing accounts.
    /// Swift-owned, freed by `LoadWalletListFreeFn` — including each
    /// row's `script_pubkey` buffer.
    pub utxos: *const UtxoRestoreEntryFFI,
    pub utxos_count: usize,
    /// Tracked asset-lock entries persisted by the
    /// `on_persist_asset_locks_fn` callback that need to be
    /// rehydrated into `ClientWalletStartState.unused_asset_locks`
    /// so wallet load resumes mid-flight registrations.
    ///
    /// Each entry's `transaction_bytes` / `proof_bytes` buffers are
    /// Swift-owned and freed by `LoadWalletListFreeFn`. `null` / `0`
    /// when the wallet has no persisted tracked locks.
    pub tracked_asset_locks: *const AssetLockEntryFFI,
    pub tracked_asset_locks_count: usize,
    /// Funding tx records for tracked asset locks at `statusRaw < 2`
    /// (Built / Broadcast). The Rust load path re-inserts each entry
    /// into the matching `standard_bip44_accounts[account_index]
    /// .transactions_mut()` bucket so the next incoming chain-lock
    /// event can find these txids in the in-memory map and promote
    /// them via `apply_chain_lock` — closing the SPV-restart gap
    /// where an asset lock would otherwise stay stuck at `Broadcast`
    /// indefinitely because the wallet's `transactions()` started
    /// empty and no follow-up CLSig at a higher height was ever
    /// going to re-fire promotion for the lower-height block.
    ///
    /// Each entry's `tx_bytes` buffer is Swift-owned and freed by
    /// `LoadWalletListFreeFn`. `null` / `0` when the wallet has no
    /// unresolved asset locks.
    pub unresolved_asset_lock_tx_records: *const UnresolvedAssetLockTxRecordFFI,
    pub unresolved_asset_lock_tx_records_count: usize,
    /// Persisted provider special transactions (ProRegTx / ProUpServTx /
    /// ProUpRegTx / ProUpRevTx) re-staged onto the wallet's provider-key
    /// accounts so rust-dashcore #876 retention keeps them resident and
    /// the masternode-list aggregation survives a restart. `null` / `0`
    /// when the wallet has no provider special txs. Each entry's
    /// `tx_bytes` buffer is Swift-owned and freed by `LoadWalletListFreeFn`.
    pub provider_special_txs: *const ProviderSpecialTxRestoreEntryFFI,
    pub provider_special_txs_count: usize,
    /// Persisted core address pools for this wallet
    pub core_address_pools: *const AccountAddressPoolFFI,
    pub core_address_pools_count: usize,
    /// Bincode-serialised
    /// `dashcore::ephemerealdata::chain_lock::ChainLock`
    /// (`bincode::config::standard()`) carrying the persisted
    /// `WalletMetadata::last_applied_chain_lock` from the last
    /// session. `null` / `0` when no chainlock was ever persisted
    /// (fresh wallet, or wallet that hasn't observed a chainlock
    /// since the metadata-persist feature shipped). When present,
    /// `build_wallet_start_state` decodes and stamps it into
    /// `wallet_info.metadata.last_applied_chain_lock` before the
    /// wallet enters the manager — so the asset-lock-resume
    /// CL-from-metadata fallback in `proof.rs` can fire on
    /// catch-up tasks at app launch without waiting for SPV to
    /// re-apply a fresh chainlock.
    pub last_applied_chain_lock_bytes: *const u8,
    pub last_applied_chain_lock_bytes_len: usize,
}

// SAFETY: Pointers are Swift-owned and lifetime-scoped to the callback.
// Sending the struct across threads without being used is fine; any
// use must happen within the callback window.
unsafe impl Send for AccountSpecFFI {}
unsafe impl Sync for AccountSpecFFI {}
unsafe impl Send for IdentityKeyRestoreFFI {}
unsafe impl Sync for IdentityKeyRestoreFFI {}
unsafe impl Send for IdentityRestoreEntryFFI {}
unsafe impl Sync for IdentityRestoreEntryFFI {}
unsafe impl Send for WalletRestoreEntryFFI {}
unsafe impl Sync for WalletRestoreEntryFFI {}
unsafe impl Send for UtxoRestoreEntryFFI {}
unsafe impl Sync for UtxoRestoreEntryFFI {}
// SAFETY: `tx_bytes` is Swift-owned and lifetime-scoped to the load
// callback, same contract as the other restore entries above.
unsafe impl Send for ProviderSpecialTxRestoreEntryFFI {}
unsafe impl Sync for ProviderSpecialTxRestoreEntryFFI {}

/// Paired free callback for the wallet-list load callback. Releases
/// any memory Swift allocated for the entries array, the per-wallet
/// accounts arrays, the optional per-wallet platform-address balance
/// arrays, every xpub byte buffer, the per-wallet identity arrays,
/// every nested c-string + c-string pointer array carried by the
/// identity entries, every per-identity `IdentityKeyRestoreFFI`
/// array together with the public-key byte buffers each row points
/// at, and every per-wallet core-address-pool. Called exactly once
/// after a successful load.
pub type LoadWalletListFreeFn =
    unsafe extern "C" fn(context: *mut c_void, entries: *const WalletRestoreEntryFFI, count: usize);
