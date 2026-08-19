//! FFI bindings for asset lock sync/resume operations.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use platform_wallet::PlatformWalletError;
use std::ffi::CString;
use std::os::raw::c_char;
use std::time::Duration;

/// Build an `OutPoint` from a 32-byte raw txid pointer and a vout.
///
/// **FFI invariant:** the `txid` parameter is typed `*const [u8; 32]`,
/// not `*const u8`. The `.expect("txid is 32 bytes")` below is sound
/// *only* because the type system pins the length. If a future caller
/// weakens the signature to `*const u8 + usize_len`, this panic
/// becomes reachable across the FFI boundary and must be replaced
/// with a fallible return. Don't relax the signature without
/// hardening the body.
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
/// - `out_derivation_path`: NUL-terminated C string with the
///   credit-output derivation path (free with
///   `platform_wallet_string_free`).
///
/// Free proof bytes with `asset_lock_manager_free_proof_bytes`.
///
/// Unlike `asset_lock_manager_create_funded_proof`, this entry point
/// does **not** take a core signer handle — the resume path only
/// re-derives the proof and the credit-output derivation path from
/// the already-tracked lock state; signing the consume transition is
/// the next stage's responsibility (e.g.
/// [`crate::platform_wallet_register_identity_with_funding_signer`]).
#[no_mangle]
pub unsafe extern "C" fn asset_lock_manager_resume(
    handle: Handle,
    txid: *const [u8; 32],
    vout: u32,
    timeout_secs: u64,
    out_proof_bytes: *mut *mut u8,
    out_proof_len: *mut usize,
    out_derivation_path: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(txid);
    check_ptr!(out_proof_bytes);
    check_ptr!(out_proof_len);
    check_ptr!(out_derivation_path);

    let out_point = parse_outpoint(txid, vout);
    // `timeout_secs == 0` requests an unbounded wait (a ChainLock is
    // guaranteed finality; a broadcast lock is pending, never failed).
    let timeout = (timeout_secs != 0).then(|| Duration::from_secs(timeout_secs));

    let option = ASSET_LOCK_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.resume_asset_lock(&out_point, timeout))
    });
    let result = unwrap_option_or_return!(option);
    let (proof, path) = unwrap_result_or_return!(result);
    let bytes = unwrap_result_or_return!(dpp::bincode::encode_to_vec(
        &proof,
        dpp::bincode::config::standard()
    ));
    let len = bytes.len();
    let boxed = bytes.into_boxed_slice();
    *out_proof_bytes = Box::into_raw(boxed) as *mut u8;
    *out_proof_len = len;
    let path_c = unwrap_result_or_return!(CString::new(path.to_string()));
    *out_derivation_path = path_c.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Fire-and-forget variant of [`asset_lock_manager_resume`] for the
/// app-launch / app-foreground catch-up flow.
///
/// Calls `resume_asset_lock` internally and drops the returned
/// `(proof, derivation_path)` tuple — the chain-lock cascade that
/// `wait_for_proof` drives also queues an `AssetLockChangeSet` via
/// `advance_asset_lock_status` that writes `statusRaw = 3 +
/// proofBytes` back to SwiftData, which is the only payload the UI
/// needs. There's no proof or path to plumb out, so the FFI surface
/// stays free of out-params (and the matching free callbacks).
///
/// Returns `ok` on a successful proof resolution, an error on
/// timeout / wait failure. The Swift caller is expected to schedule
/// this on a background queue — `runtime().block_on(...)` parks the
/// calling thread for up to `timeout_secs` (or **indefinitely** when
/// `timeout_secs == 0`, since a ChainLock is guaranteed finality).
#[no_mangle]
pub unsafe extern "C" fn asset_lock_manager_catch_up_blocking(
    handle: Handle,
    txid: *const [u8; 32],
    vout: u32,
    timeout_secs: u64,
) -> PlatformWalletFFIResult {
    check_ptr!(txid);

    let out_point = parse_outpoint(txid, vout);
    // `timeout_secs == 0` requests an unbounded wait (a ChainLock is
    // guaranteed finality; a broadcast lock is pending, never failed).
    let timeout = (timeout_secs != 0).then(|| Duration::from_secs(timeout_secs));

    tracing::info!(
        outpoint = %out_point,
        timeout_secs,
        "asset_lock_manager_catch_up_blocking: entered"
    );

    let option = ASSET_LOCK_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.resume_asset_lock(&out_point, timeout))
    });
    let result = match option {
        Some(r) => r,
        None => {
            tracing::warn!(
                outpoint = %out_point,
                "asset_lock_manager_catch_up_blocking: invalid manager handle"
            );
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidHandle,
                "Manager handle invalid".to_string(),
            );
        }
    };
    match result {
        Ok(_) => {
            tracing::info!(
                outpoint = %out_point,
                "asset_lock_manager_catch_up_blocking: resume_asset_lock succeeded"
            );
            PlatformWalletFFIResult::ok()
        }
        Err(e) => {
            tracing::warn!(
                outpoint = %out_point,
                error = %e,
                "asset_lock_manager_catch_up_blocking: resume_asset_lock failed"
            );
            match e {
                // Double-spend verdicts route through the typed conversion
                // so the host receives the real code: terminal
                // ErrorAssetLockInputConflict (42) — the one code that
                // authorises discarding a tracked lock — or the
                // provisional ErrorAssetLockInputContested (43), which
                // stops the wait but keeps the lock for a later retry.
                // Flattening either to ErrorWalletOperation would leave
                // the host with a spinner it can never resolve.
                conflict @ (PlatformWalletError::AssetLockInputConflict { .. }
                | PlatformWalletError::AssetLockInputContested { .. }) => conflict.into(),
                other => PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorWalletOperation,
                    format!("{}", other),
                ),
            }
        }
    }
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
) -> PlatformWalletFFIResult {
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
    PlatformWalletFFIResult::ok()
}
