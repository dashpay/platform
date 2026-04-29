//! C-compatible types for watch-only wallet restore via the load-side
//! callbacks on [`PersistenceCallbacks`](crate::persistence::PersistenceCallbacks).
//!
//! On write: `on_persist_account_registrations_fn` fires with the
//! `AccountSpecFFI` shape so Swift can store accounts in SwiftData.
//! On load: `on_load_wallet_list_fn` returns an array of
//! `WalletRestoreEntryFFI` which Rust assembles into a watch-only
//! `Wallet` via `Wallet::new_watch_only` + per-account `Account::from_xpub`.
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

use crate::platform_address_types::AddressBalanceEntryFFI;

/// Discriminant for [`key_wallet::account::AccountType`].
///
/// Keep the integer values stable across releases — they end up in
/// SwiftData rows on the client.
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

/// Discriminant for [`key_wallet::account::StandardAccountType`].
/// Only meaningful when `AccountSpecFFI.type_tag == AccountTypeTagFFI::Standard`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardAccountTypeTagFFI {
    Bip44 = 0,
    Bip32 = 1,
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
///   * `ProviderOperatorKeys`                — (none)
///   * `ProviderPlatformKeys`                — (none)
///   * `DashpayReceivingFunds`               — `index`, `user_identity_id`, `friend_identity_id`
///   * `DashpayExternalAccount`              — `index`, `user_identity_id`, `friend_identity_id`
///   * `PlatformPayment`                     — `index` (as `account`), `key_class`
///   * `IdentityAuthenticationEcdsa`         — `index` (as `identity_index`)
///   * `IdentityAuthenticationBls`           — `index` (as `identity_index`)
#[repr(C)]
pub struct AccountSpecFFI {
    pub type_tag: AccountTypeTagFFI,
    pub standard_tag: StandardAccountTypeTagFFI,
    pub index: u32,
    pub registration_index: u32,
    pub key_class: u32,
    pub user_identity_id: [u8; 32],
    pub friend_identity_id: [u8; 32],
    /// Bincode-encoded [`key_wallet::bip32::ExtendedPubKey`]. Valid for
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
/// Disabled-at, contract-bounds and other non-essential fields are
/// intentionally omitted — they're either always `None` for newly
/// derived identity-auth keys or get re-populated by the next
/// identity sync round if they exist on chain. The scope of this
/// restore is narrowly "make `Identity.public_keys` non-empty so
/// auth-key gates pass".
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
}

/// Per-wallet entry returned by `on_load_wallet_list_fn`.
///
/// `accounts` points to a contiguous array of length `accounts_count`.
/// Swift allocates it, the load callback stashes the pointer here, and
/// the matching free callback releases it after Rust is done reading.
#[repr(C)]
pub struct WalletRestoreEntryFFI {
    pub wallet_id: [u8; 32],
    /// [`key_wallet::Network`] discriminant, matching
    /// `platform_wallet_manager_create_wallet_from_seed`:
    /// 0 = Mainnet, 1 = Testnet, 2 = Devnet, 3 = Regtest.
    pub network: u8,
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

/// Paired free callback for the wallet-list load callback. Releases
/// any memory Swift allocated for the entries array, the per-wallet
/// accounts arrays, the optional per-wallet platform-address balance
/// arrays, every xpub byte buffer, the per-wallet identity arrays,
/// every nested c-string + c-string pointer array carried by the
/// identity entries, and every per-identity `IdentityKeyRestoreFFI`
/// array together with the public-key byte buffers each row points
/// at. Called exactly once after a successful load.
pub type LoadWalletListFreeFn =
    unsafe extern "C" fn(context: *mut c_void, entries: *const WalletRestoreEntryFFI, count: usize);
