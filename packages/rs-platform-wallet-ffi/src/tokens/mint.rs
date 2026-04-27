//! FFI binding for `TokenWallet::mint_external_signer`.
//!
//! Mint supports group-gated execution; the caller passes a flat
//! `(group_info_kind, position, action_id, action_is_proposer)` tuple
//! that we decode into `Option<GroupStateTransitionInfoStatus>` via
//! `super::group_info::decode_group_info`.
//!
//! Mint also takes an optional recipient identity id. When NULL the
//! tokens are issued to `identity_id` (mint-to-self). When non-NULL
//! the tokens are issued to that identity (subject to the contract's
//! `mintingAllowChoosingDestination` rule, enforced server-side).

use std::ffi::CStr;
use std::os::raw::c_char;

use rs_sdk_ffi::{SignerHandle, VTableSigner};

use super::group_info::{decode_group_info, GroupInfoDecode};
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;

/// Mint `amount` of token at `token_position` on `token_contract_id`.
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `identity_id`, `token_contract_id` must each point at exactly 32
///   readable bytes.
/// - `issued_to_identity_id` may be NULL (mint-to-self); when non-NULL
///   it must point at exactly 32 readable bytes.
/// - `public_note` may be NULL; when non-NULL it must be a
///   NUL-terminated UTF-8 C string.
/// - `group_info_action_id` must point at 32 bytes when
///   `group_info_kind == 2`; ignored otherwise (may be NULL).
/// - `signer_handle` must be a valid, non-destroyed handle from
///   `dash_sdk_signer_create_with_ctx`. Caller retains ownership.
/// - `out_error` may be NULL.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_token_mint(
    wallet_handle: Handle,
    identity_id: *const u8,
    token_contract_id: *const u8,
    token_position: u16,
    issued_to_identity_id: *const u8,
    amount: u64,
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

    let recipient = if issued_to_identity_id.is_null() {
        None
    } else {
        match read_identifier(issued_to_identity_id) {
            Ok(i) => Some(i),
            Err(e) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid issued_to_identity_id: {e}"),
                    );
                }
                return PlatformWalletFFIResult::ErrorInvalidIdentifier;
            }
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
            let token_wallet = wallet.tokens().clone();
            let result = block_on_worker(async move {
                let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
                token_wallet
                    .mint_external_signer(
                        id,
                        contract_id,
                        token_position,
                        recipient,
                        amount,
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
                            format!("token_mint failed: {e}"),
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
