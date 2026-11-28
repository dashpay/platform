//! The fund a registration of a DPNS name pays to join the name's contest

use std::os::raw::c_char;

use crate::decision_proposal::{json_result, required_str, sdk_wrapper};
use crate::types::SDKHandle;
use crate::{DashSDKResult, FFIError};
use serde_json::{json, Value as JsonValue};

/// Read the fund a registration of a DPNS name pays to join the name's contest now
///
/// The fund doubles once a contest holds 250 contenders and again for every 50 more, so it
/// is read from the network: one proved request for the DPNS contract and one per 100
/// contenders. It is what a registration that names no maximum states and is charged; a
/// registration signed later may be charged more.
///
/// # Arguments
/// * `sdk_handle` - Handle to the SDK instance
/// * `name` - The username: a label or a full name, in any case (e.g. "alice", "Alice.DASH")
///
/// # Returns
/// * On success: a JSON object, `{"contestFund": 10000000000}` with the fund in credits, or
///   `{"contestFund": null}` when the name joins no contest (it is not contested, or empty)
/// * On error: An error result
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized `SDKHandle`.
/// - `name` must point to a NUL-terminated C string valid for the duration of the call.
/// - On success, returns a C string pointer inside `DashSDKResult`; free it with
///   `dash_sdk_string_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_dpns_get_contest_fund_to_join(
    sdk_handle: *const SDKHandle,
    name: *const c_char,
) -> DashSDKResult {
    json_result(contest_fund_to_join_inner(sdk_handle, name).map(Some))
}

unsafe fn contest_fund_to_join_inner(
    sdk_handle: *const SDKHandle,
    name: *const c_char,
) -> Result<JsonValue, FFIError> {
    let wrapper = sdk_wrapper(sdk_handle)?;
    let name = required_str(name, "name")?.to_string();
    let contest_fund = wrapper
        .runtime
        .try_block_on(wrapper.sdk.dpns_contest_fund_to_join(&name))??;
    Ok(json!({ "contestFund": contest_fund }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
    use crate::{
        dash_sdk_error_free, dash_sdk_string_free, DashSDKErrorCode, DashSDKResultDataType,
    };
    use std::ffi::{CStr, CString};

    #[test]
    fn should_refuse_a_null_sdk_handle_or_name_before_any_request() {
        let handle = create_mock_sdk_handle();
        let name = CString::new("alice").expect("no NUL");
        unsafe {
            for (sdk, name) in [
                (std::ptr::null(), name.as_ptr()),
                (handle as *const SDKHandle, std::ptr::null()),
            ] {
                let result = dash_sdk_dpns_get_contest_fund_to_join(sdk, name);
                assert!(!result.error.is_null(), "the call must be refused");
                assert_eq!((*result.error).code, DashSDKErrorCode::InvalidParameter);
                dash_sdk_error_free(result.error);
            }
        }
        destroy_mock_sdk_handle(handle);
    }

    /// An empty name joins no contest and nothing is read for it (the mock SDK answers no
    /// request), so the fund is null rather than absent
    #[test]
    fn should_return_a_null_fund_for_a_name_that_joins_no_contest() {
        let handle = create_mock_sdk_handle();
        let name = CString::new(".dash").expect("no NUL");
        unsafe {
            let result =
                dash_sdk_dpns_get_contest_fund_to_join(handle as *const SDKHandle, name.as_ptr());
            assert!(result.error.is_null(), "the call must succeed");
            assert_eq!(result.data_type, DashSDKResultDataType::String);
            let json: JsonValue = serde_json::from_str(
                CStr::from_ptr(result.data as *const c_char)
                    .to_str()
                    .expect("UTF-8"),
            )
            .expect("JSON");
            dash_sdk_string_free(result.data as *mut c_char);
            assert_eq!(json, json!({ "contestFund": null }));
        }
        destroy_mock_sdk_handle(handle);
    }
}
