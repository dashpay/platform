//! SDK initialization and configuration

use std::sync::Arc;
use tokio::runtime::Runtime;

use dash_sdk::sdk::AddressList;
use dash_sdk::{Sdk, SdkBuilder};
use dpp::dashcore::Network;
use std::ffi::CStr;
use std::str::FromStr;

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

    // Parse DAPI addresses
    let builder = if config.dapi_addresses.is_null() {
        // Use mock SDK if no addresses provided
        SdkBuilder::new_mock().with_network(network)
    } else {
        let addresses_str = match unsafe { CStr::from_ptr(config.dapi_addresses) }.to_str() {
            Ok(s) => s,
            Err(e) => {
                return IOSSDKResult::error(IOSSDKError::new(
                    IOSSDKErrorCode::InvalidParameter,
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
                    return IOSSDKResult::error(IOSSDKError::new(
                        IOSSDKErrorCode::InvalidParameter,
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
