//! SDK initialization and configuration

use std::sync::Arc;
use tokio::runtime::Runtime;

use dash_sdk::dpp::dashcore::Network;
use dash_sdk::sdk::AddressList;
use dash_sdk::{Sdk, SdkBuilder};
use std::ffi::CStr;
use std::str::FromStr;

use crate::types::{DashSDKConfig, DashSDKNetwork, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

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

    #[cfg(test)]
    pub fn new_mock() -> Self {
        let runtime = Runtime::new().expect("Failed to create runtime");
        // Create a mock SDK using the mock builder
        let sdk = SdkBuilder::new_mock()
            .build()
            .expect("Failed to create test SDK");
        SDKWrapper::new(sdk, runtime)
    }
}

/// Create a new SDK instance
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_create(config: *const DashSDKConfig) -> DashSDKResult {
    if config.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Config is null".to_string(),
        ));
    }

    let config = &*config;

    // Parse configuration
    let network = match config.network {
        DashSDKNetwork::Mainnet => Network::Dash,
        DashSDKNetwork::Testnet => Network::Testnet,
        DashSDKNetwork::Devnet => Network::Devnet,
        DashSDKNetwork::Local => Network::Regtest,
    };

    // Create runtime
    let runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create runtime: {}", e),
            ));
        }
    };

    // Parse DAPI addresses
    let builder = if config.dapi_addresses.is_null() {
        // Use mock SDK if no addresses provided
        SdkBuilder::new_mock().with_network(network)
    } else {
        let addresses_str = match unsafe { CStr::from_ptr(config.dapi_addresses) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid DAPI addresses string: {}", e),
                ))
            }
        };

        if addresses_str.is_empty() {
            // Use mock SDK if addresses string is empty
            SdkBuilder::new_mock().with_network(network)
        } else {
            // Parse the address list
            let address_list = match AddressList::from_str(addresses_str) {
                Ok(list) => list,
                Err(e) => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Failed to parse DAPI addresses: {}", e),
                    ))
                }
            };

            SdkBuilder::new(address_list).with_network(network)
        }
    };

    // Build SDK
    let sdk_result = runtime.block_on(async { builder.build().map_err(FFIError::from) });

    match sdk_result {
        Ok(sdk) => {
            let wrapper = Box::new(SDKWrapper::new(sdk, runtime));
            let handle = Box::into_raw(wrapper) as *mut SDKHandle;
            DashSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Destroy an SDK instance
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_destroy(handle: *mut SDKHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut SDKWrapper);
    }
}

/// Get the current network the SDK is connected to
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_get_network(handle: *const SDKHandle) -> DashSDKNetwork {
    if handle.is_null() {
        return DashSDKNetwork::Mainnet;
    }

    let wrapper = &*(handle as *const SDKWrapper);
    match wrapper.sdk.network {
        Network::Dash => DashSDKNetwork::Mainnet,
        Network::Testnet => DashSDKNetwork::Testnet,
        Network::Devnet => DashSDKNetwork::Devnet,
        Network::Regtest => DashSDKNetwork::Local,
        _ => DashSDKNetwork::Local, // Fallback for any other network types
    }
}
