//! FFI bindings for asset lock sync/resume operations.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use std::time::Duration;

fn parse_outpoint(txid: *const [u8; 32], vout: u32) -> dashcore::OutPoint {
    use dashcore::hashes::Hash;
    let txid_bytes = unsafe { *txid };
    dashcore::OutPoint {
        txid: dashcore::Txid::from_slice(&txid_bytes).expect("txid is 32 bytes"),
        vout,
    }
}

/// Resume a tracked asset lock from whatever stage it's at.
///
/// On success:
/// - `out_proof_bytes`/`out_proof_len`: bincode-encoded AssetLockProof
/// - `out_private_key`: 32-byte one-time private key
///
/// Free proof bytes with `asset_lock_manager_free_proof_bytes`.
#[no_mangle]
pub unsafe extern "C" fn asset_lock_manager_resume(
    handle: Handle,
    txid: *const [u8; 32],
    vout: u32,
    timeout_secs: u64,
    out_proof_bytes: *mut *mut u8,
    out_proof_len: *mut usize,
    out_private_key: *mut [u8; 32],
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if txid.is_null()
        || out_proof_bytes.is_null()
        || out_proof_len.is_null()
        || out_private_key.is_null()
    {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let out_point = parse_outpoint(txid, vout);
    let timeout = Duration::from_secs(timeout_secs);

    ASSET_LOCK_MANAGER_STORAGE
        .with_item(handle, |manager| {
            match runtime().block_on(manager.resume_asset_lock(&out_point, timeout)) {
                Ok((proof, key)) => {
                    match dpp::bincode::encode_to_vec(&proof, dpp::bincode::config::standard()) {
                        Ok(bytes) => {
                            let len = bytes.len();
                            let boxed = bytes.into_boxed_slice();
                            *out_proof_bytes = Box::into_raw(boxed) as *mut u8;
                            *out_proof_len = len;
                            *out_private_key = key.inner.secret_bytes();
                            PlatformWalletFFIResult::Success
                        }
                        Err(e) => {
                            if !out_error.is_null() {
                                *out_error = PlatformWalletFFIError::new(
                                    PlatformWalletFFIResult::ErrorSerialization,
                                    format!("Failed to serialize proof: {}", e),
                                );
                            }
                            PlatformWalletFFIResult::ErrorSerialization
                        }
                    }
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            e.to_string(),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Recover a tracked asset lock from a serialized transaction.
///
/// Re-tracks the asset lock in memory so it can be resumed later.
/// The transaction must be a valid asset lock transaction with a
/// special transaction payload.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn asset_lock_manager_recover(
    handle: Handle,
    tx_bytes: *const u8,
    tx_bytes_len: usize,
    amount_duffs: u64,
    account_index: u32,
    funding_type: u32,
    identity_index: u32,
    txid: *const [u8; 32],
    vout: u32,
    proof_bytes: *const u8,
    proof_len: usize,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if tx_bytes.is_null() || txid.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    // Parse transaction
    let tx_data = std::slice::from_raw_parts(tx_bytes, tx_bytes_len);
    let tx: dashcore::Transaction = match dashcore::consensus::deserialize(tx_data) {
        Ok(t) => t,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorDeserialization,
                    format!("Failed to deserialize transaction: {}", e),
                );
            }
            return PlatformWalletFFIResult::ErrorDeserialization;
        }
    };

    let funding = match super::build::parse_funding_type(funding_type) {
        Some(f) => f,
        None => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!("Unknown funding type: {}", funding_type),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };

    let out_point = parse_outpoint(txid, vout);

    // Parse optional proof
    let proof = if !proof_bytes.is_null() && proof_len > 0 {
        let data = std::slice::from_raw_parts(proof_bytes, proof_len);
        match dpp::bincode::decode_from_slice(data, dpp::bincode::config::standard()) {
            Ok((p, _)) => Some(p),
            Err(e) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorDeserialization,
                        format!("Failed to deserialize proof: {}", e),
                    );
                }
                return PlatformWalletFFIResult::ErrorDeserialization;
            }
        }
    } else {
        None
    };

    ASSET_LOCK_MANAGER_STORAGE
        .with_item(handle, |manager| {
            manager.recover_asset_lock_blocking(
                tx,
                amount_duffs,
                account_index,
                funding,
                identity_index,
                out_point,
                proof,
            );
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}
