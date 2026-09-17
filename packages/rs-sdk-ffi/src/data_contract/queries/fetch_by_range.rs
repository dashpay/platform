//! Paginated enumeration of every data contract on Platform (`getDataContractsByRange`).
//!
//! One call returns one page, ordered by ascending contract id. Callers walk the
//! enumeration by passing the last `id` of a page back as `start_after`; a page shorter
//! than the requested limit is the last one.

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::platform::data_contracts_by_range::{
    DataContractsByRange, DataContractsByRangeQuery, DataContractsByRangeStart,
};
use dash_sdk::platform::{Fetch, Identifier};
use serde_json::{Map, Value};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::data_contract::json::contract_json_value;
use crate::error::{DashSDKError, DashSDKErrorCode, FFIError};
use crate::runtime::BigStackRuntime;
use crate::sdk::SDKWrapper;
use crate::types::{DashSDKResult, SDKHandle};

/// Fetch one page of the contract enumeration (`getDataContractsByRange`).
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance
/// * `limit` - Maximum number of contracts in the page; `0` means "use the default (100)"
/// * `start_after` - Start after this Base58 contract id, exclusive (optional)
/// * `start_at` - Start at this Base58 contract id, inclusive (optional)
/// * `ids_only` - Return ids only; every `dataContract` field is then `null`
///
/// # Returns
/// A JSON array, one element per contract in ascending contract id order:
/// `[{"id": "<base58 contract id>", "dataContract": <contract JSON or null>}, ...]`.
/// The array order is the cursor order, so the last element's `id` is the next
/// `start_after`. An exhausted enumeration returns `[]`, not an error.
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer.
/// - `start_after` and `start_at` may be null; when non-null they must point to NUL-terminated
///   C strings holding a Base58 contract id. They are mutually exclusive.
/// - On success, returns a C string inside `DashSDKResult`; caller frees it with SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contracts_fetch_by_range(
    sdk_handle: *const SDKHandle,
    limit: u32,
    start_after: *const c_char,
    start_at: *const c_char,
    ids_only: bool,
) -> DashSDKResult {
    match fetch_data_contracts_by_range(sdk_handle, limit, start_after, start_at, ids_only) {
        Ok(json) => match CString::new(json) {
            Ok(c_str) => DashSDKResult::success_string(c_str.into_raw()),
            Err(e) => DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create CString: {}", e),
            )),
        },
        Err(e) => DashSDKResult::error(e),
    }
}

/// Parse a Base58 contract id out of one of the cursor arguments.
///
/// # Safety
/// `value` must be non-null and point to a NUL-terminated C string that stays valid for
/// the duration of the call.
unsafe fn parse_cursor(value: *const c_char, argument: &str) -> Result<Identifier, DashSDKError> {
    let id_str = CStr::from_ptr(value).to_str().map_err(|e| {
        DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!("Invalid UTF-8 in {}: {}", argument, e),
        )
    })?;

    Identifier::from_string(id_str, Encoding::Base58).map_err(|e| {
        DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!("Invalid contract ID in {}: {}", argument, e),
        )
    })
}

/// # Safety
/// Same contract as [`dash_sdk_data_contracts_fetch_by_range`].
unsafe fn fetch_data_contracts_by_range(
    sdk_handle: *const SDKHandle,
    limit: u32,
    start_after: *const c_char,
    start_at: *const c_char,
    ids_only: bool,
) -> Result<String, DashSDKError> {
    if sdk_handle.is_null() {
        return Err(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if !start_after.is_null() && !start_at.is_null() {
        return Err(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "start_after and start_at are mutually exclusive".to_string(),
        ));
    }

    let start = if !start_after.is_null() {
        Some(DataContractsByRangeStart::After(parse_cursor(
            start_after,
            "start_after",
        )?))
    } else if !start_at.is_null() {
        Some(DataContractsByRangeStart::At(parse_cursor(
            start_at, "start_at",
        )?))
    } else {
        None
    };

    // The limit MUST travel in the request: Drive builds the proof for the limit it
    // applied (its default when none is given) and the verifier re-derives the query
    // shape from the request, so a limit dropped here fails proof verification. `0`
    // means "Drive's default page".
    let query = DataContractsByRangeQuery {
        start,
        limit: if limit > 0 { Some(limit) } else { None },
        ids_only,
    };

    let rt = BigStackRuntime::new_isolated().map_err(|e| {
        DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("Failed to create Tokio runtime: {}", e),
        )
    })?;

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let sdk = wrapper.sdk.clone();

    let page = rt.block_on(async move {
        DataContractsByRange::fetch(&sdk, query)
            .await
            .map_err(FFIError::from)
    });

    // `fetch` always returns a page, an empty one included; treat a missing page as
    // empty so an exhausted enumeration reads as `[]` rather than an error.
    let page = page?.unwrap_or_default();

    // Render every contract at the SDK's network protocol version, the same
    // way every other contract emitter does.
    let platform_version = wrapper.sdk.version();
    let mut entries = Vec::with_capacity(page.0.len());
    for (id, contract) in page.0.iter() {
        let contract_json = match contract {
            Some(contract) => contract_json_value(contract, platform_version)?,
            None => Value::Null,
        };

        let mut entry = Map::new();
        entry.insert(
            "id".to_string(),
            Value::String(id.to_string(Encoding::Base58)),
        );
        entry.insert("dataContract".to_string(), contract_json);
        entries.push(Value::Object(entry));
    }

    serde_json::to_string(&Value::Array(entries)).map_err(|e| {
        DashSDKError::new(
            DashSDKErrorCode::SerializationError,
            format!("Failed to serialize contract page: {}", e),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_contracts_fetch_by_range_null_handle() {
        unsafe {
            let result = dash_sdk_data_contracts_fetch_by_range(
                std::ptr::null(),
                10,
                std::ptr::null(),
                std::ptr::null(),
                false,
            );
            assert!(!result.error.is_null());
        }
    }

    #[test]
    fn test_data_contracts_fetch_by_range_rejects_both_cursors() {
        let after = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec")
            .expect("literal has no NUL byte");
        let at = CString::new("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec")
            .expect("literal has no NUL byte");
        let handle = crate::test_utils::test_utils::create_mock_sdk_handle();
        unsafe {
            let result = dash_sdk_data_contracts_fetch_by_range(
                handle,
                10,
                after.as_ptr(),
                at.as_ptr(),
                false,
            );
            assert!(!result.error.is_null());
            crate::test_utils::test_utils::destroy_mock_sdk_handle(handle);
        }
    }
}
