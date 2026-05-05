//! C-compatible types for Core (on-chain) address pool persistence.
//!
//! `on_persist_account_address_pools_fn` fires when a wallet's
//! on-chain address pool changes — initial population on wallet
//! create, pool extension after `next_unused`, and per-address
//! `used` flips when SPV sees activity. Each
//! `AccountAddressPoolFFI` entry in the round carries a slice of
//! these per-address rows. Swift persists each entry into
//! SwiftData (`PersistentCoreAddress`) so the Storage Explorer
//! can render derivation paths + pubkeys reactively via `@Query`.

use std::os::raw::c_char;

/// Pool-type discriminant matching `key_wallet::managed_account::AddressPoolType`.
/// Kept stable across releases — it lands in SwiftData rows.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressPoolTypeTagFFI {
    External = 0,
    Internal = 1,
    Absent = 2,
    AbsentHardened = 3,
}

/// A single on-chain address entry.
///
/// `*const c_char` strings are Rust-owned and valid only for the
/// duration of the callback; Swift must copy them before returning.
#[repr(C)]
pub struct CoreAddressEntryFFI {
    /// 33-byte compressed secp256k1 public key. Valid iff
    /// `has_public_key == true`; zero-filled otherwise. Watch-only
    /// reconstructed accounts currently always carry the pubkey.
    pub public_key: [u8; 33],
    pub has_public_key: bool,
    /// `AddressPoolTypeTagFFI` raw value.
    pub pool_type_tag: u8,
    /// Derivation index within this pool.
    pub address_index: u32,
    /// `AddressInfo.used` at emit time.
    pub is_used: bool,
    /// Cached balance in duffs from `AddressInfo.balance`.
    pub balance: u64,
    /// Base58check-encoded address (NUL-terminated). Caller-scoped.
    pub address_base58: *const c_char,
    /// BIP32 derivation path (NUL-terminated). Caller-scoped.
    pub derivation_path: *const c_char,
}

// SAFETY: strings are Rust-owned for the callback window; the struct
// itself carries no state that outlives the call.
unsafe impl Send for CoreAddressEntryFFI {}
unsafe impl Sync for CoreAddressEntryFFI {}
