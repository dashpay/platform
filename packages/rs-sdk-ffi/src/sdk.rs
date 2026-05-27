//! SDK initialization and configuration

use std::sync::{Arc, OnceLock};
use tokio::runtime::Runtime;
use tracing::{debug, error, info, warn};

use dash_sdk::dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::sdk::AddressList;
use dash_sdk::{RequestSettings, Sdk, SdkBuilder};
use std::ffi::CStr;
use std::str::FromStr;
use std::time::Duration;

use crate::context_provider::{ContextProviderHandle, ContextProviderWrapper, CoreSDKHandle};
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
    /// Optional Platform protocol version to pin the SDK to.
    ///
    /// `0` preserves the default auto-detect behavior; any non-zero value
    /// must correspond to a known `PlatformVersion` or SDK creation fails
    /// with `InvalidParameter`. Callers that zero-initialize the struct
    /// (e.g., Swift's `DashSDKConfigExtended()` or C `{0}` initializer)
    /// automatically inherit the auto-detect default.
    pub protocol_version: u32,
}

type TrustedProvider = Arc<rs_sdk_trusted_context_provider::TrustedHttpContextProvider>;

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

    #[allow(dead_code)]
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
        let runtime = init_or_get_runtime().expect("Failed to create runtime");
        // Create a mock SDK using the mock builder
        let sdk = SdkBuilder::new_mock()
            .build()
            .expect("Failed to create test SDK");
        SDKWrapper {
            sdk,
            runtime,
            trusted_provider: None,
        }
    }
}

// Shared Tokio runtime to avoid exhausting file descriptors when creating many SDK instances
static RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();

fn init_or_get_runtime() -> Result<Arc<Runtime>, String> {
    if let Some(rt) = RUNTIME.get() {
        return Ok(rt.clone());
    }
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder.thread_name("dash-sdk-worker");
    builder.worker_threads(1); // Reduce threads for mobile
    builder.enable_all();
    let rt = builder
        .build()
        .map_err(|e| format!("Failed to create runtime: {}", e))?;
    let arc = Arc::new(rt);
    let _ = RUNTIME.set(arc.clone());
    Ok(arc)
}

fn resolve_platform_version(
    protocol_version: u32,
) -> Result<Option<&'static PlatformVersion>, DashSDKResult> {
    if protocol_version == 0 {
        return Ok(None);
    }

    PlatformVersion::get(protocol_version)
        .map(Some)
        .map_err(|e| {
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Unknown protocol version {}: {}", protocol_version, e),
            ))
        })
}

fn parse_dapi_addresses(config: &DashSDKConfig) -> Result<Option<AddressList>, DashSDKResult> {
    if config.dapi_addresses.is_null() {
        return Ok(None);
    }

    let addresses_str = unsafe { CStr::from_ptr(config.dapi_addresses) }
        .to_str()
        .map_err(|e| {
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid DAPI addresses string: {}", e),
            ))
        })?;

    if addresses_str.is_empty() {
        return Ok(None);
    }

    AddressList::from_str(addresses_str).map(Some).map_err(|e| {
        DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!("Failed to parse DAPI addresses: {}", e),
        ))
    })
}

fn build_sdk_builder(config: &DashSDKConfig) -> Result<SdkBuilder, DashSDKResult> {
    let network: Network = config.network.into();
    let builder = match parse_dapi_addresses(config)? {
        Some(address_list) => SdkBuilder::new(address_list),
        None => SdkBuilder::new_mock(),
    };

    Ok(apply_common_builder_config(
        builder.with_network(network),
        config,
    ))
}

fn apply_common_builder_config(builder: SdkBuilder, config: &DashSDKConfig) -> SdkBuilder {
    // NOTE: `skip_asset_lock_proof_verification` toggles `SdkBuilder::with_proofs`,
    // which controls Platform state-proof verification for *all* SDK requests, not
    // just asset-lock proofs. The field is named for the original use case
    // (skipping asset-lock proofs in tests) but its effect is broader. See the
    // field's Rustdoc on `DashSDKConfig`.
    builder
        .with_proofs(!config.skip_asset_lock_proof_verification)
        .with_settings(RequestSettings {
            timeout: Some(Duration::from_millis(config.request_timeout_ms)),
            retries: Some(config.request_retry_count as usize),
            ..RequestSettings::default()
        })
}

fn apply_platform_version(
    builder: SdkBuilder,
    platform_version: Option<&'static PlatformVersion>,
) -> SdkBuilder {
    if let Some(platform_version) = platform_version {
        builder.with_version(platform_version)
    } else {
        builder
    }
}

fn apply_context_provider(
    mut builder: SdkBuilder,
    config: &DashSDKConfigExtended,
) -> Result<SdkBuilder, DashSDKResult> {
    if !config.context_provider.is_null() {
        let provider_wrapper =
            unsafe { &*(config.context_provider as *const ContextProviderWrapper) };
        builder = builder.with_context_provider(provider_wrapper.provider());
    } else if !config.core_sdk_handle.is_null() {
        // SAFETY: caller guarantees `core_sdk_handle` is either null or a
        // valid `CoreSDKHandle` pointer per `dash_sdk_create_extended`'s
        // safety contract; we have already checked it for null above.
        let callback_provider =
            match unsafe { crate::context_callbacks::CallbackContextProvider::from_core_sdk_handle(
                config.core_sdk_handle,
            ) } {
                Ok(callback_provider) => callback_provider,
                Err(
                    crate::context_callbacks::CallbackContextProviderCreateError::NullCoreSdkHandle
                    | crate::context_callbacks::CallbackContextProviderCreateError::NullCoreClientHandle,
                ) => {
                    return Err(DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        "Invalid core SDK handle: client pointer is null".to_string(),
                    )));
                }
                Err(
                    crate::context_callbacks::CallbackContextProviderCreateError::MissingGlobalCallbacks,
                ) => {
                    return Err(DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InternalError,
                        "Failed to create context provider. Make sure to call dash_sdk_register_context_callbacks first.".to_string(),
                    )));
                }
            };
        builder = builder.with_context_provider(callback_provider);
    } else if let Some(callback_provider) =
        crate::context_callbacks::CallbackContextProvider::from_global()
    {
        builder = builder.with_context_provider(callback_provider);
    }

    Ok(builder)
}

fn make_sdk_result(
    sdk: Sdk,
    runtime: Arc<Runtime>,
    trusted_provider: Option<TrustedProvider>,
) -> DashSDKResult {
    let wrapper = Box::new(SDKWrapper {
        sdk,
        runtime,
        trusted_provider,
    });
    let handle = Box::into_raw(wrapper) as *mut SDKHandle;
    DashSDKResult::success(handle as *mut std::os::raw::c_void)
}

fn create_sdk_from_config(
    config: &DashSDKConfig,
    platform_version: Option<&'static PlatformVersion>,
) -> DashSDKResult {
    let runtime = match init_or_get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InternalError, e));
        }
    };

    let builder = match build_sdk_builder(config) {
        Ok(builder) => builder,
        Err(result) => return result,
    };

    match apply_platform_version(builder, platform_version)
        .build()
        .map_err(FFIError::from)
    {
        Ok(sdk) => make_sdk_result(sdk, runtime, None),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

fn create_extended_sdk_from_config(
    config: &DashSDKConfigExtended,
    platform_version: Option<&'static PlatformVersion>,
) -> DashSDKResult {
    let runtime = match init_or_get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InternalError, e));
        }
    };

    let builder = match build_sdk_builder(&config.base_config) {
        Ok(builder) => builder,
        Err(result) => return result,
    };

    let builder = apply_platform_version(builder, platform_version);
    let builder = match apply_context_provider(builder, config) {
        Ok(builder) => builder,
        Err(result) => return result,
    };

    match builder.build().map_err(FFIError::from) {
        Ok(sdk) => make_sdk_result(sdk, runtime, None),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

fn build_trusted_sdk_builder(
    config: &DashSDKConfig,
) -> Result<(SdkBuilder, TrustedProvider), DashSDKResult> {
    let network: Network = config.network.into();

    info!(
        ?network,
        "dash_sdk_create_trusted: creating trusted context provider"
    );

    let is_local = matches!(network, Network::Regtest);
    let trusted_provider = if is_local {
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
                return Err(DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    format!("Failed to create local context provider: {}", e),
                )));
            }
        }
    } else {
        match rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new(
            network,
            None,
            std::num::NonZeroUsize::new(100).unwrap(),
        ) {
            Ok(provider) => {
                info!("dash_sdk_create_trusted: trusted context provider created");
                Arc::new(provider)
            }
            Err(e) => {
                error!(error = %e, "dash_sdk_create_trusted: failed to create trusted context provider");
                return Err(DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    format!("Failed to create trusted context provider: {}", e),
                )));
            }
        }
    };

    let builder = match parse_dapi_addresses(config)? {
        None => {
            info!(
                "dash_sdk_create_trusted: no DAPI addresses provided, using defaults for network"
            );
            match network {
                Network::Testnet => SdkBuilder::new_testnet(),
                Network::Mainnet => SdkBuilder::new_mainnet(),
                _ => {
                    error!(
                        ?network,
                        "dash_sdk_create_trusted: no DAPI addresses for network"
                    );
                    return Err(DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("DAPI addresses not available for network: {:?}", network),
                    )));
                }
            }
        }
        Some(address_list) => {
            info!("dash_sdk_create_trusted: using provided DAPI addresses");
            SdkBuilder::new(address_list).with_network(network)
        }
    };

    info!("dash_sdk_create_trusted: adding trusted context provider to builder");
    Ok((
        apply_common_builder_config(builder, config)
            .with_context_provider(Arc::clone(&trusted_provider)),
        trusted_provider,
    ))
}

fn spawn_trusted_quorum_prefetch(runtime: &Arc<Runtime>, trusted_provider: TrustedProvider) {
    info!("dash_sdk_create_trusted: SDK built, prefetching quorums...");

    runtime.handle().spawn(async move {
        debug!("dash_sdk_create_trusted: prefetching quorum caches");
        match trusted_provider.update_quorum_caches().await {
            Ok(_) => info!("dash_sdk_create_trusted: successfully prefetched quorums"),
            Err(e) => {
                warn!(error = %e, "dash_sdk_create_trusted: failed to prefetch quorums; continuing")
            }
        }
    });
}

fn create_trusted_sdk_from_config(
    config: &DashSDKConfig,
    platform_version: Option<&'static PlatformVersion>,
) -> DashSDKResult {
    let runtime = match init_or_get_runtime() {
        Ok(rt) => rt,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InternalError, e));
        }
    };

    let (builder, trusted_provider) = match build_trusted_sdk_builder(config) {
        Ok(parts) => parts,
        Err(result) => return result,
    };

    let provider_for_wrapper = Arc::clone(&trusted_provider);
    match apply_platform_version(builder, platform_version)
        .build()
        .map_err(FFIError::from)
    {
        Ok(sdk) => {
            spawn_trusted_quorum_prefetch(&runtime, trusted_provider);
            make_sdk_result(sdk, runtime, Some(provider_for_wrapper))
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
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

    create_sdk_from_config(&*config, None)
}

/// Create a new SDK instance with extended configuration including context provider.
///
/// Honors `config.protocol_version` (where `0` keeps the default auto-detect
/// behavior; any non-zero value pins Platform protocol version, returning
/// `InvalidParameter` if unknown).
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

    let config_ref = &*config;
    let platform_version = match resolve_platform_version(config_ref.protocol_version) {
        Ok(platform_version) => platform_version,
        Err(result) => return result,
    };

    create_extended_sdk_from_config(config_ref, platform_version)
}

/// Create a new SDK instance with trusted setup
///
/// This creates an SDK with a trusted context provider that fetches quorum keys and
/// data contracts from trusted endpoints instead of requiring proof verification.
///
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

    create_trusted_sdk_from_config(&*config, None)
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
        },
        context_provider: context_provider_handle,
        core_sdk_handle: std::ptr::null_mut(),
        protocol_version: 0,
    };

    let result = dash_sdk_create_extended(&extended_config);

    // Reclaim the context provider wrapper - the SDK has already cloned what it needs
    // via `provider_wrapper.provider()` inside `dash_sdk_create_extended`.
    let _ = Box::from_raw(context_provider_handle as *mut ContextProviderWrapper);

    result
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::{
        context_callbacks::{set_global_callbacks, CallbackResult, ContextProviderCallbacks},
        dash_sdk_create_extended, dash_sdk_destroy, dash_sdk_error_free,
        dash_sdk_get_inner_sdk_ptr, DashSDKErrorCode, FFINetwork,
    };
    use std::ffi::CString;
    use std::os::raw::c_void;

    unsafe fn read_error_message(error: *mut crate::DashSDKError) -> String {
        std::ffi::CStr::from_ptr((*error).message)
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn build_sdk_builder_applies_proofs_and_request_settings() {
        let config = DashSDKConfig {
            network: FFINetwork::Testnet,
            dapi_addresses: std::ptr::null(),
            skip_asset_lock_proof_verification: true,
            request_retry_count: 7,
            request_timeout_ms: 1_234,
        };

        let builder = match build_sdk_builder(&config) {
            Ok(builder) => builder,
            Err(_) => panic!("builder should be created"),
        };
        let sdk = builder.build().expect("mock SDK should build");

        assert!(!sdk.prove());
        assert_eq!(sdk.query_settings().request_settings.retries, Some(7));
        assert_eq!(
            sdk.query_settings().request_settings.timeout,
            Some(Duration::from_millis(1_234))
        );
    }

    #[test]
    fn build_trusted_sdk_builder_empty_addresses_use_network_defaults() {
        let dapi_addresses = CString::new("").expect("cstring");
        let config = DashSDKConfig {
            network: FFINetwork::Testnet,
            dapi_addresses: dapi_addresses.as_ptr(),
            skip_asset_lock_proof_verification: true,
            request_retry_count: 4,
            request_timeout_ms: 5_678,
        };

        let (builder, _trusted_provider) = match build_trusted_sdk_builder(&config) {
            Ok(parts) => parts,
            Err(_) => panic!("trusted builder should accept empty addresses"),
        };
        let sdk = builder.build().expect("trusted SDK builder should build");

        assert_eq!(sdk.network, Network::Testnet);
        assert!(!sdk.prove());
        assert_eq!(sdk.query_settings().request_settings.retries, Some(4));
        assert_eq!(
            sdk.query_settings().request_settings.timeout,
            Some(Duration::from_millis(5_678))
        );
    }

    #[test]
    fn dash_sdk_create_extended_rejects_null_core_client_handle() {
        extern "C" fn get_height_cb(_h: *mut c_void, out: *mut u32) -> CallbackResult {
            unsafe {
                if !out.is_null() {
                    *out = 0;
                }
            }
            CallbackResult {
                success: true,
                error_code: 0,
                error_message: std::ptr::null(),
            }
        }

        extern "C" fn get_quorum_pk_cb(
            _h: *mut c_void,
            _qt: u32,
            _qh: *const u8,
            _hgt: u32,
            out: *mut u8,
        ) -> CallbackResult {
            unsafe {
                if !out.is_null() {
                    std::ptr::write_bytes(out, 0, 48);
                }
            }
            CallbackResult {
                success: true,
                error_code: 0,
                error_message: std::ptr::null(),
            }
        }

        let _guard = crate::context_callbacks::test_support::lock_global_callbacks_for_test();

        unsafe {
            set_global_callbacks(ContextProviderCallbacks {
                core_handle: std::ptr::dangling_mut::<c_void>(),
                get_platform_activation_height: get_height_cb,
                get_quorum_public_key: get_quorum_pk_cb,
            })
            .expect("global callbacks should be set");
        }

        let mut core_sdk_handle = CoreSDKHandle {
            client: std::ptr::null_mut(),
        };
        let extended_config = DashSDKConfigExtended {
            base_config: DashSDKConfig {
                network: FFINetwork::Testnet,
                dapi_addresses: std::ptr::null(),
                skip_asset_lock_proof_verification: false,
                request_retry_count: 3,
                request_timeout_ms: 30_000,
            },
            context_provider: std::ptr::null_mut(),
            core_sdk_handle: &mut core_sdk_handle,
            protocol_version: 0,
        };

        let result = unsafe { dash_sdk_create_extended(&extended_config) };

        assert!(!result.error.is_null(), "expected invalid handle error");
        let error = unsafe { &*result.error };
        assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        let message = unsafe { read_error_message(result.error) };
        assert!(message.contains("Invalid core SDK handle"));

        unsafe {
            dash_sdk_error_free(result.error);
        }
    }

    #[test]
    fn dash_sdk_create_extended_pins_protocol_version_from_config_field() {
        let extended_config = DashSDKConfigExtended {
            base_config: DashSDKConfig {
                network: FFINetwork::Testnet,
                dapi_addresses: std::ptr::null(),
                skip_asset_lock_proof_verification: false,
                request_retry_count: 3,
                request_timeout_ms: 30_000,
            },
            context_provider: std::ptr::null_mut(),
            core_sdk_handle: std::ptr::null_mut(),
            protocol_version: 11,
        };

        let result = unsafe { dash_sdk_create_extended(&extended_config) };
        assert!(result.error.is_null(), "expected success");

        let handle = result.data as *mut SDKHandle;
        let inner_sdk_ptr = unsafe { dash_sdk_get_inner_sdk_ptr(handle) };
        assert!(!inner_sdk_ptr.is_null(), "expected inner sdk pointer");

        let sdk = unsafe { &*(inner_sdk_ptr as *const dash_sdk::Sdk) };
        assert_eq!(sdk.protocol_version_number(), 11);

        unsafe {
            dash_sdk_destroy(handle);
        }
    }

    #[test]
    fn dash_sdk_create_extended_rejects_invalid_protocol_version_from_config_field() {
        let extended_config = DashSDKConfigExtended {
            base_config: DashSDKConfig {
                network: FFINetwork::Testnet,
                dapi_addresses: std::ptr::null(),
                skip_asset_lock_proof_verification: false,
                request_retry_count: 3,
                request_timeout_ms: 30_000,
            },
            context_provider: std::ptr::null_mut(),
            core_sdk_handle: std::ptr::null_mut(),
            protocol_version: u32::MAX,
        };

        let result = unsafe { dash_sdk_create_extended(&extended_config) };
        assert!(!result.error.is_null(), "expected invalid parameter error");

        let error = unsafe { &*result.error };
        assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        let message = unsafe { read_error_message(result.error) };
        assert!(message.contains("Unknown protocol version"));

        unsafe {
            dash_sdk_error_free(result.error);
        }
    }
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
