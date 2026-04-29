//! FFI binding for `IdentityWallet::token_destroy_frozen_funds_with_external_signer`.
//!
//! Destroy-frozen-funds is group-capable; same shape as Freeze /
//! Unfreeze. The Rust builder takes no `amount` — destroying always
//! affects the full frozen balance of `frozen_identity_id`.

use std::ffi::CStr;
use std::os::raw::c_char;

use rs_sdk_ffi::{SignerHandle, VTableSigner};

use super::group_info::{decode_group_info, GroupInfoDecode};
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;

/// Destroy the entire frozen balance of `frozen_identity_id` for the
/// token at `token_position` on `token_contract_id`.
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `identity_id`, `token_contract_id`, `frozen_identity_id` must each
///   point at exactly 32 readable bytes.
/// - `public_note` may be NULL; when non-NULL it must be a
///   NUL-terminated UTF-8 C string.
/// - `group_info_action_id` must point at 32 bytes when
///   `group_info_kind == 2`; ignored otherwise (may be NULL).
/// - `signer_handle` must be a valid, non-destroyed handle from
///   `dash_sdk_signer_create_with_ctx`. Caller retains ownership.
/// - `out_error` may be NULL.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_token_destroy_frozen_funds(
    wallet_handle: Handle,
    identity_id: *const u8,
    token_contract_id: *const u8,
    token_position: u16,
    frozen_identity_id: *const u8,
    public_note: *const c_char,
    group_info_kind: u8,
    group_info_position: u16,
    group_info_action_id: *const u8,
    group_info_action_is_proposer: bool,
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
    let frozen_id = match read_identifier(frozen_identity_id) {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidIdentifier,
                    format!("Invalid frozen_identity_id: {e}"),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    let public_note_str = if public_note.is_null() {
        None
    } else {
        match CStr::from_ptr(public_note).to_str() {
            Ok("") => None,
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

    let group_info = match decode_group_info(
        group_info_kind,
        group_info_position,
        group_info_action_id,
        group_info_action_is_proposer,
        out_error,
    ) {
        GroupInfoDecode::Ok(value) => value,
        GroupInfoDecode::Err(code) => return code,
    };

    let signer_addr = signer_handle as usize;

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity_wallet = wallet.identity().clone();
            let result = block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                identity_wallet
                    .token_destroy_frozen_funds_with_external_signer(
                        id,
                        contract_id,
                        token_position,
                        frozen_id,
                        public_note_str,
                        group_info,
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
                            format!("token_destroy_frozen_funds failed: {e}"),
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
