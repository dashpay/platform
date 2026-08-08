//! FFI types for forwarding
//! [`AssetLockChangeSet`](platform_wallet::changeset::AssetLockChangeSet)
//! out of [`FFIPersister`](crate::persistence::FFIPersister) to Swift.
//!
//! Mirrors the shape of the asset-lock changeset emitted by the
//! [`AssetLockManager`](platform_wallet::wallet::asset_lock::AssetLockManager)
//! at every status transition (Built → Broadcast → InstantSendLocked →
//! ChainLocked) and on consumption. Swift maps each upsert onto a
//! `PersistentAssetLock` row keyed by the outpoint and deletes rows
//! for each removed outpoint.
//!
//! ## Ownership
//!
//! [`AssetLockEntryFFI`] points at Rust-owned byte buffers for the
//! consensus-encoded transaction + the bincode-encoded proof. Both
//! live in [`AssetLockEntryStorage`] for the callback window only —
//! Swift must copy whatever bytes it needs before the callback
//! returns. The storage Vec is dropped right after the FFI call, which
//! releases the buffers.

use bincode::config;
use dashcore::consensus::Encodable;
use dpp::prelude::AssetLockProof;
use platform_wallet::changeset::AssetLockEntry;
use platform_wallet::AssetLockStatus;

/// Flat C mirror of one [`AssetLockEntry`].
///
/// The transaction is consensus-encoded; the optional proof is
/// bincode-encoded with `dpp::bincode::config::standard()`. Both byte
/// slices are Rust-owned for the lifetime of the callback and live in
/// the matching [`AssetLockEntryStorage`].
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AssetLockEntryFFI {
    /// Outpoint identifying this credit output: 32-byte txid (raw
    /// internal byte order) followed by 4-byte little-endian vout.
    /// Matches the serialization used by `PersistentTxo.outpoint`.
    pub out_point: [u8; 36],
    /// Consensus-encoded asset-lock transaction. Rust-owned, valid only
    /// for the callback window.
    pub transaction_bytes: *const u8,
    pub transaction_bytes_len: usize,
    /// BIP44 account index that funded this asset lock.
    pub account_index: u32,
    /// Discriminant of [`key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType`]:
    /// 0 = IdentityRegistration, 1 = IdentityTopUp, 2 = IdentityTopUpNotBound,
    /// 3 = IdentityInvitation, 4 = AssetLockAddressTopUp,
    /// 5 = AssetLockShieldedAddressTopUp.
    pub funding_type: u8,
    /// Identity index used during creation.
    pub identity_index: u32,
    /// Locked amount in duffs (1 DASH = 1e8 duffs).
    pub amount_duffs: u64,
    /// Discriminant of [`AssetLockStatus`]:
    /// 0 = Built, 1 = Broadcast, 2 = InstantSendLocked, 3 = ChainLocked.
    pub status: u8,
    /// Bincode-encoded [`AssetLockProof`] (standard config). Rust-owned,
    /// `null` + `0` length when the entry has no proof yet (statuses
    /// Built / Broadcast).
    pub proof_bytes: *const u8,
    pub proof_bytes_len: usize,
}

// SAFETY: All pointer fields are Rust-owned and lifetime-scoped to the
// FFI callback. Sending the struct itself is fine; the receiver must
// not retain pointers beyond the callback window.
unsafe impl Send for AssetLockEntryFFI {}
unsafe impl Sync for AssetLockEntryFFI {}

/// Owned byte buffers backing one [`AssetLockEntryFFI`]'s pointer
/// fields. Kept alive by the callback dispatcher for the callback
/// window via a `Vec<AssetLockEntryStorage>` parallel to the
/// `Vec<AssetLockEntryFFI>`.
pub struct AssetLockEntryStorage {
    pub transaction_bytes: Vec<u8>,
    pub proof_bytes: Option<Vec<u8>>,
}

/// Build a `(Vec<AssetLockEntryFFI>, Vec<AssetLockEntryStorage>)` pair
/// from the changeset entries. The storage Vec MUST live at least as
/// long as the FFI Vec.
pub fn build_asset_lock_entries(
    entries: &[&AssetLockEntry],
) -> (Vec<AssetLockEntryFFI>, Vec<AssetLockEntryStorage>) {
    let mut storage: Vec<AssetLockEntryStorage> = Vec::with_capacity(entries.len());
    let mut ffi: Vec<AssetLockEntryFFI> = Vec::with_capacity(entries.len());

    for entry in entries {
        let mut transaction_bytes: Vec<u8> = Vec::new();
        // `Transaction::consensus_encode` returns `io::Result<usize>` and
        // never fails when writing to a `Vec<u8>`; an Err here would
        // mean a logic bug in `dashcore`, not bad input.
        entry
            .transaction
            .consensus_encode(&mut transaction_bytes)
            .expect("consensus_encode to Vec is infallible");

        let proof_bytes: Option<Vec<u8>> = entry.proof.as_ref().map(|proof| {
            dpp::bincode::encode_to_vec::<&AssetLockProof, _>(proof, config::standard())
                .expect("bincode encoding AssetLockProof is infallible")
        });

        let funding_type = funding_type_to_u8(entry.funding_type);
        let status = status_to_u8(&entry.status);

        storage.push(AssetLockEntryStorage {
            transaction_bytes,
            proof_bytes,
        });

        // Compute the pointers AFTER the storage push, then build the
        // FFI entry referencing the just-pushed storage slot. Two
        // Vecs grow independently so we cannot mix order — borrow the
        // storage slot via index right after the push.
        let slot = storage.last().expect("just pushed");
        let (proof_ptr, proof_len) = match &slot.proof_bytes {
            Some(bytes) => (bytes.as_ptr(), bytes.len()),
            None => (std::ptr::null(), 0usize),
        };

        ffi.push(AssetLockEntryFFI {
            out_point: outpoint_to_bytes(&entry.out_point),
            transaction_bytes: slot.transaction_bytes.as_ptr(),
            transaction_bytes_len: slot.transaction_bytes.len(),
            account_index: entry.account_index,
            funding_type,
            identity_index: entry.identity_index,
            amount_duffs: entry.amount_duffs,
            status,
            proof_bytes: proof_ptr,
            proof_bytes_len: proof_len,
        });
    }

    (ffi, storage)
}

/// Encode an [`OutPoint`](dashcore::OutPoint) as 36 bytes: 32-byte raw
/// txid followed by 4-byte little-endian vout. Matches the encoding
/// used by `PersistentTxo.outpoint`.
pub fn outpoint_to_bytes(outpoint: &dashcore::OutPoint) -> [u8; 36] {
    let mut bytes = [0u8; 36];
    bytes[..32].copy_from_slice(outpoint.txid.as_ref());
    bytes[32..].copy_from_slice(&outpoint.vout.to_le_bytes());
    bytes
}

fn funding_type_to_u8(
    funding_type: key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType,
) -> u8 {
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    match funding_type {
        AssetLockFundingType::IdentityRegistration => 0,
        AssetLockFundingType::IdentityTopUp => 1,
        AssetLockFundingType::IdentityTopUpNotBound => 2,
        AssetLockFundingType::IdentityInvitation => 3,
        AssetLockFundingType::AssetLockAddressTopUp => 4,
        AssetLockFundingType::AssetLockShieldedAddressTopUp => 5,
    }
}

fn status_to_u8(status: &AssetLockStatus) -> u8 {
    match status {
        AssetLockStatus::Built => 0,
        AssetLockStatus::Broadcast => 1,
        AssetLockStatus::InstantSendLocked => 2,
        AssetLockStatus::ChainLocked => 3,
        AssetLockStatus::Consumed => 4,
        AssetLockStatus::RecoveredFromChain => 5,
    }
}
