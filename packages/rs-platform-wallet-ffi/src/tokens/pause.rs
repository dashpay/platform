//! FFI binding for `IdentityWallet::token_pause_with_external_signer`.

use std::ffi::CStr;
use std::os::raw::c_char;

use rs_sdk_ffi::{SignerHandle, VTableSigner};

use super::group_info::decode_group_info;
use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Pause all operations for the token at `token_position` on `token_contract_id`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_token_pause(
    wallet_handle: Handle,
    identity_id: *const u8,
    token_contract_id: *const u8,
    token_position: u16,
    public_note: *const c_char,
    group_info_kind: u8,
    group_info_position: u16,
    group_info_action_id: *const u8,
    group_info_action_is_proposer: bool,
    _signing_key_id: u32,
    signer_handle: *mut SignerHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);

    let id = unwrap_result_or_return!(read_identifier(identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(token_contract_id));

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

    let group_info = unwrap_result_or_return!(decode_group_info(
        group_info_kind,
        group_info_position,
        group_info_action_id,
        group_info_action_is_proposer,
    ));

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity_wallet
                .token_pause_with_external_signer(
                    id,
                    contract_id,
                    token_position,
                    public_note_str,
                    group_info,
                    signer,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}
