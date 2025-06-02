//! SDK initialization and configuration

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use tokio::runtime::Runtime;

use dash_sdk::platform::{Fetch, FetchMany};
use dash_sdk::{Sdk, SdkBuilder};
use dpp::dashcore::Network;

use crate::types::{IOSSDKConfig, IOSSDKNetwork, SDKHandle};
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};

/// Internal SDK wrapper
pub(crate) struct SDKWrapper {
    pub sdk: Sdk,
    pub runtime: Arc<Runtime>,
}

impl SDKWrapper {
    fn new(sdk: Sdk, runtime: Runtime) -> Self {
        SDKWrapper {
            sdk,
            runtime: Arc::new(runtime),
        }
    }
}

/// Create a new SDK instance
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_create(config: *const IOSSDKConfig) -> IOSSDKResult {
    if config.is_null() {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Config is null".to_string(),
        ));
    }

    let config = &*config;

    // Parse configuration
    let network = match config.network {
        IOSSDKNetwork::Mainnet => Network::Dash,
        IOSSDKNetwork::Testnet => Network::Testnet,
        IOSSDKNetwork::Devnet => Network::Devnet,
        IOSSDKNetwork::Local => Network::Regtest,
    };

    // Create runtime
    let runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            return IOSSDKResult::error(IOSSDKError::new(
                IOSSDKErrorCode::InternalError,
                format!("Failed to create runtime: {}", e),
            ));
        }
    };

    // Build SDK
    let sdk_result = runtime.block_on(async {
        // For simplicity, use mock SDK for now
        // In production, you'd want to pass actual DAPI endpoints
        let builder = SdkBuilder::new_mock().with_network(network);

        builder.build().map_err(FFIError::from)
    });

    match sdk_result {
        Ok(sdk) => {
            let wrapper = Box::new(SDKWrapper::new(sdk, runtime));
            let handle = Box::into_raw(wrapper) as *mut SDKHandle;
            IOSSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}

/// Destroy an SDK instance
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_destroy(handle: *mut SDKHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut SDKWrapper);
    }
}

/// Get the current network the SDK is connected to
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_get_network(handle: *const SDKHandle) -> IOSSDKNetwork {
    if handle.is_null() {
        return IOSSDKNetwork::Mainnet;
    }

    let wrapper = &*(handle as *const SDKWrapper);
    match wrapper.sdk.network {
        Network::Dash => IOSSDKNetwork::Mainnet,
        Network::Testnet => IOSSDKNetwork::Testnet,
        Network::Devnet => IOSSDKNetwork::Devnet,
        Network::Regtest => IOSSDKNetwork::Local,
        _ => IOSSDKNetwork::Local, // Fallback for any other network types
    }
}
