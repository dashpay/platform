//! C-compatible types for watch-only wallet restore via the load-side
//! callbacks on [`PersistenceCallbacks`](crate::persistence::PersistenceCallbacks).
//!
//! On write: `on_persist_wallet_root_xpub_fn` and `on_persist_account_fn`
//! fire with these shapes so Swift can store them in SwiftData.
//! On load: `on_load_wallet_list_fn` returns an array of
//! `WalletRestoreEntryFFI` which Rust assembles into a watch-only
//! `Wallet` via `Wallet::from_xpub` + per-account `Account::from_xpub`.
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

use std::os::raw::c_void;

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
}

// SAFETY: Pointers are Swift-owned and lifetime-scoped to the callback.
// Sending the struct across threads without being used is fine; any
// use must happen within the callback window.
unsafe impl Send for AccountSpecFFI {}
unsafe impl Sync for AccountSpecFFI {}
unsafe impl Send for WalletRestoreEntryFFI {}
unsafe impl Sync for WalletRestoreEntryFFI {}

/// Function-pointer type for the load callback.
///
/// Implementations must set `*out_entries` to a Swift-allocated array
/// of `WalletRestoreEntryFFI` and `*out_count` to the length. The
/// allocation is freed by the caller via `LoadWalletListFreeFn` once
/// Rust has consumed it.
///
/// Returns 0 on success, non-zero on failure. On failure Rust does not
/// call the free callback.
pub type LoadWalletListFn = unsafe extern "C" fn(
    context: *mut c_void,
    out_entries: *mut *const WalletRestoreEntryFFI,
    out_count: *mut usize,
) -> i32;

/// Paired free callback for `LoadWalletListFn`. Releases any memory
/// Swift allocated for the entries array, the per-wallet accounts
/// arrays, and every xpub byte buffer. Called exactly once after a
/// successful `LoadWalletListFn` invocation.
pub type LoadWalletListFreeFn =
    unsafe extern "C" fn(context: *mut c_void, entries: *const WalletRestoreEntryFFI, count: usize);

/// Fires once per account when it's added to a wallet. Swift should
/// upsert into `PersistentAccount` keyed by `(wallet_id, type_tag,
/// index, ...)`.
///
/// `spec.account_xpub_bytes` is valid only for the duration of the
/// callback.
pub type PersistAccountFn = unsafe extern "C" fn(
    context: *mut c_void,
    wallet_id: *const u8,
    spec: *const AccountSpecFFI,
) -> i32;

/// Fires once per wallet at registration time carrying Swift-visible
/// metadata that isn't derivable from the xpub: network tag + birth
/// height.
///
/// `network` uses the same discriminant as
/// [`WalletRestoreEntryFFI.network`] (0 = Mainnet, 1 = Testnet,
/// 2 = Devnet, 3 = Regtest).
///
/// `birth_height` is the best estimate of the block height at which
/// the wallet started. Zero means "scan from genesis / unknown";
/// callers can overwrite with a better value when SPV discovers the
/// chain tip.
pub type PersistWalletMetadataFn = unsafe extern "C" fn(
    context: *mut c_void,
    wallet_id: *const u8,
    network: u8,
    birth_height: u32,
) -> i32;
