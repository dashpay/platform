//! SDK status query

use serde_json::json;
use std::ffi::CString;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};

/// Get SDK status including mode and quorum count
///
/// # Safety
/// - `sdk_handle` must be a valid pointer to an initialized SDKHandle.
/// - The returned C string pointer inside DashSDKResult (on success) must be freed by the caller
///   using the SDK's free routine to avoid memory leaks.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_get_status(sdk_handle: *const SDKHandle) -> DashSDKResult {
    tracing::info!("dash_sdk_get_status: called");

    if sdk_handle.is_null() {
        tracing::error!("dash_sdk_get_status: SDK handle is null");
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    tracing::debug!("dash_sdk_get_status: got SDK wrapper");

    // Get network
    let network_str = match wrapper.sdk.network {
        dash_sdk::dpp::dashcore::Network::Dash => "mainnet",
        dash_sdk::dpp::dashcore::Network::Testnet => "testnet",
        dash_sdk::dpp::dashcore::Network::Devnet => "devnet",
        dash_sdk::dpp::dashcore::Network::Regtest => "regtest",
        _ => "unknown",
    };

    // Determine mode based on whether we have a trusted provider
    let (mode, quorum_count) = if let Some(ref provider) = wrapper.trusted_provider {
        let count = provider.get_cached_quorum_count();
        tracing::debug!(
            quorum_count = count,
            "dash_sdk_get_status: trusted provider quorum count"
        );
        ("trusted", count)
    } else {
        // If no trusted provider, we're in SPV mode
        ("spv", 0)
    };

    // Create status JSON
    let status = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "network": network_str,
        "mode": mode,
        "quorumCount": quorum_count,
    });

    let json_str = match serde_json::to_string(&status) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "dash_sdk_get_status: failed to serialize status");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to serialize status: {}", e),
            ));
        }
    };

    let c_str = match CString::new(json_str) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "dash_sdk_get_status: failed to create CString");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create CString: {}", e),
            ));
        }
    };

    tracing::info!("dash_sdk_get_status: success");
    DashSDKResult::success_string(c_str.into_raw())
}
