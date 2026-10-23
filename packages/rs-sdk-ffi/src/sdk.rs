//! SDK initialization and configuration

use std::sync::{Arc, OnceLock};
use tokio::runtime::Runtime;
use tracing::{error, info, warn};

use dash_sdk::dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted;
use dash_sdk::sdk::AddressList;
use dash_sdk::{Sdk, SdkBuilder};
use std::ffi::CStr;
use std::str::FromStr;

use dash_context_provider::routing::{
    AdaptiveContextProvider, AdaptiveContextProviderSource, ContextProviderMode, ContractSource,
    SpvContextProvider, SpvSourceSlot,
};
use dash_context_provider::ContextProvider;
use rs_sdk_trusted_context_provider::{LocalContextResolver, TrustedHttpContextProvider};

use crate::context_provider::{ContextProviderHandle, ContextProviderWrapper, CoreSDKHandle};
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
///
/// A single `SDKHandle` is used from several threads at once (the Swift SDK
/// runs Platform queries concurrently off the caller's actor), so every FFI
/// entry point that is handed a live handle must borrow it shared:
/// `&*(handle as *const SDKWrapper)`. Nothing here needs `&mut` once the
/// wrapper is built: `Sdk` is `Send + Sync`, `runtime` is an `Arc`, and the
/// trusted provider mutates only through its own interior locks. Taking
/// `&mut SDKWrapper` from a handle another thread may be using would alias a
/// live shared borrow, which is undefined behaviour even if nothing is
/// written. The one exclusive operation is `dash_sdk_destroy`, whose contract
/// is that the caller has already stopped every other use of the handle.
pub(crate) struct SDKWrapper {
    pub sdk: Sdk,
    pub runtime: Arc<BigStackRuntime>,
    pub trusted_provider: Option<Arc<TrustedHttpContextProvider>>,
    pub adaptive_provider: Option<Arc<AdaptiveContextProvider>>,
    pub spv_source_slot: Option<Arc<SpvSourceSlot>>,
    /// The contracts and token configurations this SDK's provider reads, for
    /// the FFIs that look contracts up or preload them. `None` when the
    /// provider is the caller's own.
    pub local_contracts: Option<Arc<LocalContextResolver>>,
}

impl SDKWrapper {
    fn new(sdk: Sdk, runtime: Runtime) -> Self {
        SDKWrapper {
            sdk,
            runtime: Arc::new(BigStackRuntime::new(runtime)),
            trusted_provider: None,
            adaptive_provider: None,
            spv_source_slot: None,
            local_contracts: None,
        }
    }

    #[allow(dead_code)]
    fn new_with_trusted_provider(
        sdk: Sdk,
        runtime: Runtime,
        provider: Arc<TrustedHttpContextProvider>,
    ) -> Self {
        SDKWrapper {
            sdk,
            runtime: Arc::new(BigStackRuntime::new(runtime)),
            local_contracts: Some(provider.local_resolver()),
            trusted_provider: Some(provider),
            adaptive_provider: None,
            spv_source_slot: None,
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
            spv_source_slot: None,
            local_contracts: None,
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
                spv_source_slot: None,
                local_contracts: None,
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
                spv_source_slot: None,
                local_contracts: None,
            });
            let handle = Box::into_raw(wrapper) as *mut SDKHandle;
            DashSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FixedContextProviderKind {
    Trusted,
    Spv,
    Adaptive,
}

struct FixedProviderParts {
    provider: Arc<dyn ContextProvider>,
    trusted: Option<Arc<TrustedHttpContextProvider>>,
    adaptive: Option<Arc<AdaptiveContextProvider>>,
    spv_source_slot: Option<Arc<SpvSourceSlot>>,
    local_contracts: Arc<LocalContextResolver>,
}

/// Create an SDK with a fixed trusted context provider.
///
/// # Safety
/// `config` must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_create_trusted(config: *const DashSDKConfig) -> DashSDKResult {
    dash_sdk_create_with_fixed_provider(config, FixedContextProviderKind::Trusted)
}

/// Create an SDK with a fixed production SPV context provider.
///
/// Quorum lookups fail closed until a wallet manager publishes its SPV client
/// and the client has synced. Contracts and tokens come from a local cache and
/// the embedded system contracts. The trusted quorum service is never built,
/// so `quorum_url` is ignored.
///
/// # Safety
/// `config` must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_create_spv(config: *const DashSDKConfig) -> DashSDKResult {
    dash_sdk_create_with_fixed_provider(config, FixedContextProviderKind::Spv)
}

/// Create an SDK with a fixed adaptive context provider.
///
/// # Safety
/// `config` must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_create_adaptive(config: *const DashSDKConfig) -> DashSDKResult {
    dash_sdk_create_with_fixed_provider(config, FixedContextProviderKind::Adaptive)
}

unsafe fn dash_sdk_create_with_fixed_provider(
    config: *const DashSDKConfig,
    provider_kind: FixedContextProviderKind,
) -> DashSDKResult {
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

    info!(?network, ?provider_kind, "creating SDK context provider");

    // Parse DAPI addresses. Every fixed-provider SDK needs real addresses.
    // Devnet/regtest have no built-in defaults; callers must supply
    // `dapi_addresses` (and typically `quorum_url`) for those networks.
    let builder = if config.dapi_addresses.is_null() {
        info!("no DAPI addresses provided, using defaults for network");
        match network {
            Network::Testnet => SdkBuilder::new_testnet(),
            Network::Mainnet => SdkBuilder::new_mainnet(),
            _ => {
                error!(?network, "no DAPI addresses for network");
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
            error!("empty DAPI addresses provided");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "DAPI addresses cannot be empty".to_string(),
            ));
        } else {
            info!(addresses = addresses_str, "using provided DAPI addresses");
            // Parse the address list
            let address_list = match AddressList::from_str(addresses_str) {
                Ok(list) => {
                    info!("successfully parsed DAPI addresses");
                    list
                }
                Err(e) => {
                    error!(error = %e, "failed to parse DAPI addresses");
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Failed to parse DAPI addresses: {}", e),
                    ));
                }
            };

            SdkBuilder::new(address_list).with_network(network)
        }
    };

    let provider_parts = match fixed_provider_parts(config, network, provider_kind) {
        Ok(parts) => parts,
        Err(error) => return DashSDKResult::error(error),
    };

    info!(
        ?provider_kind,
        "adding fixed context provider to SDK builder"
    );
    let builder = builder.with_context_provider_arc(provider_parts.provider);

    let builder = match apply_version(builder, config.platform_version) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(e),
    };

    // Build SDK
    let sdk_result = builder.build().map_err(FFIError::from);

    match sdk_result {
        Ok(sdk) => {
            // Trusted and Adaptive may route quorum lookups to the trusted
            // service, so warm its cache; strict SPV has no trusted provider.
            if let Some(trusted) = &provider_parts.trusted {
                let provider_for_prefetch = Arc::clone(trusted);
                let runtime_clone = runtime.handle().clone();
                runtime_clone.spawn(async move {
                    // Quorum key fetches that come in while this runs share it.
                    match provider_for_prefetch.refresh_quorum_caches().await {
                        Ok(_) => info!("successfully prefetched trusted quorums"),
                        Err(e) => {
                            warn!(error = %e, "failed to prefetch trusted quorums; continuing")
                        }
                    }
                });
            }

            let wrapper = Box::new(SDKWrapper {
                sdk,
                runtime,
                trusted_provider: provider_parts.trusted,
                adaptive_provider: provider_parts.adaptive,
                spv_source_slot: provider_parts.spv_source_slot,
                local_contracts: Some(provider_parts.local_contracts),
            });
            let handle = Box::into_raw(wrapper) as *mut SDKHandle;
            DashSDKResult::success(handle as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Build the context provider of `kind` and the pieces the SDK wrapper keeps.
///
/// Trusted and Adaptive read contracts from the trusted provider's own local
/// cache, so contracts preloaded through the wrapper and contracts the
/// provider verifies proofs with are one set. Strict SPV gets a standalone
/// cache and never builds the trusted provider.
unsafe fn fixed_provider_parts(
    config: &DashSDKConfig,
    network: Network,
    kind: FixedContextProviderKind,
) -> Result<FixedProviderParts, DashSDKError> {
    if kind == FixedContextProviderKind::Spv {
        let local_contracts = Arc::new(LocalContextResolver::new());
        let provider = Arc::new(SpvContextProvider::new(
            Arc::clone(&local_contracts) as Arc<dyn ContractSource>,
            network,
        ));
        return Ok(FixedProviderParts {
            spv_source_slot: Some(provider.source_slot()),
            provider,
            trusted: None,
            adaptive: None,
            local_contracts,
        });
    }

    // See `build_trusted_provider` for how the quorum lookup base URL is chosen.
    let trusted = Arc::new(build_trusted_provider(
        network,
        explicit_quorum_url(config)?,
    )?);
    let local_contracts = trusted.local_resolver();
    if kind == FixedContextProviderKind::Trusted {
        return Ok(FixedProviderParts {
            provider: Arc::clone(&trusted) as Arc<dyn ContextProvider>,
            trusted: Some(trusted),
            adaptive: None,
            spv_source_slot: None,
            local_contracts,
        });
    }
    let adaptive = Arc::new(AdaptiveContextProvider::new(
        Arc::clone(&trusted) as Arc<dyn ContextProvider>,
        Arc::clone(&local_contracts) as Arc<dyn ContractSource>,
        network,
    ));
    Ok(FixedProviderParts {
        provider: Arc::clone(&adaptive) as Arc<dyn ContextProvider>,
        trusted: Some(trusted),
        spv_source_slot: Some(adaptive.source_slot()),
        adaptive: Some(adaptive),
        local_contracts,
    })
}

/// The caller's quorum lookup base URL; an empty string means none.
unsafe fn explicit_quorum_url(config: &DashSDKConfig) -> Result<Option<String>, DashSDKError> {
    if config.quorum_url.is_null() {
        return Ok(None);
    }
    match CStr::from_ptr(config.quorum_url).to_str() {
        Ok(s) if !s.is_empty() => Ok(Some(s.to_string())),
        Ok(_) => Ok(None),
        Err(e) => Err(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!("Invalid quorum URL string: {}", e),
        )),
    }
}

/// Quorum and contract cache size of the trusted context provider.
const TRUSTED_PROVIDER_CACHE_SIZE: std::num::NonZeroUsize = match std::num::NonZeroUsize::new(100) {
    Some(size) => size,
    None => panic!("the cache size is not zero"),
};

/// Build the trusted context provider of [`dash_sdk_create_trusted`].
///
/// The quorum lookup base URL is, in order: the caller's `quorum_url`
/// (required for devnet, also usable for non-default mainnet/testnet shards),
/// the local quorum sidecar `127.0.0.1:22444` on regtest (the dashmate Docker
/// default), or the network's default.
///
/// Its synchronous lookup never fetches. A quorum key newer than the cache is
/// fetched asynchronously by the SDK, which then verifies the response it
/// already holds again: proof verification never blocks on the network, and
/// concurrent misses share one refresh.
pub(crate) fn build_trusted_provider(
    network: Network,
    quorum_url: Option<String>,
) -> Result<TrustedHttpContextProvider, DashSDKError> {
    let cache_size = TRUSTED_PROVIDER_CACHE_SIZE;
    let provider = if let Some(quorum_url) = quorum_url {
        info!(
            quorum_url = %quorum_url,
            "dash_sdk_create_trusted: using caller-provided quorum URL"
        );
        rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new_with_url(
            network, quorum_url, cache_size,
        )
        .map_err(|e| {
            error!(error = %e, "dash_sdk_create_trusted: failed to create context provider from override URL");
            DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create context provider: {}", e),
            )
        })?
    } else if matches!(network, Network::Regtest) {
        info!("dash_sdk_create_trusted: using local quorum sidecar for regtest");
        let provider = rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new_with_url(
            network,
            "http://127.0.0.1:22444".to_string(),
            cache_size,
        )
        .map_err(|e| {
            error!(error = %e, "dash_sdk_create_trusted: failed to create local context provider");
            DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create local context provider: {}", e),
            )
        })?;
        info!("dash_sdk_create_trusted: local trusted context provider created");
        provider
    } else {
        let provider = rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new(
            network, None, // Use default quorum lookup endpoints
            cache_size,
        )
        .map_err(|e| {
            error!(error = %e, "dash_sdk_create_trusted: failed to create trusted context provider");
            DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create trusted context provider: {}", e),
            )
        })?;
        info!("dash_sdk_create_trusted: trusted context provider created");
        provider
    };
    Ok(provider.with_refetch_if_not_found(false))
}

/// Destroy an SDK instance
/// # Safety
/// - `handle` must be a valid pointer previously returned by this SDK and not yet destroyed.
/// - It may be null (no-op). After this call the handle must not be used again.
/// - This is the only entry point that takes the wrapper exclusively, so no
///   other call on the same handle may be in flight or start concurrently.
///   The Swift SDK calls it only from `SDK.deinit`, which cannot run while a
///   query holds its `SDK` (`performBlockingQuery` retains `self` across the
///   FFI call).
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

/// The SPV source slot of an SDK built with a fixed SPV-capable provider, if
/// any. The returned `Arc` shares the slot with the provider.
///
/// # Safety
/// `sdk_handle` must be a valid SDK handle for the duration of this call.
pub unsafe fn dash_sdk_spv_source_slot(
    sdk_handle: *mut SDKHandle,
) -> Result<Option<Arc<SpvSourceSlot>>, String> {
    if sdk_handle.is_null() {
        return Err("SDK handle is null".to_string());
    }
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    Ok(wrapper.spv_source_slot.clone())
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

/// The source answering `wrapper`'s quorum lookups now: an adaptive
/// provider's live routing choice, else the one source a trusted or strict
/// SPV SDK has. `None` for a provider this crate did not build.
pub(crate) fn quorum_source(wrapper: &SDKWrapper) -> Option<AdaptiveContextProviderSource> {
    if let Some(provider) = &wrapper.adaptive_provider {
        // Evaluate readiness inside the SDK's multi-thread runtime so callers
        // outside Tokio (including Swift UI code) receive the live Auto source.
        Some(wrapper.runtime.block_on(async { provider.active_source() }))
    } else if wrapper.trusted_provider.is_some() {
        Some(AdaptiveContextProviderSource::Trusted)
    } else if wrapper.spv_source_slot.is_some() {
        Some(AdaptiveContextProviderSource::Spv)
    } else {
        None
    }
}

/// Return the currently selected quorum source for UI diagnostics. A
/// provider this crate did not build reports `Trusted`.
///
/// # Safety
/// `sdk_handle` must be a valid SDK handle for the duration of this call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_get_context_provider_source(sdk_handle: *const SDKHandle) -> u8 {
    if sdk_handle.is_null() {
        return AdaptiveContextProviderSource::Trusted as u8;
    }
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    quorum_source(wrapper).unwrap_or(AdaptiveContextProviderSource::Trusted) as u8
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

/// Add known contracts to the SDK's local contract cache
///
/// This preloads data contracts into the cache the SDK's context provider
/// reads, avoiding network calls for these contracts. A contract not newer
/// than the cached one with the same id is ignored, so a stale stored copy
/// cannot replace a newer contract.
///
/// # Safety
/// - `handle` must be a valid SDK handle created with `dash_sdk_create_trusted`,
///   `dash_sdk_create_spv` or `dash_sdk_create_adaptive`
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

    let local_contracts = match &wrapper.local_contracts {
        Some(local_contracts) => Arc::clone(local_contracts),
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidState,
                "SDK has no local contract cache. Create it with dash_sdk_create_trusted, dash_sdk_create_spv or dash_sdk_create_adaptive.".to_string(),
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
        match dash_sdk::dpp::data_contract::DataContract::versioned_deserialize_untrusted(
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
    local_contracts.add_known_contracts(contracts);

    eprintln!(
        "✅ Added {} known contracts to the local contract cache",
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

#[cfg(test)]
mod trusted_provider_tests {
    use super::*;
    use dash_sdk::platform::ContextProvider;

    /// iOS and Android verify proofs with this provider. A quorum key newer
    /// than its cache must be fetched asynchronously by the SDK, never by
    /// blocking inside proof verification, whichever way the quorum URL was
    /// chosen. The future is not polled: no request is made.
    #[test]
    fn should_build_a_provider_that_fetches_missing_quorum_keys_asynchronously() {
        for quorum_url in [Some("http://127.0.0.1:1".to_string()), None] {
            let provider = build_trusted_provider(Network::Regtest, quorum_url.clone())
                .unwrap_or_else(|_| panic!("provider for {quorum_url:?}"));
            assert!(
                provider
                    .fetch_quorum_public_key(106, [0x11; 32], 1)
                    .is_some(),
                "provider for {quorum_url:?} must fetch asynchronously"
            );
        }
    }
}

#[cfg(test)]
mod fixed_provider_tests {
    use super::*;
    use crate::dash_sdk_error_free;
    use dash_sdk::dpp::data_contract::accessors::v0::{
        DataContractV0Getters, DataContractV0Setters,
    };
    use dash_sdk::dpp::platform_value::string_encoding::Encoding;
    use dash_sdk::dpp::prelude::{DataContract, Identifier};
    use dash_sdk::dpp::serialization::PlatformSerializableWithPlatformVersion;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::dpp::version::PlatformVersion;
    use std::ffi::CString;

    fn config(network: Network, dapi: &CString, quorum_url: Option<&CString>) -> DashSDKConfig {
        DashSDKConfig {
            network: network.into(),
            dapi_addresses: dapi.as_ptr(),
            skip_asset_lock_proof_verification: false,
            request_retry_count: 1,
            request_timeout_ms: 1_000,
            quorum_url: quorum_url.map_or(std::ptr::null(), |url| url.as_ptr()),
            platform_version: 0,
        }
    }

    /// The DPNS contract under an id no system contract has.
    fn app_contract(id_byte: u8) -> DataContract {
        let mut contract =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("DPNS contract");
        contract.set_id(Identifier::new([id_byte; 32]));
        contract
    }

    /// A strict SPV SDK on devnet with no quorum URL. Nothing connects; the
    /// DAPI address is only parsed.
    fn strict_spv_sdk_without_a_quorum_service() -> *mut SDKHandle {
        let dapi = CString::new("http://127.0.0.1:1").expect("address");
        let config = config(Network::Devnet, &dapi, None);

        let result = unsafe { dash_sdk_create_spv(&config) };
        if !result.error.is_null() {
            let message = unsafe { CStr::from_ptr((*result.error).message) }
                .to_string_lossy()
                .into_owned();
            unsafe { dash_sdk_error_free(result.error) };
            panic!("strict SPV SDK failed to build: {message}");
        }
        result.data as *mut SDKHandle
    }

    /// Strict SPV never asks the trusted quorum service, so building it must
    /// not need one: a devnet SDK without a quorum URL builds, and it holds no
    /// trusted provider.
    #[test]
    fn should_build_a_strict_spv_sdk_without_a_quorum_service() {
        let handle = strict_spv_sdk_without_a_quorum_service();
        let wrapper = unsafe { &*(handle as *const SDKWrapper) };
        assert!(wrapper.trusted_provider.is_none());
        assert!(wrapper.spv_source_slot.is_some());
        assert!(wrapper.local_contracts.is_some());
        unsafe { dash_sdk_destroy(handle) };
    }

    /// Contracts preloaded through the FFI and contracts the provider verifies
    /// proofs with must be one set, for every provider kind; otherwise a
    /// preloaded contract is invisible to proof verification, and a contract
    /// the SDK registers is invisible to the document FFIs.
    #[test]
    fn should_share_one_contract_cache_between_preloading_and_proofs() {
        let dapi = CString::new("http://127.0.0.1:1").expect("address");
        let quorum_url = CString::new("http://127.0.0.1:1").expect("quorum url");
        for kind in [
            FixedContextProviderKind::Trusted,
            FixedContextProviderKind::Spv,
            FixedContextProviderKind::Adaptive,
        ] {
            let config = config(Network::Regtest, &dapi, Some(&quorum_url));
            let Ok(parts) = (unsafe { fixed_provider_parts(&config, Network::Regtest, kind) })
            else {
                panic!("{kind:?} provider failed to build");
            };

            let preloaded = app_contract(0x61);
            let preloaded_id = preloaded.id();
            parts.local_contracts.add_known_contract(preloaded);
            assert!(
                parts
                    .provider
                    .get_data_contract(&preloaded_id, PlatformVersion::latest())
                    .expect("lookup")
                    .is_some(),
                "{kind:?} provider must see a preloaded contract"
            );

            let registered = Arc::new(app_contract(0x62));
            let registered_id = registered.id();
            parts.provider.register_data_contract(registered);
            assert!(
                parts
                    .local_contracts
                    .get_known_contract(&registered_id)
                    .is_some(),
                "{kind:?} local cache must see a registered contract"
            );
        }
    }

    /// `dash_sdk_add_known_contracts` must reach the cache a strict SPV
    /// provider verifies proofs with; it has no trusted provider to fill.
    #[test]
    fn should_preload_contracts_into_a_strict_spv_sdk() {
        let handle = strict_spv_sdk_without_a_quorum_service();
        let wrapper = unsafe { &*(handle as *const SDKWrapper) };
        let version = wrapper.sdk.version();
        let mut contract =
            load_system_data_contract(SystemDataContract::DPNS, version).expect("DPNS contract");
        contract.set_id(Identifier::new([0x63; 32]));
        let id = contract.id();
        let bytes = contract
            .serialize_to_bytes_with_platform_version(version)
            .expect("serialized contract");
        let ids = CString::new(id.to_string(Encoding::Base58)).expect("contract id");
        let pointers = [bytes.as_ptr()];
        let lengths = [bytes.len()];

        let added = unsafe {
            dash_sdk_add_known_contracts(
                handle,
                ids.as_ptr(),
                pointers.as_ptr(),
                lengths.as_ptr(),
                1,
            )
        };
        assert!(added.error.is_null(), "preloading must succeed");
        let provider = wrapper.sdk.context_provider().expect("provider");
        assert!(provider
            .get_data_contract(&id, version)
            .expect("lookup")
            .is_some());
        unsafe { dash_sdk_destroy(handle) };
    }
}
