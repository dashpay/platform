//! FFI binding for `IdentityWallet::token_transfer_with_external_signer`.

use std::ffi::CStr;
use std::os::raw::c_char;

use dash_sdk::platform::tokens::transitions::TransferResult;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use super::balances_json::{token_balances_to_json_cstring, write_empty_balances_json};
use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Transfer tokens from `identity_id` to `recipient_id`.
///
/// On success, `out_balances_json` is written with a heap-allocated C
/// string holding a JSON object mapping each affected identity's
/// base58 id to its proof-verified post-transfer balance, encoded as a
/// decimal **string** (u64 exceeds JSON's safe-integer range). For a
/// standard transfer this carries both the sender's and the
/// recipient's balances. History-tracking / group-action tokens carry
/// no balances in the proof result, so an empty object `{}` is written.
/// The caller owns the string and must free it via
/// [`platform_wallet_string_free`](crate::types::platform_wallet_string_free).
/// On error nothing is written through `out_balances_json` (it is set
/// to null first) and the failure surfaces via the returned result.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_token_transfer(
    wallet_handle: Handle,
    identity_id: *const u8,
    token_contract_id: *const u8,
    token_position: u16,
    recipient_id: *const u8,
    amount: u64,
    public_note: *const c_char,
    _signing_key_id: u32,
    signer_handle: *mut SignerHandle,
    out_balances_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(out_balances_json);
    *out_balances_json = std::ptr::null_mut();

    let from_id = unwrap_result_or_return!(read_identifier(identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(token_contract_id));
    let to_id = unwrap_result_or_return!(read_identifier(recipient_id));

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
                .token_transfer_with_external_signer(
                    from_id,
                    contract_id,
                    token_position,
                    to_id,
                    amount,
                    public_note_str,
                    signer,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let transfer_result = unwrap_result_or_return!(result);

    // Map the proof-verified outcome to the balances JSON. Only the
    // standard (non-history, non-group) transfer carries identity
    // balances; the other variants have none to persist.
    match transfer_result {
        TransferResult::IdentitiesBalances(balances) => {
            let c_str = unwrap_result_or_return!(token_balances_to_json_cstring(&balances));
            *out_balances_json = c_str;
        }
        TransferResult::HistoricalDocument(_) | TransferResult::GroupActionWithDocument(_, _) => {
            *out_balances_json = unwrap_result_or_return!(write_empty_balances_json());
        }
    }

    PlatformWalletFFIResult::ok()
}
