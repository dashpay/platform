//! FFI bindings for asset lock transaction building.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

/// Build an asset lock transaction.
///
/// On success:
/// - `out_tx_bytes`/`out_tx_len`: serialized signed transaction
/// - `out_private_key`: 32-byte one-time private key
///
/// Free tx bytes with `asset_lock_manager_free_tx_bytes`.
#[no_mangle]
pub unsafe extern "C" fn asset_lock_manager_build_transaction(
    handle: Handle,
    amount_duffs: u64,
    account_index: u32,
    funding_type: u32,
    identity_index: u32,
    out_tx_bytes: *mut *mut u8,
    out_tx_len: *mut usize,
    out_private_key: *mut [u8; 32],
) -> PlatformWalletFfiResult {
    check_ptr!(out_tx_bytes);
    check_ptr!(out_tx_len);
    check_ptr!(out_private_key);

    let option = parse_funding_type(funding_type);
    let funding = unwrap_option_or_return!(option);

    let option = ASSET_LOCK_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.build_asset_lock_transaction(
            amount_duffs,
            account_index,
            funding,
            identity_index,
        ))
    });
    let result = unwrap_option_or_return!(option);
    let (tx, key) = unwrap_result_or_return!(result);

    let serialized = dashcore::consensus::serialize(&tx);
    let len = serialized.len();
    let boxed = serialized.into_boxed_slice();

    *out_tx_bytes = Box::into_raw(boxed) as *mut u8;
    *out_tx_len = len;
    *out_private_key = key.inner.secret_bytes();

    PlatformWalletFfiResult::ok()
}

/// Build, broadcast, and wait for an asset lock proof.
///
/// On success:
/// - `out_proof_bytes`/`out_proof_len`: bincode-encoded AssetLockProof
/// - `out_private_key`: 32-byte one-time private key
/// - `out_txid`: 32-byte transaction ID
///
/// Free proof bytes with `asset_lock_manager_free_proof_bytes`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn asset_lock_manager_create_funded_proof(
    handle: Handle,
    amount_duffs: u64,
    account_index: u32,
    funding_type: u32,
    identity_index: u32,
    out_proof_bytes: *mut *mut u8,
    out_proof_len: *mut usize,
    out_private_key: *mut [u8; 32],
    out_txid: *mut [u8; 32],
) -> PlatformWalletFfiResult {
    check_ptr!(out_proof_bytes);
    check_ptr!(out_proof_len);
    check_ptr!(out_private_key);
    check_ptr!(out_txid);

    let funding = unwrap_option_or_return!(parse_funding_type(funding_type));

    let option = ASSET_LOCK_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.create_funded_asset_lock_proof(
            amount_duffs,
            account_index,
            funding,
            identity_index,
        ))
    });
    let result = unwrap_option_or_return!(option);
    let (proof, key, out_point) = unwrap_result_or_return!(result);

    let bytes = unwrap_result_or_return!(dpp::bincode::encode_to_vec(
        &proof,
        dpp::bincode::config::standard()
    ));

    let len = bytes.len();
    let boxed = bytes.into_boxed_slice();
    *out_proof_bytes = Box::into_raw(boxed) as *mut u8;
    *out_proof_len = len;
    *out_private_key = key.inner.secret_bytes();
    let mut txid_bytes = [0u8; 32];
    txid_bytes.copy_from_slice(&out_point.txid[..]);
    *out_txid = txid_bytes;

    PlatformWalletFfiResult::ok()
}

/// Free transaction bytes.
#[no_mangle]
pub unsafe extern "C" fn asset_lock_manager_free_tx_bytes(bytes: *mut u8, len: usize) {
    if !bytes.is_null() && len > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(bytes, len));
    }
}

/// Free proof bytes.
#[no_mangle]
pub unsafe extern "C" fn asset_lock_manager_free_proof_bytes(bytes: *mut u8, len: usize) {
    if !bytes.is_null() && len > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(bytes, len));
    }
}

/// Parse a u32 funding type tag into the Rust enum.
pub(super) fn parse_funding_type(
    value: u32,
) -> Option<key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType> {
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    match value {
        0 => Some(AssetLockFundingType::IdentityRegistration),
        1 => Some(AssetLockFundingType::IdentityTopUp),
        2 => Some(AssetLockFundingType::IdentityTopUpNotBound),
        3 => Some(AssetLockFundingType::IdentityInvitation),
        4 => Some(AssetLockFundingType::AssetLockAddressTopUp),
        5 => Some(AssetLockFundingType::AssetLockShieldedAddressTopUp),
        _ => None,
    }
}
