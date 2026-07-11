//! FFI binding for `IdentityWallet::token_claim_with_external_signer`.
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

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Claim a distribution payout for `identity_id` from the token at
/// `token_position` on `token_contract_id`.
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
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);

    let id = unwrap_result_or_return!(read_identifier(identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(token_contract_id));

    let dist_type = match distribution_type {
        0 => TokenDistributionType::PreProgrammed,
        1 => TokenDistributionType::Perpetual,
        other => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("Invalid distribution_type: {other} (expected 0 or 1)"),
            );
        }
    };

    let public_note_str = if public_note.is_null() {
        None
    } else {
        {
            let s = unwrap_result_or_return!(CStr::from_ptr(public_note).to_str());
            if s.is_empty() {
                None
            } else {
                Some(s.to_owned())
            }
        }
    };

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity_wallet
                .token_claim_with_external_signer(
                    id,
                    contract_id,
                    token_position,
                    dist_type,
                    public_note_str,
                    signer,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}
