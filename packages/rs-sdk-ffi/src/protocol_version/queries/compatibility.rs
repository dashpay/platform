//! Thin C bindings for the SDK-owned protocol compatibility lifecycle.

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::{
    ProtocolCompatibility, ProtocolCompatibilityState, ProtocolCompatibilityUnavailableReason,
};
use serde::Serialize;
use std::ffi::CString;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompatibilitySnapshot {
    generation: u64,
    status: &'static str,
    verified_protocol_version: Option<u32>,
    effective_protocol_version: u32,
    maximum_supported_protocol_version: u32,
    unavailable_reason: Option<&'static str>,
}

impl From<ProtocolCompatibility> for CompatibilitySnapshot {
    fn from(value: ProtocolCompatibility) -> Self {
        let (status, unavailable_reason) = match value.state {
            ProtocolCompatibilityState::Unavailable { reason } => (
                "unavailable",
                Some(match reason {
                    ProtocolCompatibilityUnavailableReason::NotChecked => "notChecked",
                    ProtocolCompatibilityUnavailableReason::TimedOut => "timedOut",
                    ProtocolCompatibilityUnavailableReason::CheckFailed => "checkFailed",
                    ProtocolCompatibilityUnavailableReason::ProofsDisabled => "proofsDisabled",
                }),
            ),
            ProtocolCompatibilityState::Checking => ("checking", None),
            ProtocolCompatibilityState::Checked => ("checked", None),
        };
        Self {
            generation: value.generation,
            status,
            verified_protocol_version: value.verified_protocol_version,
            effective_protocol_version: value.effective_protocol_version,
            maximum_supported_protocol_version: value.maximum_supported_protocol_version,
            unavailable_reason,
        }
    }
}

fn json_result(value: &impl Serialize) -> DashSDKResult {
    match serde_json::to_string(value)
        .map_err(FFIError::from)
        .and_then(|json| CString::new(json).map_err(FFIError::from))
    {
        Ok(json) => DashSDKResult::success_string(json.into_raw()),
        Err(error) => DashSDKResult::error(error.into()),
    }
}

fn invalid_parameter(message: &str) -> DashSDKResult {
    DashSDKResult::error(DashSDKError::new(
        DashSDKErrorCode::InvalidParameter,
        message.to_owned(),
    ))
}

/// Mark an informational check as pending and return its generation.
/// This is synchronous, makes no network request, and never restricts operations.
///
/// # Safety
/// `sdk_handle` must be live and `out_generation` must point to writable u64 storage.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_begin_protocol_compatibility_check(
    sdk_handle: *const SDKHandle,
    out_generation: *mut u64,
) -> DashSDKResult {
    if sdk_handle.is_null() || out_generation.is_null() {
        return invalid_parameter("SDK handle or generation output is null");
    }
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    *out_generation = wrapper.sdk.begin_protocol_compatibility_check();
    DashSDKResult::success(std::ptr::null_mut())
}

/// Run the Rust-owned bounded informational check for `generation`.
/// The returned snapshot can have a newer generation if this check was superseded.
/// Network/check failures are represented by the snapshot, not fabricated success.
///
/// # Safety
/// `sdk_handle` must remain live until the blocking call returns. Free the successful
/// JSON string with `dash_sdk_string_free`, or the error with `dash_sdk_error_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_refresh_protocol_compatibility(
    sdk_handle: *const SDKHandle,
    generation: u64,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return invalid_parameter("SDK handle is null");
    }
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let snapshot = wrapper
        .runtime
        .block_on(wrapper.sdk.refresh_protocol_compatibility(generation));
    json_result(&CompatibilitySnapshot::from(snapshot))
}

/// Read clone-shared compatibility state without making a network request.
///
/// # Safety
/// `sdk_handle` must be live. Free the successful JSON string with
/// `dash_sdk_string_free`, or the error with `dash_sdk_error_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_get_protocol_compatibility(
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return invalid_parameter("SDK handle is null");
    }
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    json_result(&CompatibilitySnapshot::from(
        wrapper.sdk.protocol_compatibility(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn should_preserve_snapshot_status_versions_and_string_ownership() {
        let result = json_result(&CompatibilitySnapshot::from(ProtocolCompatibility {
            generation: 7,
            state: ProtocolCompatibilityState::Checked,
            verified_protocol_version: Some(99),
            effective_protocol_version: 13,
            maximum_supported_protocol_version: 14,
        }));
        assert!(result.error.is_null());
        let text = unsafe { CStr::from_ptr(result.data.cast()) };
        let value: serde_json::Value = serde_json::from_slice(text.to_bytes()).expect("snapshot");
        assert_eq!(value["status"], "checked");
        assert_eq!(value["verifiedProtocolVersion"], 99);
        assert_eq!(value["generation"], 7);
        unsafe { crate::dash_sdk_string_free(result.data.cast()) };
    }

    #[test]
    fn should_reject_null_handles_without_touching_outputs() {
        let mut generation = 42;
        let results = unsafe {
            [
                dash_sdk_begin_protocol_compatibility_check(std::ptr::null(), &mut generation),
                dash_sdk_refresh_protocol_compatibility(std::ptr::null(), 0),
                dash_sdk_get_protocol_compatibility(std::ptr::null()),
            ]
        };
        assert_eq!(generation, 42);
        for result in results {
            assert!(!result.error.is_null());
            assert_eq!(
                unsafe { (*result.error).code },
                DashSDKErrorCode::InvalidParameter
            );
            unsafe { crate::dash_sdk_error_free(result.error) };
        }
    }

    #[test]
    fn should_publish_pending_check_generation_synchronously() {
        use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};
        let handle = create_mock_sdk_handle();
        unsafe {
            let mut generation = 0;
            let begin = dash_sdk_begin_protocol_compatibility_check(handle, &mut generation);
            assert!(begin.error.is_null());
            assert!(generation > 0);

            let snapshot = dash_sdk_get_protocol_compatibility(handle);
            assert!(snapshot.error.is_null());
            let json: serde_json::Value =
                serde_json::from_slice(CStr::from_ptr(snapshot.data.cast()).to_bytes())
                    .expect("snapshot");
            assert_eq!(json["generation"], generation);
            assert_eq!(json["status"], "checking");
            crate::dash_sdk_string_free(snapshot.data.cast());

            let invalid = dash_sdk_begin_protocol_compatibility_check(handle, std::ptr::null_mut());
            assert!(!invalid.error.is_null());
            assert_eq!((*invalid.error).code, DashSDKErrorCode::InvalidParameter);
            crate::dash_sdk_error_free(invalid.error);
            destroy_mock_sdk_handle(handle);
        }
    }
}
