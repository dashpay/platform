use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use std::ffi::CString;

/// Refresh the SDK's protocol version from the connected network.
///
/// Issues an ordinary **proven** `getEpochsInfo` query and ratchets this SDK's
/// auto-detected protocol version up to the network's version through the same
/// proof + quorum-signature-verified path every other query uses — no unverified
/// fallback (see [`dash_sdk::Sdk::refresh_protocol_version`]). The resulting
/// protocol version number propagates to every clone of this SDK — including
/// the `Sdk` clone held by a `PlatformWalletManager` — because the version is
/// stored in a shared `Arc<AtomicU32>`.
///
/// Call this on app start and after every network switch so fee-sensitive
/// flows (shielded pool shield/unshield/transfer/withdraw) reserve against the
/// network's actual protocol version instead of the SDK's seed version.
///
/// As part of the refresh the SDK also best-effort probes the known DAPI peers
/// (unproven `getStatus`, concurrent, short timeout) and biases subsequent peer
/// selection towards nodes whose software already supports this client's latest
/// protocol version. Probe failures are ignored and no peer is ever excluded —
/// when no peer matches, selection falls back to any live peer.
///
/// For an SDK pinned to a fixed protocol version (version updating disabled)
/// the proven version query is skipped and the pinned version is returned
/// unchanged; the best-effort peer probe above still runs.
///
/// # Parameters
/// * `sdk_handle` - Handle to the SDK instance.
///
/// # Returns
/// * On success, a `DashSDKResult` carrying a heap-allocated C string with the
///   decimal protocol version number (e.g. `"12"`).
/// * On failure, a `DashSDKResult` carrying an error.
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer to an initialized `SDKHandle`.
/// - The function does not retain references to the input pointer beyond the duration of the call.
/// - On success, the returned `DashSDKResult` contains a heap-allocated C string; the caller must
///   free it using the SDK's string-free routine to avoid leaks.
/// - Passing a dangling or invalid pointer for `sdk_handle` results in undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_refresh_protocol_version(
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let result = wrapper
        .runtime
        .block_on(wrapper.sdk.refresh_protocol_version());

    match result {
        Ok(version) => match CString::new(version.to_string()) {
            Ok(c_str) => DashSDKResult::success_string(c_str.into_raw()),
            Err(e) => DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create CString: {}", e),
            )),
        },
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("Failed to refresh protocol version: {}", e),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refresh_protocol_version_null_handle() {
        unsafe {
            let result = dash_sdk_refresh_protocol_version(std::ptr::null());
            assert!(!result.error.is_null());
        }
    }
}
