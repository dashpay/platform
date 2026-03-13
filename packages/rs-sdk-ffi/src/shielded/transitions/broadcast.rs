//! Generic broadcast for pre-built shielded state transitions.

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::serialization::PlatformDeserializable;
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;

/// Broadcast a pre-built, serialized state transition.
///
/// Use this with the `dash_sdk_shielded_build_*` functions to build a transition
/// first, inspect or store it, then broadcast when ready.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `st_bytes`: Serialized state transition bytes (from a `build_*` function)
/// - `st_len`: Length of the serialized bytes
///
/// # Returns
/// `DashSDKResult` with no data on success, error on failure.
///
/// # Safety
/// - `sdk_handle` must be a valid SDK handle.
/// - `st_bytes` must point to `st_len` valid bytes of a serialized `StateTransition`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_broadcast(
    sdk_handle: *const SDKHandle,
    st_bytes: *const u8,
    st_len: usize,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if st_bytes.is_null() || st_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "State transition bytes are null or empty".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let bytes = std::slice::from_raw_parts(st_bytes, st_len);

    let state_transition = match StateTransition::deserialize_from_bytes(bytes) {
        Ok(st) => st,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Failed to deserialize state transition: {}", e),
            ))
        }
    };

    let result = wrapper.runtime.block_on(async {
        state_transition
            .broadcast(&wrapper.sdk, None)
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(()) => DashSDKResult::success(std::ptr::null_mut()),
        Err(e) => DashSDKResult::error(e.into()),
    }
}
