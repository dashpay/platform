//! What is left of the budgets of identity keys (protocol version 14)

use dash_sdk::dpp::identity::KeyID;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::identity_keys_remaining_budgets::{
    IdentityKeysRemainingBudgets, IdentityKeysRemainingBudgetsQuery,
};
use dash_sdk::platform::Fetch;
use serde_json::{Map, Value};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::slice;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch what is left of the budgets of the given keys of an identity.
///
/// An authentication key registered with a total budget spends it as its transitions run;
/// Platform tracks what remains next to the key. Raising the budget with a key limits update
/// raises what remains by the same amount.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_id`: Base58-encoded identity ID
/// - `key_ids`: the ids of the keys to look up, at least one, none repeated
/// - `key_ids_len`: how many ids `key_ids` points at
///
/// # Returns
/// A JSON object keyed by key id: `{"5": "1000", "6": null}`. A budgeted key maps to what is
/// left of its budget in credits, as a decimal string; a key that has no budget, or that the
/// identity does not have, maps to null.
///
/// # Safety
/// - `sdk_handle`, `identity_id` and `key_ids` must be valid, non-null pointers.
/// - `identity_id` must point to a NUL-terminated C string valid for the duration of the call.
/// - `key_ids` must point to `key_ids_len` readable `u32` values.
/// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_fetch_keys_remaining_budgets(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
    key_ids: *const u32,
    key_ids_len: usize,
) -> DashSDKResult {
    if sdk_handle.is_null() || identity_id.is_null() || key_ids.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle, identity ID or key ids is null".to_string(),
        ));
    }
    if key_ids_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "At least one key id is required".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ))
        }
    };

    let key_ids: Vec<KeyID> = slice::from_raw_parts(key_ids, key_ids_len).to_vec();

    let result: Result<Map<String, Value>, FFIError> = wrapper.runtime.block_on(async {
        let budgets = IdentityKeysRemainingBudgets::fetch(
            &wrapper.sdk,
            IdentityKeysRemainingBudgetsQuery {
                identity_id: id,
                key_ids: key_ids.clone(),
            },
        )
        .await
        .map_err(FFIError::from)?
        .unwrap_or_default();

        Ok(key_ids
            .iter()
            .map(|key_id| {
                let remaining = match budgets.get(key_id) {
                    Some(Some(credits)) => Value::String(credits.to_string()),
                    _ => Value::Null,
                };
                (key_id.to_string(), remaining)
            })
            .collect())
    });

    match result {
        Ok(map) => match CString::new(Value::Object(map).to_string()) {
            Ok(s) => DashSDKResult::success_string(s.into_raw()),
            Err(e) => DashSDKResult::error(
                FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
            ),
        },
        Err(e) => DashSDKResult::error(e.into()),
    }
}
