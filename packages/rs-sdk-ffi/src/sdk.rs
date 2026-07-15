//! SDK initialization and configuration

use std::sync::{Arc, OnceLock};
use tokio::runtime::Runtime;
use tracing::{debug, error, info, warn};

use dash_sdk::dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use dash_sdk::sdk::AddressList;
use dash_sdk::{Sdk, SdkBuilder};
use std::ffi::CStr;
use std::str::FromStr;

use crate::context_provider::{
    AdaptiveContextProvider, ContextProviderHandle, ContextProviderMode, ContextProviderSource,
    ContextProviderWrapper, CoreSDKHandle,
};
use crate::runtime::BigStackRuntime;
use crate::types::{DashSDKConfig, FFINetwork, Network, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

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

fn apply_version(builder: SdkBuilder, platform_version: u32) -> Result<SdkBuilder, DashSDKError> {
    if platform_version == 0 {
        return Ok(builder);
    }
    match dash_sdk::dpp::version::PlatformVersion::get(platform_version) {
        Ok(v) => Ok(builder.with_version(v)),
        Err(e) => Err(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!("Invalid platform_version {}: {}", platform_version, e),
        )),
    }
}

/// Internal SDK wrapper
pub(crate) struct SDKWrapper {
    pub sdk: Sdk,
    pub runtime: Arc<BigStackRuntime>,
    pub trusted_provider: Option<Arc<rs_sdk_trusted_context_provider::TrustedHttpContextProvider>>,
    pub adaptive_provider: Option<Arc<AdaptiveContextProvider>>,
}

impl SDKWrapper {
    fn new(sdk: Sdk, runtime: Runtime) -> Self {
        SDKWrapper {
            sdk,
            runtime: Arc::new(BigStackRuntime::new(runtime)),
            trusted_provider: None,
            adaptive_provider: None,
        }
    }

    #[allow(dead_code)]
    fn new_with_trusted_provider(
        sdk: Sdk,
        runtime: Runtime,
        provider: Arc<rs_sdk_trusted_context_provider::TrustedHttpContextProvider>,
    ) -> Self {
        SDKWrapper {
            sdk,
            runtime: Arc::new(BigStackRuntime::new(runtime)),
            trusted_provider: Some(provider),
            adaptive_provider: None,
        }
    }

    #[cfg(test)]
    pub fn new_mock() -> Self {
        let runtime = init_or_get_runtime().expect("Failed to create runtime");
        // Create a mock SDK using the mock builder
        let sdk = SdkBuilder::new_mock()
            .build()
            .expect("Failed to create test SDK");
        SDKWrapper {
            sdk,
            runtime,
            trusted_provider: None,
            adaptive_provider: None,
        }
    }
}

// Shared Tokio runtime to avoid exhausting file descriptors when creating many SDK instances
static RUNTIME: OnceLock<Arc<BigStackRuntime>> = OnceLock::new();

fn init_or_get_runtime() -> Result<Arc<BigStackRuntime>, String> {
    if let Some(rt) = RUNTIME.get() {
        return Ok(rt.clone());
    }
    let rt =
        BigStackRuntime::build_shared().map_err(|e| format!("Failed to create runtime: {}", e))?;
    let arc = Arc::new(rt);
    let _ = RUNTIME.set(arc.clone());
    Ok(arc)
}

/// Create a new SDK instance
///
/// # Safety
/// - `config` must be a valid pointer to a DashSDKConfig structure for the duration of the call.
/// - The returned handle inside `DashSDKResult` must be destroyed using the SDK destroy function to avoid leaks.
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
    let network: Network = config.network.into();

    // Use shared runtime
    let runtime = match init_or_get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InternalError, e));
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

    let builder = match apply_version(builder, config.platform_version) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(e),
    };

    // Build SDK
    let sdk_result = builder.build().map_err(FFIError::from);

    match sdk_result {
        Ok(sdk) => {
            // Clone Arc<Runtime> into the wrapper
            let wrapper = Box::new(SDKWrapper {
                sdk,
                runtime,
                trusted_provider: None,
                adaptive_provider: None,
            });
            let handle = Box::into_raw(wrapper) as *mut SDKHandle;
            DashSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Create a new SDK instance with extended configuration including context provider
///
/// # Safety
/// - `config` must be a valid pointer to a DashSDKConfigExtended structure for the duration of the call.
/// - Any embedded pointers (context_provider/core_sdk_handle) must be valid when non-null.
/// - The returned handle inside `DashSDKResult` must be destroyed using the SDK destroy function to avoid leaks.
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
    let network: Network = base_config.network.into();

    // Use shared runtime
    let runtime = match init_or_get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InternalError, e));
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
        // Use registered global callbacks if available; otherwise return an error
        if let Some(callback_provider) =
            crate::context_callbacks::CallbackContextProvider::from_global()
        {
            builder = builder.with_context_provider(callback_provider);
        } else {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                "Failed to create context provider. Make sure to call dash_sdk_register_context_callbacks first.".to_string(),
            ));
        }
    } else {
        // No context provider specified - try to use global callbacks if available
        if let Some(callback_provider) =
            crate::context_callbacks::CallbackContextProvider::from_global()
        {
            builder = builder.with_context_provider(callback_provider);
        }
    }

    let builder = match apply_version(builder, base_config.platform_version) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(e),
    };

    // Build SDK
    let sdk_result = builder.build().map_err(FFIError::from);

    match sdk_result {
        Ok(sdk) => {
            let wrapper = Box::new(SDKWrapper {
                sdk,
                runtime,
                trusted_provider: None,
                adaptive_provider: None,
            });
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
/// # Safety
/// - `config` must be a valid pointer to a DashSDKConfig structure for the duration of the call.
/// - The returned handle inside `DashSDKResult` must be destroyed using the SDK destroy function to avoid leaks.
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
    let network: Network = config.network.into();

    // Use shared runtime
    let runtime = match init_or_get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InternalError, e));
        }
    };

    info!(
        ?network,
        "dash_sdk_create_trusted: creating trusted context provider"
    );

    // Create trusted context provider. Resolution order for the quorum
    // lookup base URL:
    //   1. Caller-provided `config.quorum_url` (highest priority — required
    //      for devnet, also usable for non-default mainnet/testnet shards).
    //   2. Regtest fallback to the local quorum sidecar `127.0.0.1:22444`
    //      (the dashmate Docker default).
    //   3. Network-derived default (mainnet/testnet only).
    let explicit_quorum_url: Option<String> = if config.quorum_url.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(config.quorum_url) }.to_str() {
            Ok(s) if !s.is_empty() => Some(s.to_string()),
            Ok(_) => None,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid quorum URL string: {}", e),
                ))
            }
        }
    };
    let trusted_provider = if let Some(quorum_url) = explicit_quorum_url {
        info!(
            quorum_url = %quorum_url,
            "dash_sdk_create_trusted: using caller-provided quorum URL"
        );
        match rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new_with_url(
            network,
            quorum_url,
            std::num::NonZeroUsize::new(100).unwrap(),
        ) {
            Ok(provider) => Arc::new(provider),
            Err(e) => {
                error!(error = %e, "dash_sdk_create_trusted: failed to create context provider from override URL");
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    format!("Failed to create context provider: {}", e),
                ));
            }
        }
    } else if matches!(network, Network::Regtest) {
        info!("dash_sdk_create_trusted: using local quorum sidecar for regtest");
        match rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new_with_url(
            network,
            "http://127.0.0.1:22444".to_string(),
            std::num::NonZeroUsize::new(100).unwrap(),
        ) {
            Ok(provider) => {
                info!("dash_sdk_create_trusted: local trusted context provider created");
                Arc::new(provider)
            }
            Err(e) => {
                error!(error = %e, "dash_sdk_create_trusted: failed to create local context provider");
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    format!("Failed to create local context provider: {}", e),
                ));
            }
        }
    } else {
        match rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new(
            network,
            None,                                      // Use default quorum lookup endpoints
            std::num::NonZeroUsize::new(100).unwrap(), // Cache size
        ) {
            Ok(provider) => {
                info!("dash_sdk_create_trusted: trusted context provider created");
                Arc::new(provider)
            }
            Err(e) => {
                error!(error = %e, "dash_sdk_create_trusted: failed to create trusted context provider");
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    format!("Failed to create trusted context provider: {}", e),
                ));
            }
        }
    };

    // Parse DAPI addresses - for trusted setup, we always need real addresses.
    // Devnet/regtest have no built-in defaults; callers must supply
    // `dapi_addresses` (and typically `quorum_url`) for those networks.
    let builder = if config.dapi_addresses.is_null() {
        info!("dash_sdk_create_trusted: no DAPI addresses provided, using defaults for network");
        match network {
            Network::Testnet => SdkBuilder::new_testnet(),
            Network::Mainnet => SdkBuilder::new_mainnet(),
            _ => {
                error!(
                    ?network,
                    "dash_sdk_create_trusted: no DAPI addresses for network"
                );
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
            error!("dash_sdk_create_trusted: empty DAPI addresses provided");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "DAPI addresses cannot be empty for trusted setup".to_string(),
            ));
        } else {
            info!(
                addresses = addresses_str,
                "dash_sdk_create_trusted: using provided DAPI addresses"
            );
            // Parse the address list
            let address_list = match AddressList::from_str(addresses_str) {
                Ok(list) => {
                    info!("dash_sdk_create_trusted: successfully parsed addresses");
                    list
                }
                Err(e) => {
                    error!(error = %e, "dash_sdk_create_trusted: failed to parse addresses");
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Failed to parse DAPI addresses: {}", e),
                    ));
                }
            };

            SdkBuilder::new(address_list).with_network(network)
        }
    };

    // Clone trusted provider for prefetching quorums
    let provider_for_prefetch = Arc::clone(&trusted_provider);
    let provider_for_wrapper = Arc::clone(&trusted_provider);
    let adaptive_provider = Arc::new(AdaptiveContextProvider::new(
        Arc::clone(&trusted_provider) as Arc<dyn drive_proof_verifier::ContextProvider>,
        network,
    ));

    // Install one adaptive provider for the SDK's entire lifetime.
    info!("dash_sdk_create_trusted: adding adaptive context provider to builder");
    let builder = builder.with_context_provider_arc(
        Arc::clone(&adaptive_provider) as Arc<dyn drive_proof_verifier::ContextProvider>
    );

    let builder = match apply_version(builder, config.platform_version) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(e),
    };

    // Build SDK
    let sdk_result = builder.build().map_err(FFIError::from);

    match sdk_result {
        Ok(sdk) => {
            // Prefetch quorums for trusted setup
            info!("dash_sdk_create_trusted: SDK built, prefetching quorums...");

            let runtime_clone = runtime.handle().clone();
            runtime_clone.spawn(async move {
                // First, try a simple HTTP test
                debug!("dash_sdk_create_trusted: testing basic HTTP connectivity");
                match reqwest::get("https://www.google.com").await {
                    Ok(_) => debug!("dash_sdk_create_trusted: basic HTTP test successful (Google)"),
                    Err(e) => warn!(error = %e, "dash_sdk_create_trusted: basic HTTP test failed"),
                }

                // Try the quorums endpoint directly
                debug!("dash_sdk_create_trusted: testing quorums endpoint directly");
                match reqwest::get("https://quorums.testnet.networks.dash.org/quorums").await {
                    Ok(resp) => debug!(status = %resp.status(), "dash_sdk_create_trusted: direct quorums endpoint test successful"),
                    Err(e) => warn!(error = %e, "dash_sdk_create_trusted: direct quorums endpoint test failed"),
                }

                // Now try through the provider
                match provider_for_prefetch.update_quorum_caches().await {
                    Ok(_) => info!("dash_sdk_create_trusted: successfully prefetched quorums"),
                    Err(e) => warn!(error = %e, "dash_sdk_create_trusted: failed to prefetch quorums; continuing"),
                }
            });

            let wrapper = Box::new(SDKWrapper {
                sdk,
                runtime,
                trusted_provider: Some(provider_for_wrapper),
                adaptive_provider: Some(adaptive_provider),
            });
            let handle = Box::into_raw(wrapper) as *mut SDKHandle;
            DashSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Destroy an SDK instance
/// # Safety
/// - `handle` must be a valid pointer previously returned by this SDK and not yet destroyed.
/// - It may be null (no-op). After this call the handle must not be used again.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_destroy(handle: *mut SDKHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut SDKWrapper);
    }
}

/// Get a raw pointer to the inner SDK struct from an SDK handle.
///
/// Returns an opaque pointer valid as long as the SDK handle is alive.
/// Used by platform-wallet-ffi to create a `PlatformWalletManager`.
///
/// # Safety
/// - `handle` must be a valid, non-null SDK handle.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_get_inner_sdk_ptr(
    handle: *const SDKHandle,
) -> *const std::os::raw::c_void {
    if handle.is_null() {
        return std::ptr::null();
    }
    let wrapper = &*(handle as *const SDKWrapper);
    &wrapper.sdk as *const dash_sdk::Sdk as *const std::os::raw::c_void
}

/// Populate the fixed adaptive provider's SPV source.
///
/// This changes internal adaptive state only; the SDK context provider itself
/// is never replaced.
///
/// # Safety
/// `sdk_handle` must be a valid SDK handle for the duration of this call.
pub unsafe fn dash_sdk_set_spv_source(
    sdk_handle: *mut SDKHandle,
    provider: Arc<dyn drive_proof_verifier::ContextProvider>,
    is_ready: Arc<dyn Fn() -> bool + Send + Sync>,
) -> Result<(), String> {
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let adaptive = wrapper
        .adaptive_provider
        .as_ref()
        .ok_or_else(|| "SDK does not use an adaptive context provider".to_string())?;
    adaptive
        .set_spv_source(provider, is_ready)
        .map_err(str::to_string)
}

/// Set the adaptive quorum routing mode without replacing the SDK provider.
///
/// # Safety
/// `sdk_handle` must be a valid SDK handle for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_set_context_provider_mode(
    sdk_handle: *mut SDKHandle,
    mode: u8,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }
    let mode = match ContextProviderMode::try_from(mode) {
        Ok(mode) => mode,
        Err(()) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid context provider mode: {mode}"),
            ));
        }
    };
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    match &wrapper.adaptive_provider {
        Some(provider) => {
            provider.set_mode(mode);
            DashSDKResult::success(std::ptr::null_mut())
        }
        None => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK does not use an adaptive context provider".to_string(),
        )),
    }
}

/// Return the currently selected quorum source for UI diagnostics.
///
/// # Safety
/// `sdk_handle` must be a valid SDK handle for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_get_context_provider_source(sdk_handle: *const SDKHandle) -> u8 {
    if sdk_handle.is_null() {
        return ContextProviderSource::Trusted as u8;
    }
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    wrapper
        .adaptive_provider
        .as_ref()
        .map_or(ContextProviderSource::Trusted as u8, |provider| {
            provider.active_source() as u8
        })
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
    match crate::context_callbacks::set_global_callbacks(
        crate::context_callbacks::ContextProviderCallbacks {
            core_handle: callbacks.core_handle,
            get_platform_activation_height: callbacks.get_platform_activation_height,
            get_quorum_public_key: callbacks.get_quorum_public_key,
        },
    ) {
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
        },
    );

    let wrapper = Box::new(ContextProviderWrapper::new(context_provider));
    let context_provider_handle = Box::into_raw(wrapper) as *mut ContextProviderHandle;

    // Read fields from the caller's config individually rather than copying
    // the struct (which would duplicate the raw `dapi_addresses` pointer).
    // The pointer is only borrowed for the duration of this call; the
    // downstream `dash_sdk_create_extended` reads and copies the string data
    // immediately before returning.
    let config_ref = &*config;
    let extended_config = DashSDKConfigExtended {
        base_config: DashSDKConfig {
            network: config_ref.network,
            dapi_addresses: config_ref.dapi_addresses,
            skip_asset_lock_proof_verification: config_ref.skip_asset_lock_proof_verification,
            request_retry_count: config_ref.request_retry_count,
            request_timeout_ms: config_ref.request_timeout_ms,
            quorum_url: config_ref.quorum_url,
            platform_version: config_ref.platform_version,
        },
        context_provider: context_provider_handle,
        core_sdk_handle: std::ptr::null_mut(),
    };

    // Use the extended creation function
    let result = dash_sdk_create_extended(&extended_config);

    // Reclaim the context provider wrapper - the SDK has already cloned what it needs
    // via `provider_wrapper.provider()` inside `dash_sdk_create_extended`.
    let _ = Box::from_raw(context_provider_handle as *mut ContextProviderWrapper);

    result
}

/// Get the current network the SDK is connected to
///
/// # Safety
/// - `handle` must be a valid pointer to an SDKHandle (or null, in which case a default is returned).
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_get_network(handle: *const SDKHandle) -> FFINetwork {
    if handle.is_null() {
        return FFINetwork::Mainnet;
    }

    let wrapper = &*(handle as *const SDKWrapper);
    wrapper.sdk.network.into()
}

/// Add known contracts to the SDK's trusted context provider
///
/// This allows pre-loading data contracts into the trusted provider's cache,
/// avoiding network calls for these contracts.
///
/// # Safety
/// - `handle` must be a valid SDK handle created with dash_sdk_create_trusted
/// - `contract_ids` must be a valid comma-separated list of contract IDs
/// - `serialized_contracts` must be a valid pointer to an array of serialized contract data
/// - `contract_lengths` must be a valid pointer to an array of contract data lengths
/// - `contract_count` must match the actual number of contracts provided
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_add_known_contracts(
    handle: *const SDKHandle,
    contract_ids: *const std::os::raw::c_char,
    serialized_contracts: *const *const u8,
    contract_lengths: *const usize,
    contract_count: usize,
) -> DashSDKResult {
    if handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if contract_ids.is_null() || serialized_contracts.is_null() || contract_lengths.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Invalid parameters".to_string(),
        ));
    }

    let wrapper = &*(handle as *const SDKWrapper);

    // Check if this SDK has a trusted provider
    let provider = match &wrapper.trusted_provider {
        Some(p) => p.clone(),
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidState,
                "SDK does not have a trusted context provider. Use dash_sdk_create_trusted to create an SDK with trusted provider.".to_string(),
            ));
        }
    };

    // Parse contract IDs
    let ids_str = match CStr::from_ptr(contract_ids).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid contract IDs string: {}", e),
            ));
        }
    };

    let ids: Vec<&str> = ids_str.split(',').map(|s| s.trim()).collect();

    if ids.len() != contract_count {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "Contract ID count mismatch: expected {}, got {}",
                contract_count,
                ids.len()
            ),
        ));
    }

    // Deserialize and add contracts
    let mut contracts = Vec::new();
    for (i, id) in ids.iter().take(contract_count).enumerate() {
        let contract_data =
            std::slice::from_raw_parts(*serialized_contracts.add(i), *contract_lengths.add(i));

        // Deserialize the contract using DPP
        let platform_version = wrapper.sdk.version();
        match dash_sdk::dpp::data_contract::DataContract::versioned_deserialize(
            contract_data,
            false, // don't validate (we trust the data)
            platform_version,
        ) {
            Ok(contract) => {
                eprintln!("✅ Successfully deserialized contract: {}", id);
                contracts.push(contract);
            }
            Err(e) => {
                eprintln!("❌ Failed to deserialize contract {}: {}", id, e);
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::SerializationError,
                    format!("Failed to deserialize contract {}: {}", id, e),
                ));
            }
        }
    }

    // Add all contracts to the provider
    provider.add_known_contracts(contracts);

    eprintln!(
        "✅ Added {} known contracts to trusted provider",
        contract_count
    );

    DashSDKResult::success(std::ptr::null_mut())
}

/// Create a mock SDK instance with a dump directory (for offline testing)
///
/// # Safety
/// - `dump_dir` must be either null (no dumps) or a valid pointer to a NUL-terminated C string readable for the duration of the call.
/// - The returned handle must be destroyed using the SDK destroy function to avoid leaks.
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
        eprintln!(
            "🔵 dash_sdk_create_handle_with_mock: loading mock vectors from {}",
            path.display()
        );
        builder = builder.with_dump_dir(&path);
    }

    // Build SDK inside the runtime context to satisfy any async initialization paths
    let _guard = runtime.enter();
    let sdk_result = builder.build();

    match sdk_result {
        Ok(sdk) => {
            let wrapper = Box::new(SDKWrapper::new(sdk, runtime));
            Box::into_raw(wrapper) as *mut SDKHandle
        }
        Err(e) => {
            eprintln!(
                "❌ dash_sdk_create_handle_with_mock: failed to build mock SDK: {}",
                e
            );
            std::ptr::null_mut()
        }
    }
}
