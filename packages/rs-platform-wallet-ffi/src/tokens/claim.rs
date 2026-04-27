//! FFI binding for `TokenWallet::claim_external_signer`.
//!
//! Claim is not group-gated — there's no `group_info_*` payload here.
//! It does take a distribution-type discriminant, surfaced as a `u8`.
//!
//! Distribution-type mapping (mirrors
//! `dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType`):
//!
//! | discriminant | variant         |
//! |--------------|-----------------|
//! | `0`          | `PreProgrammed` |
//! | `1`          | `Perpetual`     |
//!
//! Any other value is rejected with `ErrorInvalidParameter`.

use std::ffi::CStr;
use std::os::raw::c_char;

use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;

/// Claim a distribution payout for `identity_id` from the token at
/// `token_position` on `token_contract_id`.
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `identity_id`, `token_contract_id` must each point at exactly 32
///   readable bytes.
/// - `public_note` may be NULL; when non-NULL it must be a
///   NUL-terminated UTF-8 C string.
/// - `signer_handle` must be a valid, non-destroyed handle from
///   `dash_sdk_signer_create_with_ctx`. Caller retains ownership.
/// - `out_error` may be NULL.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_token_claim(
    wallet_handle: Handle,
    identity_id: *const u8,
    token_contract_id: *const u8,
    token_position: u16,
    distribution_type: u8,
    public_note: *const c_char,
    _signing_key_id: u32,
    signer_handle: *mut SignerHandle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if signer_handle.is_null() {
        if !out_error.is_null() {
            *out_error = PlatformWalletFFIError::new(
                PlatformWalletFFIResult::ErrorNullPointer,
                "signer_handle is null",
            );
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let id = match read_identifier(identity_id) {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidIdentifier,
                    format!("Invalid identity_id: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };
    let contract_id = match read_identifier(token_contract_id) {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidIdentifier,
                    format!("Invalid token_contract_id: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    let dist_type = match distribution_type {
        0 => TokenDistributionType::PreProgrammed,
        1 => TokenDistributionType::Perpetual,
        other => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!("Invalid distribution_type: {other} (expected 0 or 1)"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };

    let public_note_str = if public_note.is_null() {
        None
    } else {
        match CStr::from_ptr(public_note).to_str() {
            Ok(s) if s.is_empty() => None,
            Ok(s) => Some(s.to_owned()),
            Err(e) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        format!("public_note is not valid UTF-8: {e}"),
                    );
                }
                return PlatformWalletFFIResult::ErrorUtf8Conversion;
            }
        }
    };

    let signer_addr = signer_handle as usize;

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let token_wallet = wallet.tokens().clone();
            let result = block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                token_wallet
                    .claim_external_signer(
                        id,
                        contract_id,
                        token_position,
                        dist_type,
                        public_note_str,
                        signer,
                    )
                    .await
            });
            match result {
                Ok(_) => PlatformWalletFFIResult::Success,
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("token_claim failed: {e}"),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidHandle,
                    "Invalid platform-wallet handle",
                );
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}
