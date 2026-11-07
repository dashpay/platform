//! SDK status query

use dash_context_provider::routing::AdaptiveContextProviderSource;
use serde_json::json;
use std::ffi::CString;

use crate::sdk::{quorum_source, SDKWrapper};
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};

/// Get SDK status as JSON: `version`, `network`, `mode`, `quorumSource` and
/// `quorumCount`.
///
/// - `mode` is the context provider the SDK was built with: `"trusted"`,
///   `"spv"`, `"adaptive"`, or `"custom"` for a provider this crate did not
///   build (caller-supplied or mock).
/// - `quorumSource` is the source answering quorum lookups now: `"trusted"`
///   or `"spv"`. An adaptive SDK reports its live routing choice. It is
///   `null` for a custom provider.
/// - `quorumCount` is the number of quorums in the trusted provider's cache,
///   whichever source answers; `0` without a trusted provider.
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

    let network_str = wrapper.sdk.network.to_string();

    // An adaptive SDK also holds a trusted provider, so it is checked first,
    // in the order `quorum_source` uses.
    let mode = if wrapper.adaptive_provider.is_some() {
        "adaptive"
    } else if wrapper.trusted_provider.is_some() {
        "trusted"
    } else if wrapper.spv_source_slot.is_some() {
        "spv"
    } else {
        "custom"
    };
    let quorum_source = quorum_source(wrapper).map(|source| match source {
        AdaptiveContextProviderSource::Trusted => "trusted",
        AdaptiveContextProviderSource::Spv => "spv",
    });

    let quorum_count = wrapper
        .trusted_provider
        .as_ref()
        .map_or(0, |provider| provider.get_cached_quorum_count());
    tracing::debug!(
        mode,
        quorum_source,
        quorum_count,
        "dash_sdk_get_status: quorum sources"
    );

    let status = json!({
        "version": env!("CARGO_PKG_VERSION"),
        "network": network_str,
        "mode": mode,
        "quorumSource": quorum_source,
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

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};

    use dash_context_provider::routing::ContextProviderMode;
    use serde_json::Value;

    use super::dash_sdk_get_status;
    use crate::sdk::SDKWrapper;
    use crate::types::{DashSDKConfig, Network, SDKHandle};
    use crate::{
        dash_sdk_create_adaptive, dash_sdk_create_spv, dash_sdk_create_trusted, dash_sdk_destroy,
        dash_sdk_error_free, dash_sdk_set_context_provider_mode, dash_sdk_string_free,
        DashSDKResult,
    };

    /// A fixed-provider SDK built from `create`. Nothing connects: the DAPI
    /// address and the quorum URL are loopback and only parsed.
    fn sdk(
        create: unsafe extern "C" fn(*const DashSDKConfig) -> DashSDKResult,
        network: Network,
        quorum_url: Option<&CString>,
    ) -> *mut SDKHandle {
        let dapi = CString::new("http://127.0.0.1:1").expect("address");
        let config = DashSDKConfig {
            network: network.into(),
            dapi_addresses: dapi.as_ptr(),
            skip_asset_lock_proof_verification: false,
            request_retry_count: 1,
            request_timeout_ms: 1_000,
            quorum_url: quorum_url.map_or(std::ptr::null(), |url| url.as_ptr()),
            platform_version: 0,
        };
        let result = unsafe { create(&config) };
        if !result.error.is_null() {
            let message = unsafe { CStr::from_ptr((*result.error).message) }
                .to_string_lossy()
                .into_owned();
            unsafe { dash_sdk_error_free(result.error) };
            panic!("SDK failed to build: {message}");
        }
        result.data as *mut SDKHandle
    }

    fn status(handle: *const SDKHandle) -> Value {
        let result = unsafe { dash_sdk_get_status(handle) };
        if !result.error.is_null() {
            let message = unsafe { CStr::from_ptr((*result.error).message) }
                .to_string_lossy()
                .into_owned();
            unsafe { dash_sdk_error_free(result.error) };
            panic!("status failed: {message}");
        }
        assert!(!result.data.is_null(), "status returned no JSON");
        let json = result.data as *mut std::os::raw::c_char;
        let status = serde_json::from_str(
            unsafe { CStr::from_ptr(json) }
                .to_str()
                .expect("UTF-8 status"),
        )
        .expect("JSON status");
        unsafe { dash_sdk_string_free(json) };
        status
    }

    /// An adaptive SDK holds a trusted provider but routes quorum lookups
    /// between it and SPV at run time. Its status must say it is adaptive and
    /// which source answers now, not report it as trusted for holding a
    /// trusted provider.
    #[test]
    fn should_report_an_adaptive_sdk_and_the_source_it_routes_to() {
        let quorum_url = CString::new("http://127.0.0.1:1").expect("quorum URL");
        let handle = sdk(
            dash_sdk_create_adaptive,
            Network::Regtest,
            Some(&quorum_url),
        );

        let auto = status(handle);
        assert_eq!(auto["mode"], "adaptive");
        assert_eq!(
            auto["quorumSource"], "trusted",
            "Auto routes to the trusted provider until an SPV source is ready"
        );

        let set =
            unsafe { dash_sdk_set_context_provider_mode(handle, ContextProviderMode::Spv as u8) };
        assert!(set.error.is_null(), "setting the mode must succeed");
        let spv = status(handle);
        assert_eq!(spv["mode"], "adaptive");
        assert_eq!(spv["quorumSource"], "spv");

        unsafe { dash_sdk_destroy(handle) };
    }

    /// Trusted and strict SPV SDKs each have one source. An SDK whose
    /// provider the caller supplied has no source this crate can name.
    #[test]
    fn should_report_the_fixed_sources_and_a_caller_supplied_provider() {
        let quorum_url = CString::new("http://127.0.0.1:1").expect("quorum URL");
        let trusted = sdk(dash_sdk_create_trusted, Network::Regtest, Some(&quorum_url));
        let status_of_trusted = status(trusted);
        assert_eq!(status_of_trusted["mode"], "trusted");
        assert_eq!(status_of_trusted["quorumSource"], "trusted");
        unsafe { dash_sdk_destroy(trusted) };

        let spv = sdk(dash_sdk_create_spv, Network::Devnet, None);
        let status_of_spv = status(spv);
        assert_eq!(status_of_spv["mode"], "spv");
        assert_eq!(status_of_spv["quorumSource"], "spv");
        assert_eq!(status_of_spv["quorumCount"], 0);
        unsafe { dash_sdk_destroy(spv) };

        let custom = Box::into_raw(Box::new(SDKWrapper::new_mock())) as *mut SDKHandle;
        let status_of_custom = status(custom);
        assert_eq!(status_of_custom["mode"], "custom");
        assert!(status_of_custom["quorumSource"].is_null());
        unsafe { dash_sdk_destroy(custom) };
    }
}
