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
use crate::context_provider::{ContextProviderHandle, ContextProviderWrapper, CoreSDKHandle};

/// Extended SDK configuration with context provider support
#[repr(C)]
pub struct DashSDKConfigExtended {
    /// Base SDK configuration
    pub base_config: DashSDKConfig,
    /// Optional context provider handle
    pub context_provider: *mut ContextProviderHandle,
    /// Optional Core SDK handle for automatic context provider creation
    pub core_sdk_handle: *mut CoreSDKHandle,
}

/// Internal SDK wrapper
pub(crate) struct SDKWrapper {
    pub sdk: Sdk,
    pub runtime: Arc<Runtime>,
    pub trusted_provider: Option<Arc<rs_sdk_trusted_context_provider::TrustedHttpContextProvider>>,
}

impl SDKWrapper {
    fn new(sdk: Sdk, runtime: Runtime) -> Self {
        SDKWrapper {
            sdk,
            runtime: Arc::new(runtime),
            trusted_provider: None,
        }
    }

    fn new_with_trusted_provider(
        sdk: Sdk,
        runtime: Runtime,
        provider: Arc<rs_sdk_trusted_context_provider::TrustedHttpContextProvider>,
    ) -> Self {
        SDKWrapper {
            sdk,
            runtime: Arc::new(runtime),
            trusted_provider: Some(provider),
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
        DashSDKNetwork::SDKMainnet => Network::Dash,
        DashSDKNetwork::SDKTestnet => Network::Testnet,
        DashSDKNetwork::SDKRegtest => Network::Regtest,
        DashSDKNetwork::SDKDevnet => Network::Devnet,
        DashSDKNetwork::SDKLocal => Network::Regtest,
    };

    // Create runtime
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .thread_name("dash-sdk-worker")
        .worker_threads(1)  // Reduce threads for mobile
        .enable_all()
        .build() {
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
    let sdk_result = builder.build().map_err(FFIError::from);

    match sdk_result {
        Ok(sdk) => {
            let wrapper = Box::new(SDKWrapper::new(sdk, runtime));
            let handle = Box::into_raw(wrapper) as *mut SDKHandle;
            DashSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Create a new SDK instance with extended configuration including context provider
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_create_extended(
    config: *const DashSDKConfigExtended,
) -> DashSDKResult {
    if config.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Config is null".to_string(),
        ));
    }

    let config = &*config;
    let base_config = &config.base_config;

    // Parse configuration
    let network = match base_config.network {
        DashSDKNetwork::SDKMainnet => Network::Dash,
        DashSDKNetwork::SDKTestnet => Network::Testnet,
        DashSDKNetwork::SDKRegtest => Network::Regtest,
        DashSDKNetwork::SDKDevnet => Network::Devnet,
        DashSDKNetwork::SDKLocal => Network::Regtest,
    };

    // Create runtime
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .thread_name("dash-sdk-worker")
        .worker_threads(1)  // Reduce threads for mobile
        .enable_all()
        .build() {
        Ok(rt) => rt,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create runtime: {}", e),
            ));
        }
    };

    // Parse DAPI addresses
    let mut builder = if base_config.dapi_addresses.is_null() {
        // Use mock SDK if no addresses provided
        SdkBuilder::new_mock().with_network(network)
    } else {
        let addresses_str = match unsafe { CStr::from_ptr(base_config.dapi_addresses) }.to_str() {
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

    // Check if context provider is provided
    if !config.context_provider.is_null() {
        let provider_wrapper = &*(config.context_provider as *const ContextProviderWrapper);
        builder = builder.with_context_provider(provider_wrapper.provider());
    } else if !config.core_sdk_handle.is_null() {
        // Try to create context provider from global callbacks
        if let Some(callback_provider) = crate::context_callbacks::CallbackContextProvider::from_global() {
            builder = builder.with_context_provider(callback_provider);
        } else {
            // Fallback to deprecated method (which will also check for global callbacks)
            use crate::context_provider::dash_sdk_context_provider_from_core;
            
            let context_provider_handle = dash_sdk_context_provider_from_core(
                config.core_sdk_handle,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            );
            
            if context_provider_handle.is_null() {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    "Failed to create context provider. Make sure to call dash_sdk_register_context_callbacks first.".to_string(),
                ));
            }
            
            let provider_wrapper = &*(context_provider_handle as *const ContextProviderWrapper);
            builder = builder.with_context_provider(provider_wrapper.provider());
        }
    } else {
        // No context provider specified - try to use global callbacks if available
        if let Some(callback_provider) = crate::context_callbacks::CallbackContextProvider::from_global() {
            builder = builder.with_context_provider(callback_provider);
        }
    }

    // Build SDK
    let sdk_result = builder.build().map_err(FFIError::from);

    match sdk_result {
        Ok(sdk) => {
            let wrapper = Box::new(SDKWrapper::new(sdk, runtime));
            let handle = Box::into_raw(wrapper) as *mut SDKHandle;
            DashSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Create a new SDK instance with trusted setup
///
/// This creates an SDK with a trusted context provider that fetches quorum keys and
/// data contracts from trusted endpoints instead of requiring proof verification.
///
/// # Safety
/// - `config` must be a valid pointer to a DashSDKConfig structure
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_create_trusted(config: *const DashSDKConfig) -> DashSDKResult {
    if config.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Config is null".to_string(),
        ));
    }

    let config = &*config;

    // Parse configuration
    let network = match config.network {
        DashSDKNetwork::SDKMainnet => Network::Dash,
        DashSDKNetwork::SDKTestnet => Network::Testnet,
        DashSDKNetwork::SDKRegtest => Network::Regtest,
        DashSDKNetwork::SDKDevnet => Network::Devnet,
        DashSDKNetwork::SDKLocal => Network::Regtest,
    };

    // Create runtime
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .thread_name("dash-sdk-worker")
        .worker_threads(1)  // Reduce threads for mobile
        .enable_all()
        .build() {
        Ok(rt) => rt,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create runtime: {}", e),
            ));
        }
    };

    eprintln!("🔵 dash_sdk_create_trusted: Creating trusted context provider for network: {:?}", network);
    
    // Create trusted context provider
    let trusted_provider = match rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new(
        network,
        None,  // Use default quorum lookup endpoints
        std::num::NonZeroUsize::new(100).unwrap(),  // Cache size
    ) {
        Ok(provider) => {
            eprintln!("✅ dash_sdk_create_trusted: Trusted context provider created successfully");
            Arc::new(provider)
        },
        Err(e) => {
            eprintln!("❌ dash_sdk_create_trusted: Failed to create trusted context provider: {}", e);
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create trusted context provider: {}", e),
            ));
        }
    };

    // Parse DAPI addresses - for trusted setup, we always need real addresses
    let builder = if config.dapi_addresses.is_null() {
        eprintln!("🔵 dash_sdk_create_trusted: No DAPI addresses provided, using default addresses for network");
        // Use default addresses for the network
        match network {
            Network::Testnet => {
                // Use testnet addresses from WASM SDK
                let default_addresses = vec![
                    "https://52.12.176.90:1443",
                    "https://35.82.197.197:1443",
                    "https://44.240.98.102:1443",
                    "https://52.34.144.50:1443",
                    "https://44.239.39.153:1443",
                    "https://35.164.23.245:1443",
                    "https://54.149.33.167:1443",
                ].join(",");
                
                eprintln!("🔵 dash_sdk_create_trusted: Using default testnet addresses: {}", default_addresses);
                let address_list = match AddressList::from_str(&default_addresses) {
                    Ok(list) => list,
                    Err(e) => {
                        eprintln!("❌ dash_sdk_create_trusted: Failed to parse default addresses: {}", e);
                        return DashSDKResult::error(DashSDKError::new(
                            DashSDKErrorCode::InternalError,
                            format!("Failed to parse default addresses: {}", e),
                        ))
                    }
                };
                SdkBuilder::new(address_list).with_network(network)
            },
            Network::Dash => {
                // Use mainnet addresses from WASM SDK
                let default_addresses = vec![
                    "https://149.28.241.190:443",
                    "https://198.7.115.48:443",
                    "https://134.255.182.186:443",
                    "https://93.115.172.39:443",
                    "https://5.189.164.253:443",
                    "https://178.215.237.134:443",
                    "https://157.66.81.162:443",
                    "https://173.212.232.90:443",
                ].join(",");
                
                eprintln!("🔵 dash_sdk_create_trusted: Using default mainnet addresses");
                let address_list = match AddressList::from_str(&default_addresses) {
                    Ok(list) => list,
                    Err(e) => {
                        eprintln!("❌ dash_sdk_create_trusted: Failed to parse default addresses: {}", e);
                        return DashSDKResult::error(DashSDKError::new(
                            DashSDKErrorCode::InternalError,
                            format!("Failed to parse default addresses: {}", e),
                        ))
                    }
                };
                SdkBuilder::new(address_list).with_network(network)
            },
            _ => {
                eprintln!("❌ dash_sdk_create_trusted: No DAPI addresses for network: {:?}", network);
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("DAPI addresses not available for network: {:?}", network),
                ));
            }
        }
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
            eprintln!("❌ dash_sdk_create_trusted: Empty DAPI addresses provided");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "DAPI addresses cannot be empty for trusted setup".to_string(),
            ));
        } else {
            eprintln!("🔵 dash_sdk_create_trusted: Using provided DAPI addresses: {}", addresses_str);
            // Parse the address list
            let address_list = match AddressList::from_str(addresses_str) {
                Ok(list) => {
                    eprintln!("✅ dash_sdk_create_trusted: Successfully parsed addresses");
                    list
                },
                Err(e) => {
                    eprintln!("❌ dash_sdk_create_trusted: Failed to parse addresses: {}", e);
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Failed to parse DAPI addresses: {}", e),
                    ))
                }
            };

            SdkBuilder::new(address_list).with_network(network)
        }
    };

    // Clone trusted provider for prefetching quorums
    let provider_for_prefetch = Arc::clone(&trusted_provider);
    let provider_for_wrapper = Arc::clone(&trusted_provider);
    
    // Add trusted context provider
    eprintln!("🔵 dash_sdk_create_trusted: Adding trusted context provider to builder");
    let builder = builder.with_context_provider(Arc::clone(&trusted_provider));

    // Build SDK
    let sdk_result = builder.build().map_err(FFIError::from);

    match sdk_result {
        Ok(sdk) => {
            // Prefetch quorums for trusted setup
            eprintln!("🔵 dash_sdk_create_trusted: SDK built, prefetching quorums...");
            
            let runtime_clone = runtime.handle().clone();
            runtime_clone.spawn(async move {
                // First, try a simple HTTP test
                eprintln!("🔵 Testing basic HTTP connectivity...");
                match reqwest::get("https://www.google.com").await {
                    Ok(_) => eprintln!("✅ Basic HTTP test successful (Google)"),
                    Err(e) => eprintln!("❌ Basic HTTP test failed: {}", e),
                }
                
                // Try the quorums endpoint directly
                eprintln!("🔵 Testing quorums endpoint directly...");
                match reqwest::get("https://quorums.testnet.networks.dash.org/quorums").await {
                    Ok(resp) => eprintln!("✅ Direct quorums endpoint test successful, status: {}", resp.status()),
                    Err(e) => eprintln!("❌ Direct quorums endpoint test failed: {}", e),
                }
                
                // Now try through the provider
                match provider_for_prefetch.update_quorum_caches().await {
                    Ok(_) => eprintln!("✅ dash_sdk_create_trusted: Successfully prefetched quorums"),
                    Err(e) => eprintln!("⚠️ dash_sdk_create_trusted: Failed to prefetch quorums: {}. Continuing anyway.", e),
                }
            });
            
            let wrapper = Box::new(SDKWrapper::new_with_trusted_provider(sdk, runtime, provider_for_wrapper));
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

/// Register global context provider callbacks
///
/// This must be called before creating an SDK instance that needs Core SDK functionality.
/// The callbacks will be used by all SDK instances created after registration.
///
/// # Safety
/// - `callbacks` must contain valid function pointers that remain valid for the lifetime of the SDK
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_register_context_callbacks(
    callbacks: *const crate::context_callbacks::ContextProviderCallbacks,
) -> i32 {
    if callbacks.is_null() {
        return -1;
    }

    let callbacks = &*callbacks;
    match crate::context_callbacks::set_global_callbacks(crate::context_callbacks::ContextProviderCallbacks {
        core_handle: callbacks.core_handle,
        get_platform_activation_height: callbacks.get_platform_activation_height,
        get_quorum_public_key: callbacks.get_quorum_public_key,
    }) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Create a new SDK instance with explicit context callbacks
///
/// This is an alternative to registering global callbacks. The callbacks are used only for this SDK instance.
///
/// # Safety
/// - `config` must be a valid pointer to a DashSDKConfig structure
/// - `callbacks` must contain valid function pointers that remain valid for the lifetime of the SDK
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_create_with_callbacks(
    config: *const DashSDKConfig,
    callbacks: *const crate::context_callbacks::ContextProviderCallbacks,
) -> DashSDKResult {
    if config.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Config is null".to_string(),
        ));
    }

    if callbacks.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Callbacks is null".to_string(),
        ));
    }

    // Create extended config with callback-based context provider
    let callbacks = &*callbacks;
    let context_provider = crate::context_callbacks::CallbackContextProvider::new(
        crate::context_callbacks::ContextProviderCallbacks {
            core_handle: callbacks.core_handle,
            get_platform_activation_height: callbacks.get_platform_activation_height,
            get_quorum_public_key: callbacks.get_quorum_public_key,
        }
    );
    
    let wrapper = Box::new(ContextProviderWrapper::new(context_provider));
    let context_provider_handle = Box::into_raw(wrapper) as *mut ContextProviderHandle;
    
    let extended_config = DashSDKConfigExtended {
        base_config: *config,
        context_provider: context_provider_handle,
        core_sdk_handle: std::ptr::null_mut(),
    };
    
    // Use the extended creation function
    dash_sdk_create_extended(&extended_config)
}

/// Get the current network the SDK is connected to
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_get_network(handle: *const SDKHandle) -> DashSDKNetwork {
    if handle.is_null() {
        return DashSDKNetwork::SDKMainnet;
    }

    let wrapper = &*(handle as *const SDKWrapper);
    match wrapper.sdk.network {
        Network::Dash => DashSDKNetwork::SDKMainnet,
        Network::Testnet => DashSDKNetwork::SDKTestnet,
        Network::Regtest => DashSDKNetwork::SDKRegtest,
        Network::Devnet => DashSDKNetwork::SDKDevnet,
        _ => DashSDKNetwork::SDKLocal, // Fallback for any other network types
    }
}

/// Create a mock SDK instance with a dump directory (for offline testing)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_create_handle_with_mock(
    dump_dir: *const std::os::raw::c_char,
) -> *mut SDKHandle {
    // Create runtime
    let runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    // Parse dump directory
    let dump_dir_str = if dump_dir.is_null() {
        ""
    } else {
        match CStr::from_ptr(dump_dir).to_str() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        }
    };

    // Create mock SDK
    let mut builder = SdkBuilder::new_mock();

    if !dump_dir_str.is_empty() {
        let path = std::path::PathBuf::from(dump_dir_str);
        builder = builder.with_dump_dir(&path);
    }

    // Build SDK
    let sdk_result = builder.build();

    match sdk_result {
        Ok(sdk) => {
            let wrapper = Box::new(SDKWrapper::new(sdk, runtime));
            Box::into_raw(wrapper) as *mut SDKHandle
        }
        Err(_) => std::ptr::null_mut(),
    }
}
