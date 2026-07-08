//! FFI binding for `IdentityWallet::token_mint_with_external_signer`.

use std::ffi::CStr;
use std::os::raw::c_char;

use dash_sdk::platform::tokens::transitions::MintResult;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use super::balances_json::{single_token_balance_to_json_cstring, write_empty_balances_json};
use super::group_info::decode_group_info;
use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Mint `amount` of token at `token_position` on `token_contract_id`.
///
/// On success, `out_balances_json` is written with a heap-allocated C
/// string holding a JSON object mapping the recipient identity's base58
/// id to its proof-verified post-mint balance, encoded as a decimal
/// **string** (u64 exceeds JSON's safe-integer range), e.g.
/// `{"<recipientBase58>": "<newBalance>"}`. History-tracking / group
/// tokens carry no per-identity balance in the proof result, so an empty
/// object `{}` is written. The caller owns the string and must free it
/// via [`platform_wallet_string_free`](crate::types::platform_wallet_string_free).
/// On error nothing is written through `out_balances_json` (it is set to
/// null first) and the failure surfaces via the returned result.
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
    out_balances_json: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(signer_handle);
    check_ptr!(out_balances_json);
    *out_balances_json = std::ptr::null_mut();

    let id = unwrap_result_or_return!(read_identifier(identity_id));
    let contract_id = unwrap_result_or_return!(read_identifier(token_contract_id));

    let recipient = if issued_to_identity_id.is_null() {
        None
    } else {
        Some(unwrap_result_or_return!(read_identifier(
            issued_to_identity_id
        )))
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
                .token_mint_with_external_signer(
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
        })
    });
    let result = unwrap_option_or_return!(option);
    let mint_result = unwrap_result_or_return!(result);

    // Map the proof-verified outcome to the balances JSON, mirroring the
    // burn path. A standard mint carries the recipient's new balance
    // (`recipient` from the proof); a group-authorized mint carries the
    // recipient's balance once the action CLOSES. The recipient defaults
    // to the minting identity `id` when no explicit destination was given.
    // The document variants, and a group action still awaiting
    // co-signatures (`balance == None`), have nothing to persist.
    match mint_result {
        MintResult::TokenBalance(recipient_id, new_balance) => {
            let c_str = unwrap_result_or_return!(single_token_balance_to_json_cstring(
                &recipient_id,
                new_balance
            ));
            *out_balances_json = c_str;
        }
        MintResult::GroupActionWithBalance(_, _, Some(new_balance)) => {
            let target = recipient.unwrap_or(id);
            let c_str = unwrap_result_or_return!(single_token_balance_to_json_cstring(
                &target,
                new_balance
            ));
            *out_balances_json = c_str;
        }
        MintResult::HistoricalDocument(_)
        | MintResult::GroupActionWithDocument(_, _)
        | MintResult::GroupActionWithBalance(_, _, None) => {
            *out_balances_json = unwrap_result_or_return!(write_empty_balances_json());
        }
    }

    PlatformWalletFFIResult::ok()
}
