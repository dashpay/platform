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

/// Curve discriminant for [`CoreAddressEntryFFI::key_type_tag`],
/// selecting which `key_wallet::managed_account::address_pool::PublicKeyType`
/// variant [`CoreAddressEntryFFI::public_key`] holds. Meaningful only
/// when `public_key_len > 0`. Kept stable across releases — it lands in
/// SwiftData rows next to the key bytes so a BLS operator or Ed25519
/// platform-node key survives the persist/restore round-trip that the
/// ECDSA-only 33-byte slot used to drop.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyTypeTagFFI {
    /// 33-byte compressed secp256k1 public key.
    ECDSA = 0,
    /// 48-byte BLS masternode-operator public key.
    BLS = 1,
    /// 32-byte Ed25519 platform-node public key.
    EdDSA = 2,
}

impl KeyTypeTagFFI {
    /// Validate a foreign byte into a `KeyTypeTagFFI` before use — reading
    /// an out-of-range value directly into the `repr(u8)` field would be UB.
    pub fn try_from_u8(b: u8) -> Option<Self> {
        Some(match b {
            0 => Self::ECDSA,
            1 => Self::BLS,
            2 => Self::EdDSA,
            _ => return None,
        })
    }
}

/// A single on-chain address entry.
///
/// `*const c_char` strings are Rust-owned and valid only for the
/// duration of the callback; Swift must copy them before returning.
#[repr(C)]
pub struct CoreAddressEntryFFI {
    /// Typed public key, left-aligned in a fixed 48-byte slot sized for
    /// the widest supported key (BLS-48). Exactly `public_key_len`
    /// leading bytes are meaningful; the remainder is zero-filled. The
    /// whole slot is zero when `public_key_len == 0` (no key).
    pub public_key: [u8; 48],
    /// Count of valid leading bytes in `public_key`: `0` = no key,
    /// `33` = ECDSA, `48` = BLS, `32` = EdDSA. The `(public_key_len,
    /// key_type_tag)` pair must agree with those widths; a mismatch is
    /// treated as "no key" on decode.
    pub public_key_len: u8,
    /// [`KeyTypeTagFFI`] raw value. Meaningful only when
    /// `public_key_len > 0`; ignored (and conventionally `0`) otherwise.
    pub key_type_tag: u8,
    /// `AddressPoolTypeTagFFI` raw value.
    pub pool_type_tag: u8,
    /// Derivation index within this pool.
    pub address_index: u32,
    /// Whether `AddressInfo.state` was `Used` at emit time.
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
