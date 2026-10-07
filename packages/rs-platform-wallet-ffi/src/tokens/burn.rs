//! FFI binding for `IdentityWallet::token_burn_with_external_signer`.

use std::ffi::CStr;
use std::os::raw::c_char;

use dash_sdk::platform::tokens::transitions::BurnResult;
use dpp::prelude::Identifier;
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use super::balances_json::{single_token_balance_to_json_cstring, write_empty_balances_json};
use super::group_info::decode_group_info;
use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::read_identifier;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Burn `amount` of token at `token_position` on `token_contract_id`,
/// debiting the caller's balance.
///
/// On success, `out_balances_json` is written with a heap-allocated C
/// string holding a JSON object mapping the burning identity's base58
/// id to its proof-verified remaining balance, encoded as a decimal
/// **string** (u64 exceeds JSON's safe-integer range), e.g.
/// `{"<ownerBase58>": "<remaining>"}`. A no-history group burn carries
/// a balance only when the caller proposed the action.
/// History results and co-signers carry no per-identity balance, so
/// an empty object `{}` is written. The caller owns the string and must
/// free it via
/// [`platform_wallet_string_free`](crate::types::platform_wallet_string_free).
/// On error nothing is written through `out_balances_json` (it is set
/// to null first) and the failure surfaces via the returned result.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_token_burn(
    wallet_handle: Handle,
    identity_id: *const u8,
    token_contract_id: *const u8,
    token_position: u16,
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
                .token_burn_with_external_signer(
                    id,
                    contract_id,
                    token_position,
                    amount,
                    public_note_str,
                    group_info,
                    signer,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let burn_result = unwrap_result_or_return!(result);

    *out_balances_json = unwrap_result_or_return!(burn_result_to_balances_json(burn_result, &id));

    PlatformWalletFFIResult::ok()
}

// A no-history group burn may carry the proposer's balance, including zero.
// Co-signers carry no balance even after closure; history results also have
// nothing to persist. An empty object leaves existing cached balances alone.
fn burn_result_to_balances_json(
    burn_result: BurnResult,
    caller: &Identifier,
) -> Result<*mut c_char, PlatformWalletFFIResult> {
    match burn_result {
        BurnResult::TokenBalance(owner, remaining) => {
            single_token_balance_to_json_cstring(&owner, remaining)
        }
        BurnResult::GroupActionWithBalance(_, _, Some(remaining)) => {
            single_token_balance_to_json_cstring(caller, remaining)
        }
        BurnResult::HistoricalDocument(_)
        | BurnResult::GroupActionWithDocument(_, _)
        | BurnResult::GroupActionWithBalance(_, _, None) => write_empty_balances_json(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::group::group_action_status::GroupActionStatus;
    use dpp::platform_value::string_encoding::Encoding;
    use std::ffi::CString;

    #[test]
    fn should_return_empty_balances_for_a_closed_group_burn_cosigner() {
        let caller = Identifier::from([2; 32]);
        let ptr = burn_result_to_balances_json(
            BurnResult::GroupActionWithBalance(8, GroupActionStatus::ActionClosed, None),
            &caller,
        )
        .expect("serialize closed co-signer result");
        let json = unsafe { CString::from_raw(ptr) };
        assert_eq!(json.to_str().expect("UTF-8 JSON"), "{}");
    }

    #[test]
    fn should_return_proposer_group_burn_balance_including_zero() {
        let caller = Identifier::from([1; 32]);
        for balance in [7, 0] {
            let ptr = burn_result_to_balances_json(
                BurnResult::GroupActionWithBalance(
                    3,
                    GroupActionStatus::ActionClosed,
                    Some(balance),
                ),
                &caller,
            )
            .expect("serialize proposer result");
            let json = unsafe { CString::from_raw(ptr) };
            let parsed: serde_json::Value =
                serde_json::from_str(json.to_str().expect("UTF-8 JSON"))
                    .expect("parse balance object");
            assert_eq!(
                parsed,
                serde_json::json!({caller.to_string(Encoding::Base58): balance.to_string()})
            );
        }
    }
}
