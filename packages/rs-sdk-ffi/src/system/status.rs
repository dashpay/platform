//! SDK status query

use serde_json::json;
use std::ffi::CString;
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};

/// Get SDK status including mode and quorum count
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_get_status(sdk_handle: *const SDKHandle) -> DashSDKResult {
    eprintln!("🔵 dash_sdk_get_status: Called");

    if sdk_handle.is_null() {
        eprintln!("❌ dash_sdk_get_status: SDK handle is null");
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    eprintln!("🔵 dash_sdk_get_status: Got SDK wrapper");

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
        eprintln!(
            "🔵 dash_sdk_get_status: Got quorum count from trusted provider: {}",
            count
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
            eprintln!("❌ dash_sdk_get_status: Failed to serialize status: {}", e);
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to serialize status: {}", e),
            ));
        }
    };

    let c_str = match CString::new(json_str) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ dash_sdk_get_status: Failed to create CString: {}", e);
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create CString: {}", e),
            ));
        }
    };

    eprintln!("✅ dash_sdk_get_status: Success");
    DashSDKResult::success_string(c_str.into_raw())
}
