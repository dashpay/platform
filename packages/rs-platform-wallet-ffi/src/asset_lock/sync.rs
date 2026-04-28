//! FFI bindings for asset lock sync/resume operations.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
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
) -> PlatformWalletFfiResult {
    check_ptr!(txid);
    check_ptr!(out_proof_bytes);
    check_ptr!(out_proof_len);
    check_ptr!(out_private_key);

    let out_point = parse_outpoint(txid, vout);
    let timeout = Duration::from_secs(timeout_secs);

    let option = ASSET_LOCK_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.resume_asset_lock(&out_point, timeout))
    });
    let result = unwrap_option_or_return!(option);
    let (proof, key) = unwrap_result_or_return!(result);
    let bytes = unwrap_result_or_return!(dpp::bincode::encode_to_vec(
        &proof,
        dpp::bincode::config::standard()
    ));
    let len = bytes.len();
    let boxed = bytes.into_boxed_slice();
    *out_proof_bytes = Box::into_raw(boxed) as *mut u8;
    *out_proof_len = len;
    *out_private_key = key.inner.secret_bytes();
    PlatformWalletFfiResult::ok()
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
) -> PlatformWalletFfiResult {
    check_ptr!(tx_bytes);
    check_ptr!(txid);

    // Parse transaction
    let tx_data = std::slice::from_raw_parts(tx_bytes, tx_bytes_len);
    let tx: dashcore::Transaction =
        unwrap_result_or_return!(dashcore::consensus::deserialize(tx_data));

    let funding = unwrap_option_or_return!(super::build::parse_funding_type(funding_type));

    let out_point = parse_outpoint(txid, vout);

    // Parse optional proof
    let proof = if !proof_bytes.is_null() && proof_len > 0 {
        let data = std::slice::from_raw_parts(proof_bytes, proof_len);
        let (p, _) = unwrap_result_or_return!(dpp::bincode::decode_from_slice(
            data,
            dpp::bincode::config::standard()
        ));
        Some(p)
    } else {
        None
    };

    let option = ASSET_LOCK_MANAGER_STORAGE.with_item(handle, |manager| {
        manager.recover_asset_lock_blocking(
            tx,
            amount_duffs,
            account_index,
            funding,
            identity_index,
            out_point,
            proof,
        );
    });
    unwrap_option_or_return!(option);
    PlatformWalletFfiResult::ok()
}
