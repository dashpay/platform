//! Blocks of the Platform chain and the state transitions in them (protocol version 15): thin
//! bindings over `Sdk::fetch_blocks` and `Sdk::fetch_owner_transactions_in_block`. rs-sdk sends
//! the request, verifies the Tenderdash block proofs when the SDK proves (the default) and
//! refuses a network below protocol version 15; these functions read the C arguments, call it
//! and write the answer as JSON.
//!
//! A block and a state transition are written the same way everywhere in this module and in
//! `transition_status`; see [`block_json`] and [`transaction_json`].
//!
//! Each read runs on the SDK handle's own runtime with `BigStackRuntime::try_block_on`: the
//! SDK keeps its gRPC channels in a pool that outlives one call, and a channel's worker lives
//! on the runtime that created it, so a runtime made for one call and then dropped would
//! break the next request on that channel. A worker thread the OS will not start comes back
//! as an error instead of aborting the host app.

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKResult, FFIError};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::blocks::{
    Block, BlockTransaction, Blocks, BlocksQuery, OwnerTransactionsInBlock,
    OwnerTransactionsInBlockQuery,
};
use serde_json::{json, Value as JsonValue};
use std::ffi::{c_char, CStr, CString};

/// Fetches up to `count` consecutive blocks from `start_height`, each with every state
/// transition it holds and the result of its execution (`Sdk::fetch_blocks`).
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `start_height` - The height of the first block, from 1.
/// * `count` - How many blocks to return, from 1 to 10. The node refuses any other count.
///
/// # Returns
/// A JSON object, free it with `dash_sdk_string_free`:
///
/// ```json
/// {"proved": true,
///  "blocks": [{"height": 120, "hash": "<hex>", "time_ms": 1700000000000,
///              "core_chain_locked_height": 2100, "app_hash": "<hex>",
///              "proposer_pro_tx_hash": "<hex>",
///              "transactions": [{"hash": "<hex>", "code": 0, "executed": true,
///                                "gas_used": 1200,
///                                "result_data": "<hex>", "type": "IdentityCreditTransfer",
///                                "owner_id": "<base58>"}]}]}
/// ```
///
/// `proved` is true when the SDK verifies proofs: each block is then the one a validator
/// quorum committed at its height, with exactly these transitions and results. There can be
/// fewer blocks than asked for, and nothing proves a block left off the end does not exist.
/// A transaction's `code` is 0 when it executed, otherwise the code of the error it paid for,
/// and `executed` is rs-sdk's reading of that code (`BlockTransaction::executed`); `owner_id`
/// is null for a transition that reports no owner. Hashes are hex in the byte order the block
/// carries them.
///
/// Before the network runs protocol version 15 the call fails with the SDK's protocol
/// version refusal, without sending anything.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_system_get_blocks(
    sdk_handle: *const SDKHandle,
    start_height: u64,
    count: u32,
) -> DashSDKResult {
    json_result(get_blocks(sdk_handle, start_height, count))
}

unsafe fn get_blocks(
    sdk_handle: *const SDKHandle,
    start_height: u64,
    count: u32,
) -> Result<JsonValue, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let sdk = &wrapper.sdk;
    let blocks = wrapper
        .runtime
        .try_block_on(sdk.fetch_blocks(BlocksQuery {
            start_height,
            count,
        }))??;
    Ok(blocks_json(&blocks, sdk.prove()))
}

/// Fetches the state transitions one identity owns in the block at `height`, each with the
/// result of its execution (`Sdk::fetch_owner_transactions_in_block`).
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
/// * `height` - The height of the block, from 1.
/// * `owner_id` - The identity, base58.
///
/// # Returns
/// A JSON object, free it with `dash_sdk_string_free`:
///
/// ```json
/// {"proved": true, "height": 120, "transactions": [<transaction>, ...]}
/// ```
///
/// with each transaction written as in [`dash_sdk_system_get_blocks`]. When `proved` is true
/// every transaction is in that block, with that result, and is the owner's, but the list is
/// NOT proved complete: a node can leave some of the owner's transitions out. Reading the
/// whole block with [`dash_sdk_system_get_blocks`] gives them all. A recipient is not an
/// owner.
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `owner_id` must be a valid pointer to a NUL-terminated string for the duration of the
///   call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_system_get_owner_transactions_in_block(
    sdk_handle: *const SDKHandle,
    height: u64,
    owner_id: *const c_char,
) -> DashSDKResult {
    json_result(get_owner_transactions_in_block(
        sdk_handle, height, owner_id,
    ))
}

unsafe fn get_owner_transactions_in_block(
    sdk_handle: *const SDKHandle,
    height: u64,
    owner_id: *const c_char,
) -> Result<JsonValue, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let owner_id = required_str(owner_id, "owner_id")?;
    let owner_id = Identifier::from_base58_text(owner_id).ok_or_else(|| {
        FFIError::InvalidParameter("owner_id is not a base58 identifier".to_string())
    })?;
    let sdk = &wrapper.sdk;
    let answer =
        wrapper
            .runtime
            .try_block_on(sdk.fetch_owner_transactions_in_block(
                OwnerTransactionsInBlockQuery { height, owner_id },
            ))??;
    Ok(owner_transactions_json(&answer, sdk.prove()))
}

/// The SDK behind `sdk_handle`.
///
/// # Safety
/// `sdk_handle` must be null or point to an initialized `SDKHandle` that outlives `'a`.
pub(super) unsafe fn sdk_wrapper<'a>(
    sdk_handle: *const SDKHandle,
) -> Result<&'a SDKWrapper, FFIError> {
    if sdk_handle.is_null() {
        return Err(FFIError::InvalidParameter("SDK handle is null".to_string()));
    }
    Ok(&*(sdk_handle as *const SDKWrapper))
}

/// The UTF-8 string `field` points to.
///
/// # Safety
/// `ptr` must be null or point to a NUL-terminated string that outlives `'a`.
pub(super) unsafe fn required_str<'a>(
    ptr: *const c_char,
    field: &str,
) -> Result<&'a str, FFIError> {
    if ptr.is_null() {
        return Err(FFIError::InvalidParameter(format!("{field} is null")));
    }
    CStr::from_ptr(ptr)
        .to_str()
        .map_err(|e| FFIError::InvalidParameter(format!("{field} is not UTF-8: {e}")))
}

/// A result holding `json` written as an owned C string (freed with `dash_sdk_string_free`).
pub(super) fn json_result(json: Result<JsonValue, FFIError>) -> DashSDKResult {
    match json.and_then(|json| CString::new(json.to_string()).map_err(FFIError::from)) {
        Ok(string) => DashSDKResult::success_string(string.into_raw()),
        Err(error) => DashSDKResult::error(error.into()),
    }
}

/// A state transition a block holds, with its result. See [`dash_sdk_system_get_blocks`].
pub(super) fn transaction_json(transaction: &BlockTransaction) -> JsonValue {
    json!({
        "hash": hex::encode(transaction.hash),
        "code": transaction.code,
        "executed": transaction.executed(),
        "gas_used": transaction.gas_used,
        "result_data": hex::encode(&transaction.result_data),
        "type": transaction.state_transition.name(),
        "owner_id": transaction
            .state_transition
            .owner_id()
            .map(|owner_id| owner_id.to_string(Encoding::Base58)),
    })
}

/// A block with its state transitions. See [`dash_sdk_system_get_blocks`].
fn block_json(block: &Block) -> JsonValue {
    json!({
        "height": block.height,
        "hash": hex::encode(block.hash),
        "time_ms": block.time_ms,
        "core_chain_locked_height": block.core_chain_locked_height,
        "app_hash": hex::encode(block.app_hash),
        "proposer_pro_tx_hash": hex::encode(block.proposer_pro_tx_hash),
        "transactions": block.transactions.iter().map(transaction_json).collect::<Vec<_>>(),
    })
}

fn blocks_json(blocks: &Blocks, proved: bool) -> JsonValue {
    let Blocks(blocks) = blocks;
    json!({
        "proved": proved,
        "blocks": blocks.iter().map(block_json).collect::<Vec<_>>(),
    })
}

fn owner_transactions_json(answer: &OwnerTransactionsInBlock, proved: bool) -> JsonValue {
    json!({
        "proved": proved,
        "height": answer.height,
        "transactions": answer.transactions.iter().map(transaction_json).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::{dash_sdk_error_free, DashSDKErrorCode};
    use dash_sdk::dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dash_sdk::dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dash_sdk::dpp::state_transition::StateTransition;

    pub(crate) fn transaction(hash: [u8; 32], code: u32) -> BlockTransaction {
        BlockTransaction {
            state_transition: StateTransition::IdentityCreditTransfer(
                IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0::default()),
            ),
            hash,
            code,
            result_data: vec![0xab, 0xcd],
            gas_used: 1_200,
        }
    }

    /// The error code of `result`, freeing the error.
    pub(crate) unsafe fn error_code(result: DashSDKResult) -> DashSDKErrorCode {
        assert!(!result.error.is_null(), "expected an error");
        let code = (*result.error).code;
        dash_sdk_error_free(result.error);
        code
    }

    #[test]
    fn should_refuse_a_null_handle() {
        unsafe {
            assert_eq!(
                error_code(dash_sdk_system_get_blocks(std::ptr::null(), 1, 1)),
                DashSDKErrorCode::InvalidParameter
            );
            let owner =
                CString::new(Identifier::new([1; 32]).to_string(Encoding::Base58)).expect("no NUL");
            assert_eq!(
                error_code(dash_sdk_system_get_owner_transactions_in_block(
                    std::ptr::null(),
                    1,
                    owner.as_ptr(),
                )),
                DashSDKErrorCode::InvalidParameter
            );
        }
    }

    #[test]
    fn should_refuse_a_missing_or_malformed_owner_id_before_asking_the_network() {
        let handle = create_mock_sdk_handle();
        let not_base58 = CString::new("0OIl").expect("no NUL");
        // Far longer than any identifier's base58 form: refused without being decoded
        let oversized = CString::new("1".repeat(100_000)).expect("no NUL");
        unsafe {
            assert_eq!(
                error_code(dash_sdk_system_get_owner_transactions_in_block(
                    handle,
                    1,
                    std::ptr::null(),
                )),
                DashSDKErrorCode::InvalidParameter
            );
            assert_eq!(
                error_code(dash_sdk_system_get_owner_transactions_in_block(
                    handle,
                    1,
                    not_base58.as_ptr(),
                )),
                DashSDKErrorCode::InvalidParameter
            );
            assert_eq!(
                error_code(dash_sdk_system_get_owner_transactions_in_block(
                    handle,
                    1,
                    oversized.as_ptr(),
                )),
                DashSDKErrorCode::InvalidParameter
            );
            destroy_mock_sdk_handle(handle);
        }
    }

    #[test]
    fn should_write_a_block_with_its_transactions() {
        let block = Block {
            height: 120,
            hash: [9; 32],
            time_ms: 1_700_000_000_000,
            core_chain_locked_height: 2_100,
            app_hash: [1; 32],
            proposer_pro_tx_hash: [2; 32],
            transactions: vec![transaction([7; 32], 40_100)],
        };
        let json = blocks_json(&Blocks(vec![block]), true);
        assert_eq!(
            json,
            json!({
                "proved": true,
                "blocks": [{
                    "height": 120,
                    "hash": "09".repeat(32),
                    "time_ms": 1_700_000_000_000u64,
                    "core_chain_locked_height": 2_100,
                    "app_hash": "01".repeat(32),
                    "proposer_pro_tx_hash": "02".repeat(32),
                    "transactions": [{
                        "hash": "07".repeat(32),
                        "code": 40_100,
                        "executed": false,
                        "gas_used": 1_200,
                        "result_data": "abcd",
                        "type": "IdentityCreditTransfer",
                        "owner_id": Identifier::default().to_string(Encoding::Base58),
                    }],
                }],
            })
        );
    }

    #[test]
    fn should_write_an_owners_transactions_with_the_proved_flag() {
        let answer = OwnerTransactionsInBlock {
            height: 5,
            transactions: vec![transaction([3; 32], 0)],
        };
        let json = owner_transactions_json(&answer, false);
        assert_eq!(json["proved"], false);
        assert_eq!(json["height"], 5);
        assert_eq!(json["transactions"][0]["hash"], "03".repeat(32));
        assert_eq!(json["transactions"][0]["code"], 0);
        assert_eq!(json["transactions"][0]["executed"], true);
    }
}
