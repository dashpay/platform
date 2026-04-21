//! FFI types + helpers for forwarding
//! [`IdentityChangeSet`](platform_wallet::IdentityChangeSet) and
//! [`IdentityKeysChangeSet`](platform_wallet::IdentityKeysChangeSet)
//! out of [`FFIPersister`](crate::persistence::FFIPersister) to Swift.
//!
//! The shape maps 1:1 onto Swift's `PersistentIdentity` +
//! `PersistentPublicKey` SwiftData models so the Swift handler can
//! apply each changeset as plain row upserts/removes.
//!
//! ## Ownership
//!
//! Both `IdentityEntryFFI` and `IdentityKeyEntryFFI` carry heap
//! allocations (C-strings for label / derivation path, byte buffers
//! for `public_key_data`). Each struct has a paired free helper that
//! releases every allocation it owns. The callback callers
//! ([`persistence.rs`](crate::persistence)) build temporary arrays of
//! these structs, fire the callback synchronously, then call the free
//! helper for every entry before returning — Swift must consume
//! whatever it needs to persist before returning from the callback.

use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

use platform_wallet::changeset::{IdentityEntry, IdentityKeyEntry};
use platform_wallet::wallet::identity::types::key_storage::PrivateKeyData;

// `IdentityStatus` discriminants are mirrored on the Swift side. Keep
// this encoding in sync with the `repr(u8)` order in
// `platform-wallet/src/wallet/identity/types/key_storage.rs`.
use platform_wallet::IdentityStatus;

/// Flat C mirror of [`IdentityEntry`]'s persistable scalars.
///
/// Public keys are NOT included here — they travel in
/// [`IdentityKeyEntryFFI`] alongside their private-key material via a
/// separate callback. Fields that don't map onto the Swift schema
/// (block times, DPNS names, DashPay profile/payments) are skipped;
/// DashPay overlays already ride on the dedicated
/// `dashpay_profiles` / `dashpay_payments_overlay` surfaces on the
/// parent changeset.
#[repr(C)]
pub struct IdentityEntryFFI {
    pub identity_id: [u8; 32],
    pub balance: u64,
    pub revision: u64,
    /// BIP-9 HD identity index. Included so Swift can reconstruct the
    /// derivation path on watch-only restore.
    pub identity_index: u32,
    /// User-visible label. Heap-owned, NUL-terminated UTF-8, or
    /// `null` when absent.
    pub label: *mut c_char,
    /// `IdentityStatus` discriminant (see DPP enum).
    pub status: u8,
    /// Set iff the identity carries a wallet id. Swift uses this to
    /// link `PersistentIdentity.walletId` back to `PersistentWallet`.
    pub wallet_id_is_some: bool,
    pub wallet_id: [u8; 32],
}

/// Private-key encoding discriminant on [`IdentityKeyEntryFFI`].
///
/// The Rust side's [`PrivateKeyData`] enum carries either raw 32-byte
/// key material (`Clear`) or a seed-derivation reference
/// (`AtWalletDerivationPath`). We expose both via a tag + the fields
/// that matter for each variant; ignore anything outside the variant's
/// column set when decoding.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivateKeyKindFFI {
    /// `has_private_key == false`. The `private_key_*` columns are
    /// unset; Swift should clear `privateKeyKeychainIdentifier`.
    None = 0,
    /// Raw 32-byte key material in `private_key_bytes`. Swift should
    /// store it in the Keychain and record the resulting identifier
    /// on `PersistentPublicKey.privateKeyKeychainIdentifier`.
    Clear = 1,
    /// Seed-derived key. `private_key_wallet_id` identifies the
    /// wallet and `private_key_derivation_path` is the BIP-32 path
    /// as a string (e.g. `m/9'/5'/...`). Swift can persist the path
    /// string (or re-derive lazily at signing time); no Keychain
    /// write is needed because the seed already lives in the Keychain
    /// at the wallet level.
    AtWalletDerivationPath = 2,
}

/// Flat C mirror of [`IdentityKeyEntry`] for forwarding across FFI.
///
/// `public_key_data_ptr` / `public_key_data_len` own a heap-allocated
/// copy of the public-key bytes (compressed secp256k1 for ECDSA, hash
/// for hash160, etc. — depends on `key_type`). Released by
/// [`free_identity_key_entry_ffi`].
#[repr(C)]
pub struct IdentityKeyEntryFFI {
    pub identity_id: [u8; 32],
    pub key_id: u32,

    // IdentityPublicKey mirror — mirrors the layout of the existing
    // `IdentityPublicKeyFFI` so Swift can share decoder logic if it
    // wants to; we keep this struct self-contained to avoid cross-file
    // coupling on the Swift side.
    pub purpose: u8,
    pub security_level: u8,
    pub key_type: u8,
    pub read_only: bool,
    pub disabled_at_is_some: bool,
    pub disabled_at: u64,
    pub public_key_data_ptr: *mut u8,
    pub public_key_data_len: usize,

    // Private-key payload. Layout mirrors [`PrivateKeyKindFFI`]:
    // - `None`: every `private_key_*` column is meaningless.
    // - `Clear`: `private_key_bytes` holds the raw 32 bytes.
    // - `AtWalletDerivationPath`: `private_key_wallet_id` + the
    //   NUL-terminated `private_key_derivation_path` C-string are
    //   populated. The path is heap-owned and released by
    //   `free_identity_key_entry_ffi`.
    pub private_key_kind: u8,
    pub private_key_bytes: [u8; 32],
    pub private_key_wallet_id: [u8; 32],
    pub private_key_derivation_path: *mut c_char,
}

/// Composite identifier for [`IdentityKeysChangeSet::removed`] entries
/// on the FFI boundary. A flat `[u8; 32]` + `u32` pair so Swift can
/// iterate an array directly without a secondary indirection.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IdentityKeyRemovalFFI {
    pub identity_id: [u8; 32],
    pub key_id: u32,
}

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

impl IdentityEntryFFI {
    /// Copy an [`IdentityEntry`] into a fresh FFI struct. The caller
    /// owns the allocated label C-string and must release it via
    /// [`free_identity_entry_ffi`].
    pub fn from_entry(entry: &IdentityEntry) -> Self {
        let label = entry
            .label
            .as_deref()
            .map(|s| {
                // Strings with interior NUL bytes become `null`;
                // profile / label fields should never contain NUL in
                // practice, and silently dropping is safer than
                // aborting the persist round.
                CString::new(s)
                    .map(|c| c.into_raw())
                    .unwrap_or(ptr::null_mut())
            })
            .unwrap_or(ptr::null_mut());

        let (wallet_id_is_some, wallet_id) = match entry.wallet_id {
            Some(id) => (true, id),
            None => (false, [0u8; 32]),
        };

        Self {
            identity_id: entry.id.to_buffer(),
            balance: entry.balance,
            revision: entry.revision,
            identity_index: entry.identity_index,
            label,
            status: status_discriminant(entry.status),
            wallet_id_is_some,
            wallet_id,
        }
    }
}

impl IdentityKeyEntryFFI {
    /// Copy an [`IdentityKeyEntry`] into a fresh FFI struct. The
    /// caller owns the heap-allocated `public_key_data_ptr` byte
    /// buffer and (when present) the `private_key_derivation_path`
    /// C-string; release both via [`free_identity_key_entry_ffi`].
    pub fn from_entry(entry: &IdentityKeyEntry) -> Self {
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;

        let pk_bytes = entry.public_key.data().as_slice().to_vec();
        let pk_len = pk_bytes.len();
        let pk_boxed = pk_bytes.into_boxed_slice();
        let public_key_data_ptr = Box::into_raw(pk_boxed) as *mut u8;

        let (disabled_some, disabled_at) = match entry.public_key.disabled_at() {
            Some(ts) => (true, ts),
            None => (false, 0u64),
        };

        // Decode the private-key variant. Always initialize the
        // column for the chosen variant; leave the rest zeroed so
        // the Swift decoder's switch on `private_key_kind` ignores
        // the unused fields.
        let mut private_key_kind = PrivateKeyKindFFI::None as u8;
        let mut private_key_bytes = [0u8; 32];
        let mut private_key_wallet_id = [0u8; 32];
        let mut private_key_derivation_path: *mut c_char = ptr::null_mut();

        if let Some(pk) = &entry.private_key {
            match pk {
                PrivateKeyData::Clear(zeroizing_bytes) => {
                    private_key_kind = PrivateKeyKindFFI::Clear as u8;
                    private_key_bytes.copy_from_slice(zeroizing_bytes.as_ref());
                }
                PrivateKeyData::AtWalletDerivationPath {
                    wallet_id,
                    derivation_path,
                } => {
                    private_key_kind = PrivateKeyKindFFI::AtWalletDerivationPath as u8;
                    private_key_wallet_id = *wallet_id;
                    // Paths never contain NUL; fall back to null on
                    // the impossible-in-practice failure.
                    private_key_derivation_path = CString::new(derivation_path.to_string())
                        .map(|c| c.into_raw())
                        .unwrap_or(ptr::null_mut());
                }
            }
        }

        Self {
            identity_id: entry.identity_id.to_buffer(),
            key_id: entry.key_id,
            purpose: entry.public_key.purpose() as u8,
            security_level: entry.public_key.security_level() as u8,
            key_type: entry.public_key.key_type() as u8,
            read_only: entry.public_key.read_only(),
            disabled_at_is_some: disabled_some,
            disabled_at,
            public_key_data_ptr,
            public_key_data_len: pk_len,
            private_key_kind,
            private_key_bytes,
            private_key_wallet_id,
            private_key_derivation_path,
        }
    }
}

/// Map an [`IdentityStatus`] onto its FFI discriminant. Hand-mapped so
/// the encoding stays stable even if the upstream enum grows new
/// variants (the default branch is the sentinel `0xFF = Unknown`).
fn status_discriminant(status: IdentityStatus) -> u8 {
    match status {
        IdentityStatus::Unknown => 0,
        IdentityStatus::PendingCreation => 1,
        IdentityStatus::Active => 2,
        IdentityStatus::FailedCreation => 3,
        IdentityStatus::NotFound => 4,
    }
}

// ---------------------------------------------------------------------------
// Destructors
// ---------------------------------------------------------------------------

/// Release heap allocations owned by an [`IdentityEntryFFI`] — the
/// label string. Safe to call on an entry with `label = null`.
///
/// # Safety
///
/// `entry` must have been produced by [`IdentityEntryFFI::from_entry`]
/// and not previously freed.
pub unsafe fn free_identity_entry_ffi(entry: &mut IdentityEntryFFI) {
    if !entry.label.is_null() {
        // Reclaim + drop the CString allocated by `CString::into_raw`.
        let _ = unsafe { CString::from_raw(entry.label) };
        entry.label = ptr::null_mut();
    }
}

/// Release heap allocations owned by an [`IdentityKeyEntryFFI`] —
/// the public-key data buffer and, when present, the derivation-path
/// string for the `AtWalletDerivationPath` variant.
///
/// # Safety
///
/// `entry` must have been produced by
/// [`IdentityKeyEntryFFI::from_entry`] and not previously freed.
pub unsafe fn free_identity_key_entry_ffi(entry: &mut IdentityKeyEntryFFI) {
    if !entry.public_key_data_ptr.is_null() && entry.public_key_data_len > 0 {
        // Reconstruct the boxed slice we created via `Box::into_raw`
        // on a `Box<[u8]>`. Using `Vec::from_raw_parts` would over-
        // allocate because the slice's capacity equals its length
        // post-`into_boxed_slice`.
        let slice = unsafe {
            std::slice::from_raw_parts_mut(entry.public_key_data_ptr, entry.public_key_data_len)
        };
        let _ = unsafe { Box::from_raw(slice as *mut [u8]) };
        entry.public_key_data_ptr = ptr::null_mut();
        entry.public_key_data_len = 0;
    }
    if !entry.private_key_derivation_path.is_null() {
        let _ = unsafe { CString::from_raw(entry.private_key_derivation_path) };
        entry.private_key_derivation_path = ptr::null_mut();
    }
    // Best-effort zero of the raw private key bytes on release.
    // This doesn't substitute for full `Zeroizing` coverage — the
    // Rust side keeps the original in `Zeroizing<[u8; 32]>` — but
    // it closes the copy we made for the callback window.
    for byte in entry.private_key_bytes.iter_mut() {
        *byte = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    use platform_wallet::changeset::{IdentityEntry, IdentityKeyEntry};
    use platform_wallet::wallet::identity::types::key_storage::PrivateKeyData;
    use zeroize::Zeroizing;

    #[test]
    fn test_identity_entry_ffi_round_trip() {
        let entry = IdentityEntry {
            id: Identifier::from([7u8; 32]),
            balance: 1_234_567,
            revision: 3,
            identity_index: 42,
            label: Some("Alice".to_string()),
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            dpns_names: Vec::new(),
            contested_dpns_names: Vec::new(),
            status: IdentityStatus::Active,
            wallet_id: Some([9u8; 32]),
            dashpay_profile: None,
            dashpay_payments: Default::default(),
        };
        let mut ffi = IdentityEntryFFI::from_entry(&entry);
        assert_eq!(ffi.identity_id, [7u8; 32]);
        assert_eq!(ffi.balance, 1_234_567);
        assert_eq!(ffi.revision, 3);
        assert_eq!(ffi.identity_index, 42);
        assert_eq!(ffi.status, 2); // Active
        assert!(ffi.wallet_id_is_some);
        assert_eq!(ffi.wallet_id, [9u8; 32]);
        assert!(!ffi.label.is_null());
        let label_str = unsafe { std::ffi::CStr::from_ptr(ffi.label).to_str().unwrap() };
        assert_eq!(label_str, "Alice");
        unsafe { free_identity_entry_ffi(&mut ffi) };
        assert!(ffi.label.is_null());
    }

    #[test]
    fn test_identity_entry_ffi_no_label_no_wallet() {
        let entry = IdentityEntry {
            id: Identifier::from([1u8; 32]),
            balance: 0,
            revision: 0,
            identity_index: 0,
            label: None,
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            dpns_names: Vec::new(),
            contested_dpns_names: Vec::new(),
            status: IdentityStatus::Unknown,
            wallet_id: None,
            dashpay_profile: None,
            dashpay_payments: Default::default(),
        };
        let mut ffi = IdentityEntryFFI::from_entry(&entry);
        assert!(ffi.label.is_null());
        assert!(!ffi.wallet_id_is_some);
        assert_eq!(ffi.wallet_id, [0u8; 32]);
        assert_eq!(ffi.status, 0); // Unknown
        unsafe { free_identity_entry_ffi(&mut ffi) };
    }

    #[test]
    fn test_identity_key_entry_ffi_clear() {
        let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 5,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0xAB; 33]),
            disabled_at: None,
        });
        let entry = IdentityKeyEntry {
            identity_id: Identifier::from([2u8; 32]),
            key_id: 5,
            public_key,
            private_key: Some(PrivateKeyData::Clear(Zeroizing::new([0xCD; 32]))),
        };
        let mut ffi = IdentityKeyEntryFFI::from_entry(&entry);
        assert_eq!(ffi.identity_id, [2u8; 32]);
        assert_eq!(ffi.key_id, 5);
        assert_eq!(ffi.purpose, Purpose::AUTHENTICATION as u8);
        assert_eq!(ffi.security_level, SecurityLevel::HIGH as u8);
        assert_eq!(ffi.key_type, KeyType::ECDSA_SECP256K1 as u8);
        assert!(!ffi.read_only);
        assert!(!ffi.disabled_at_is_some);
        assert_eq!(ffi.public_key_data_len, 33);
        let data_slice =
            unsafe { std::slice::from_raw_parts(ffi.public_key_data_ptr, ffi.public_key_data_len) };
        assert_eq!(data_slice, &[0xAB; 33]);
        assert_eq!(ffi.private_key_kind, PrivateKeyKindFFI::Clear as u8);
        assert_eq!(ffi.private_key_bytes, [0xCD; 32]);
        assert!(ffi.private_key_derivation_path.is_null());
        unsafe { free_identity_key_entry_ffi(&mut ffi) };
        assert!(ffi.public_key_data_ptr.is_null());
        assert_eq!(ffi.private_key_bytes, [0u8; 32]);
    }

    #[test]
    fn test_identity_key_entry_ffi_none_private_key() {
        let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::ENCRYPTION,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: true,
            data: BinaryData::new(vec![0x12; 20]),
            disabled_at: Some(1_700_000_000),
        });
        let entry = IdentityKeyEntry {
            identity_id: Identifier::from([3u8; 32]),
            key_id: 0,
            public_key,
            private_key: None,
        };
        let mut ffi = IdentityKeyEntryFFI::from_entry(&entry);
        assert_eq!(ffi.private_key_kind, PrivateKeyKindFFI::None as u8);
        assert!(ffi.read_only);
        assert!(ffi.disabled_at_is_some);
        assert_eq!(ffi.disabled_at, 1_700_000_000);
        unsafe { free_identity_key_entry_ffi(&mut ffi) };
    }
}
