//! SDK initialization and configuration

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use tokio::runtime::Runtime;

use dash_sdk::{Sdk, SdkBuilder};
use dash_sdk::platform::{Fetch, FetchMany};

use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult};
use crate::types::{SDKHandle, IOSSDKConfig, IOSSDKNetwork};

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
        IOSSDKNetwork::Mainnet => dash_sdk::Network::Dash,
        IOSSDKNetwork::Testnet => dash_sdk::Network::Testnet,
        IOSSDKNetwork::Devnet => dash_sdk::Network::Devnet,
        IOSSDKNetwork::Local => dash_sdk::Network::Regtest,
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
        let mut builder = SdkBuilder::new(network);
        
        // Configure wallet if provided
        if !config.wallet_mnemonic.is_null() {
            let mnemonic = match CStr::from_ptr(config.wallet_mnemonic).to_str() {
                Ok(s) => s,
                Err(e) => return Err(FFIError::from(e)),
            };
            
            let passphrase = if !config.wallet_passphrase.is_null() {
                match CStr::from_ptr(config.wallet_passphrase).to_str() {
                    Ok(s) => Some(s),
                    Err(e) => return Err(FFIError::from(e)),
                }
            } else {
                None
            };
            
            builder = builder.with_wallet(mnemonic, passphrase);
        }
        
        // Apply other settings
        builder = builder
            .with_skip_asset_lock_proof_verification(config.skip_asset_lock_proof_verification)
            .with_request_retry_count(config.request_retry_count as usize)
            .with_request_timeout(std::time::Duration::from_millis(config.request_timeout_ms));
        
        builder.build()
            .await
            .map_err(FFIError::from)
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
    match wrapper.sdk.network() {
        dash_sdk::Network::Dash => IOSSDKNetwork::Mainnet,
        dash_sdk::Network::Testnet => IOSSDKNetwork::Testnet,
        dash_sdk::Network::Devnet => IOSSDKNetwork::Devnet,
        dash_sdk::Network::Regtest => IOSSDKNetwork::Local,
        _ => IOSSDKNetwork::Local, // Fallback for any other network types
    }
}

/// Get wallet address (if wallet is configured)
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_get_wallet_address(handle: *const SDKHandle) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    
    let wrapper = &*(handle as *const SDKWrapper);
    
    // Get unused address from wallet
    let address = wrapper.runtime.block_on(async {
        match wrapper.sdk.wallet() {
            Some(wallet) => {
                match wallet.unused_address() {
                    Ok(addr) => Some(addr.to_string()),
                    Err(_) => None,
                }
            }
            None => None,
        }
    });
    
    match address {
        Some(addr) => {
            match CString::new(addr) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        None => std::ptr::null_mut(),
    }
}

/// Get wallet balance in credits
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_get_wallet_balance(handle: *const SDKHandle) -> u64 {
    if handle.is_null() {
        return 0;
    }
    
    let wrapper = &*(handle as *const SDKWrapper);
    
    wrapper.runtime.block_on(async {
        match wrapper.sdk.wallet() {
            Some(wallet) => {
                match wallet.balance() {
                    Ok(balance) => balance.total_balance(),
                    Err(_) => 0,
                }
            }
            None => 0,
        }
    })
}

/// Refresh wallet state (sync with network)
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_refresh_wallet(handle: *mut SDKHandle) -> *mut IOSSDKError {
    if handle.is_null() {
        return Box::into_raw(Box::new(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "Handle is null".to_string(),
        )));
    }
    
    let wrapper = &mut *(handle as *mut SDKWrapper);
    
    let result = wrapper.runtime.block_on(async {
        match wrapper.sdk.wallet() {
            Some(wallet) => wallet.reload_utxos()
                .await
                .map_err(|e| FFIError::InternalError(e.to_string())),
            None => Err(FFIError::InvalidState("No wallet configured".to_string())),
        }
    });
    
    match result {
        Ok(_) => std::ptr::null_mut(),
        Err(e) => Box::into_raw(Box::new(e.into())),
    }
}