//! FFI bindings for asset lock transaction building.
//!
//! These entry points are signer-driven: the asset-lock build path
//! never sees a raw credit-output private key. Instead the caller
//! supplies a [`MnemonicResolverHandle`] (the same vtable used by
//! `dash_sdk_sign_with_mnemonic_resolver_and_path`); the FFI wraps
//! it in a [`MnemonicResolverCoreSigner`] for the lifetime of the
//! call and returns the credit-output derivation path as a C
//! string. Callers later hand the path back to the same resolver
//! when consuming the asset lock on Platform.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use rs_sdk_ffi::MnemonicResolverCoreSigner;
use rs_sdk_ffi::MnemonicResolverHandle;
use std::ffi::CString;
use std::os::raw::c_char;

/// Build an asset lock transaction via an external mnemonic resolver.
///
/// On success:
/// - `out_tx_bytes`/`out_tx_len`: serialized signed transaction
/// - `out_derivation_path`: NUL-terminated C string with the
///   credit-output derivation path (e.g. `m/9'/1'/5'/0'/0'/3'/0'`).
///   Free with `platform_wallet_string_free`.
///
/// Free tx bytes with `asset_lock_manager_free_tx_bytes`.
///
/// # Safety
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle` produced by
///   [`crate::dash_sdk_mnemonic_resolver_create`]. The caller retains
///   ownership; this function does NOT destroy it.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn asset_lock_manager_build_transaction(
    handle: Handle,
    amount_duffs: u64,
    account_index: u32,
    funding_type: u32,
    identity_index: u32,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_tx_bytes: *mut *mut u8,
    out_tx_len: *mut usize,
    out_derivation_path: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(core_signer_handle);
    check_ptr!(out_tx_bytes);
    check_ptr!(out_tx_len);
    check_ptr!(out_derivation_path);

    let option = parse_funding_type(funding_type);
    let funding = unwrap_option_or_return!(option);

    let signer_addr = core_signer_handle as usize;

    let option = ASSET_LOCK_MANAGER_STORAGE.with_item(handle, |manager| {
        let wallet_id = manager.wallet_id();
        let network = manager.network();
        // SAFETY: `signer_addr` came from `core_signer_handle` which
        // the caller pinned alive for this call (see fn-level safety
        // doc). The `MnemonicResolverCoreSigner` lives only on this
        // stack frame and is dropped before the function returns.
        let signer = unsafe {
            MnemonicResolverCoreSigner::new(
                signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        runtime().block_on(manager.build_asset_lock_transaction(
            amount_duffs,
            account_index,
            funding,
            identity_index,
            &signer,
        ))
    });
    let result = unwrap_option_or_return!(option);
    let (tx, path) = unwrap_result_or_return!(result);

    let serialized = dashcore::consensus::serialize(&tx);
    let len = serialized.len();
    let boxed = serialized.into_boxed_slice();

    *out_tx_bytes = Box::into_raw(boxed) as *mut u8;
    *out_tx_len = len;
    let path_c = unwrap_result_or_return!(CString::new(path.to_string()));
    *out_derivation_path = path_c.into_raw();

    PlatformWalletFFIResult::ok()
}

/// Build, broadcast, and wait for an asset lock proof via an
/// external mnemonic resolver.
///
/// On success:
/// - `out_proof_bytes`/`out_proof_len`: bincode-encoded AssetLockProof
/// - `out_derivation_path`: NUL-terminated C string with the
///   credit-output derivation path. Free with
///   `platform_wallet_string_free`.
/// - `out_txid`: 32-byte transaction ID
///
/// Free proof bytes with `asset_lock_manager_free_proof_bytes`.
///
/// # Safety
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle`. Ownership is retained by the
///   caller.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn asset_lock_manager_create_funded_proof(
    handle: Handle,
    amount_duffs: u64,
    account_index: u32,
    funding_type: u32,
    identity_index: u32,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_proof_bytes: *mut *mut u8,
    out_proof_len: *mut usize,
    out_derivation_path: *mut *mut c_char,
    out_txid: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    check_ptr!(core_signer_handle);
    check_ptr!(out_proof_bytes);
    check_ptr!(out_proof_len);
    check_ptr!(out_derivation_path);
    check_ptr!(out_txid);

    let funding = unwrap_option_or_return!(parse_funding_type(funding_type));

    let signer_addr = core_signer_handle as usize;

    let option = ASSET_LOCK_MANAGER_STORAGE.with_item(handle, |manager| {
        let wallet_id = manager.wallet_id();
        let network = manager.network();
        let signer = unsafe {
            MnemonicResolverCoreSigner::new(
                signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        runtime().block_on(manager.create_funded_asset_lock_proof(
            amount_duffs,
            account_index,
            funding,
            identity_index,
            &signer,
        ))
    });
    let result = unwrap_option_or_return!(option);
    let (proof, path, out_point) = unwrap_result_or_return!(result);

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
    let mut txid_bytes = [0u8; 32];
    txid_bytes.copy_from_slice(&out_point.txid[..]);
    *out_txid = txid_bytes;

    PlatformWalletFFIResult::ok()
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
