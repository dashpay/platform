//! Context Provider FFI bindings
//!
//! This module provides FFI bindings for configuring context providers,
//! allowing the Platform SDK to connect to Core SDK for proof verification.

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Weak};

use arc_swap::ArcSwapOption;
use dash_sdk::dpp::data_contract::TokenConfiguration;
use dash_sdk::dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::error::ContextProviderError;
use drive_proof_verifier::ContextProvider;

use crate::context_callbacks::{CallbackContextProvider, ContextProviderCallbacks};
use crate::types::Network;

/// Handle for Core SDK that can be passed to Platform SDK
/// This matches the definition from dash_spv_ffi.h
#[repr(C)]
pub struct CoreSDKHandle {
    pub client: *mut std::ffi::c_void,
}

/// Opaque handle to a context provider
#[repr(C)]
pub struct ContextProviderHandle {
    _private: [u8; 0],
}

/// Internal wrapper for context provider
/// Adapter wrapping any [`ContextProvider`] as an opaque
/// [`ContextProviderHandle`] for callback-based SDK configuration.
pub struct ContextProviderWrapper {
    provider: Arc<dyn ContextProvider>,
}

impl ContextProviderWrapper {
    pub fn new(provider: impl ContextProvider + 'static) -> Self {
        Self {
            provider: Arc::new(provider),
        }
    }

    pub fn provider(&self) -> Arc<dyn ContextProvider> {
        Arc::clone(&self.provider)
    }
}

/// Runtime policy for quorum public-key resolution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ContextProviderMode {
    Auto = 0,
    Spv = 1,
    Trusted = 2,
}

impl TryFrom<u8> for ContextProviderMode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Auto),
            1 => Ok(Self::Spv),
            2 => Ok(Self::Trusted),
            _ => Err(()),
        }
    }
}

/// Quorum source selected for a lookup at the current instant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AdaptiveContextProviderSource {
    Trusted = 0,
    Spv = 1,
}

struct SpvContextSource {
    lease_id: u64,
    provider: Arc<dyn ContextProvider>,
    is_ready: Arc<dyn Fn() -> Result<bool, ContextProviderError> + Send + Sync>,
}

/// Owns the currently active manager-backed SPV source without changing the
/// SDK's context-provider identity.
pub struct SpvSourceController {
    source: ArcSwapOption<SpvContextSource>,
    next_lease_id: AtomicU64,
}

impl Default for SpvSourceController {
    fn default() -> Self {
        Self {
            source: ArcSwapOption::empty(),
            next_lease_id: AtomicU64::new(1),
        }
    }
}

impl SpvSourceController {
    /// Acquire the SPV source for one running wallet manager.
    ///
    /// Only an empty controller can be acquired. The returned lease clears the
    /// source on drop if it still owns the matching generation.
    pub fn acquire(
        self: &Arc<Self>,
        provider: Arc<dyn ContextProvider>,
        is_ready: Arc<dyn Fn() -> Result<bool, ContextProviderError> + Send + Sync>,
    ) -> Result<SpvSourceLease, &'static str> {
        let lease_id = self.next_lease_id.fetch_add(1, Ordering::Relaxed);
        let source = Arc::new(SpvContextSource {
            lease_id,
            provider,
            is_ready,
        });
        let previous = self
            .source
            .compare_and_swap(std::ptr::null::<SpvContextSource>(), Some(source));
        if previous.is_some() {
            Err("another wallet manager owns the SPV context source")
        } else {
            Ok(SpvSourceLease {
                controller: Arc::downgrade(self),
                lease_id,
            })
        }
    }

    fn release(&self, lease_id: u64) {
        self.source.rcu(|current| match current {
            Some(source) if source.lease_id == lease_id => None,
            _ => current.clone(),
        });
    }

    fn source(&self) -> Result<Arc<SpvContextSource>, ContextProviderError> {
        self.source.load_full().ok_or_else(|| {
            ContextProviderError::Generic("SPV context source is not active".to_string())
        })
    }

    fn ready_source(&self) -> Result<Arc<SpvContextSource>, ContextProviderError> {
        let source = self.source()?;
        if !(source.is_ready)()? {
            return Err(ContextProviderError::Generic(
                "SPV context source is not ready".to_string(),
            ));
        }
        Ok(source)
    }
}

/// Conditional ownership token for a manager-backed SPV source.
pub struct SpvSourceLease {
    controller: Weak<SpvSourceController>,
    lease_id: u64,
}

impl Drop for SpvSourceLease {
    fn drop(&mut self) {
        if let Some(controller) = self.controller.upgrade() {
            controller.release(self.lease_id);
        }
    }
}

/// Contract and token resolver shared by fixed SPV and adaptive providers.
///
/// This wrapper intentionally exposes no quorum-key method, so the production
/// SPV provider cannot accidentally fall back to trusted quorum data.
pub(crate) struct AuxiliaryContextProvider {
    provider: Arc<dyn ContextProvider>,
}

impl AuxiliaryContextProvider {
    pub(crate) fn new(provider: Arc<dyn ContextProvider>) -> Self {
        Self { provider }
    }

    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        self.provider.get_data_contract(id, platform_version)
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        self.provider.get_token_configuration(token_id)
    }
}

/// Fixed production provider: quorum keys come only from the active SPV
/// source, while contracts and tokens come from the local auxiliary resolver.
pub struct SpvContextProvider {
    spv: Arc<SpvSourceController>,
    auxiliary: Arc<AuxiliaryContextProvider>,
    network: Network,
}

impl SpvContextProvider {
    pub(crate) fn new(
        spv: Arc<SpvSourceController>,
        auxiliary: Arc<AuxiliaryContextProvider>,
        network: Network,
    ) -> Self {
        Self {
            spv,
            auxiliary,
            network,
        }
    }
}

impl ContextProvider for SpvContextProvider {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        self.spv.ready_source()?.provider.get_quorum_public_key(
            quorum_type,
            quorum_hash,
            core_chain_locked_height,
        )
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        activation_height(self.network)
    }

    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        self.auxiliary.get_data_contract(id, platform_version)
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        self.auxiliary.get_token_configuration(token_id)
    }
}

/// Context provider whose identity remains fixed while quorum routing adapts.
pub struct AdaptiveContextProvider {
    trusted: Arc<dyn ContextProvider>,
    spv: Arc<SpvSourceController>,
    auxiliary: Arc<AuxiliaryContextProvider>,
    mode: AtomicU8,
    network: Network,
}

impl AdaptiveContextProvider {
    pub(crate) fn new(
        trusted: Arc<dyn ContextProvider>,
        spv: Arc<SpvSourceController>,
        auxiliary: Arc<AuxiliaryContextProvider>,
        network: Network,
    ) -> Self {
        Self {
            trusted,
            spv,
            auxiliary,
            mode: AtomicU8::new(ContextProviderMode::Auto as u8),
            network,
        }
    }

    pub fn set_mode(&self, mode: ContextProviderMode) {
        self.mode.store(mode as u8, Ordering::Release);
    }

    pub fn mode(&self) -> ContextProviderMode {
        ContextProviderMode::try_from(self.mode.load(Ordering::Acquire))
            .expect("context-provider mode is only written from validated values")
    }

    pub fn active_source(&self) -> AdaptiveContextProviderSource {
        match self.mode() {
            ContextProviderMode::Trusted => AdaptiveContextProviderSource::Trusted,
            ContextProviderMode::Spv => AdaptiveContextProviderSource::Spv,
            ContextProviderMode::Auto if self.spv.ready_source().is_ok() => {
                AdaptiveContextProviderSource::Spv
            }
            ContextProviderMode::Auto => AdaptiveContextProviderSource::Trusted,
        }
    }
}

impl ContextProvider for AdaptiveContextProvider {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        match self.mode() {
            ContextProviderMode::Trusted => self.trusted.get_quorum_public_key(
                quorum_type,
                quorum_hash,
                core_chain_locked_height,
            ),
            ContextProviderMode::Spv => self.spv.ready_source()?.provider.get_quorum_public_key(
                quorum_type,
                quorum_hash,
                core_chain_locked_height,
            ),
            ContextProviderMode::Auto => match self.spv.ready_source() {
                Ok(source) => source.provider.get_quorum_public_key(
                    quorum_type,
                    quorum_hash,
                    core_chain_locked_height,
                ),
                Err(_) => self.trusted.get_quorum_public_key(
                    quorum_type,
                    quorum_hash,
                    core_chain_locked_height,
                ),
            },
        }
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        activation_height(self.network)
    }

    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        self.auxiliary.get_data_contract(id, platform_version)
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        self.auxiliary.get_token_configuration(token_id)
    }
}

fn activation_height(network: Network) -> Result<CoreBlockHeight, ContextProviderError> {
    match network {
        Network::Mainnet => Ok(2_132_092),
        Network::Testnet => Ok(1_090_319),
        Network::Devnet | Network::Regtest => Ok(1),
    }
}

// Note: Core SDK FFI types are opaque to rs-sdk-ffi and referenced via raw pointers.

// Note: Core SDK functions are now provided via callbacks instead of direct linking
// This allows Platform SDK to be built independently and linked at runtime

// Note: The deprecated CoreBridgeContextProvider has been removed.

/// Create a context provider from callbacks
///
/// # Safety
/// - `callbacks` must contain valid function pointers
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_context_provider_from_callbacks(
    callbacks: *const ContextProviderCallbacks,
) -> *mut ContextProviderHandle {
    if callbacks.is_null() {
        return std::ptr::null_mut();
    }

    let callbacks = &*callbacks;
    let provider = CallbackContextProvider::new(ContextProviderCallbacks {
        core_handle: callbacks.core_handle,
        get_platform_activation_height: callbacks.get_platform_activation_height,
        get_quorum_public_key: callbacks.get_quorum_public_key,
    });

    let wrapper = Box::new(ContextProviderWrapper::new(provider));
    Box::into_raw(wrapper) as *mut ContextProviderHandle
}

/// Destroy a context provider handle
///
/// # Safety
/// - `handle` must be a valid context provider handle or null
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_context_provider_destroy(handle: *mut ContextProviderHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut ContextProviderWrapper);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use dash_sdk::dpp::data_contract::TokenConfiguration;
    use dash_sdk::dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use dash_sdk::dpp::version::PlatformVersion;
    use drive_proof_verifier::{ContextProvider, ContextProviderError};

    use super::{
        AdaptiveContextProvider, AdaptiveContextProviderSource, AuxiliaryContextProvider,
        ContextProviderMode, SpvContextProvider, SpvSourceController,
    };
    use crate::types::Network;

    struct TestProvider {
        name: &'static str,
        key_byte: u8,
        fail_quorum: bool,
    }

    impl ContextProvider for TestProvider {
        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            if self.fail_quorum {
                return Err(ContextProviderError::InvalidQuorum(format!(
                    "{} quorum miss",
                    self.name
                )));
            }
            Ok([self.key_byte; 48])
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(999)
        }

        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Err(ContextProviderError::Generic(format!(
                "{} contract route",
                self.name
            )))
        }

        fn get_token_configuration(
            &self,
            _id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Err(ContextProviderError::Generic(format!(
                "{} token route",
                self.name
            )))
        }
    }

    fn provider(name: &'static str, key_byte: u8) -> Arc<dyn ContextProvider> {
        Arc::new(TestProvider {
            name,
            key_byte,
            fail_quorum: false,
        })
    }

    fn failing_provider(name: &'static str) -> Arc<dyn ContextProvider> {
        Arc::new(TestProvider {
            name,
            key_byte: 0,
            fail_quorum: true,
        })
    }

    fn quorum_key(provider: &AdaptiveContextProvider) -> Result<[u8; 48], ContextProviderError> {
        provider.get_quorum_public_key(1, [2; 32], 3)
    }

    fn adaptive(
        trusted: Arc<dyn ContextProvider>,
        network: Network,
    ) -> (AdaptiveContextProvider, Arc<SpvSourceController>) {
        let spv = Arc::new(SpvSourceController::default());
        let auxiliary = Arc::new(AuxiliaryContextProvider::new(Arc::clone(&trusted)));
        (
            AdaptiveContextProvider::new(trusted, Arc::clone(&spv), auxiliary, network),
            spv,
        )
    }

    #[test]
    fn routes_quorum_lookups_by_mode_and_readiness() {
        let (adaptive, spv) = adaptive(provider("trusted", 0x11), Network::Testnet);

        assert_eq!(quorum_key(&adaptive).unwrap(), [0x11; 48]);
        assert_eq!(
            adaptive.active_source(),
            AdaptiveContextProviderSource::Trusted
        );

        adaptive.set_mode(ContextProviderMode::Spv);
        assert!(
            quorum_key(&adaptive).is_err(),
            "SPV without a source must fail"
        );

        let ready = Arc::new(AtomicBool::new(false));
        let ready_for_callback = Arc::clone(&ready);
        let _lease = spv
            .acquire(
                provider("spv", 0x22),
                Arc::new(move || Ok(ready_for_callback.load(Ordering::Acquire))),
            )
            .unwrap();
        assert!(
            quorum_key(&adaptive).is_err(),
            "unready SPV must fail closed"
        );
        assert_eq!(adaptive.active_source(), AdaptiveContextProviderSource::Spv);

        ready.store(true, Ordering::Release);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x22; 48]);

        adaptive.set_mode(ContextProviderMode::Trusted);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x11; 48]);

        adaptive.set_mode(ContextProviderMode::Auto);
        ready.store(false, Ordering::Release);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x11; 48]);
        assert_eq!(
            adaptive.active_source(),
            AdaptiveContextProviderSource::Trusted
        );
        ready.store(true, Ordering::Release);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x22; 48]);
        assert_eq!(adaptive.active_source(), AdaptiveContextProviderSource::Spv);
    }

    #[test]
    fn routes_contracts_and_tokens_to_trusted_for_every_mode() {
        let (adaptive, spv) = adaptive(provider("trusted", 0x11), Network::Mainnet);
        let _lease = spv
            .acquire(provider("spv", 0x22), Arc::new(|| Ok(true)))
            .unwrap();
        let id = Identifier::new([7; 32]);

        for mode in [
            ContextProviderMode::Auto,
            ContextProviderMode::Spv,
            ContextProviderMode::Trusted,
        ] {
            adaptive.set_mode(mode);
            let contract_error = adaptive
                .get_data_contract(&id, PlatformVersion::latest())
                .unwrap_err()
                .to_string();
            let token_error = adaptive
                .get_token_configuration(&id)
                .unwrap_err()
                .to_string();
            assert!(contract_error.contains("trusted contract route"));
            assert!(token_error.contains("trusted token route"));
        }
    }

    #[test]
    fn source_lease_prevents_hijacking_and_releases_on_drop() {
        let controller = Arc::new(SpvSourceController::default());
        let lease = controller
            .acquire(provider("spv", 0x22), Arc::new(|| Ok(true)))
            .unwrap();
        assert!(controller
            .acquire(provider("other", 0x33), Arc::new(|| Ok(true)))
            .is_err());

        drop(lease);
        let _replacement = controller
            .acquire(provider("other", 0x33), Arc::new(|| Ok(true)))
            .unwrap();
        assert_eq!(
            controller
                .ready_source()
                .unwrap()
                .provider
                .get_quorum_public_key(1, [2; 32], 3)
                .unwrap(),
            [0x33; 48]
        );
    }

    #[test]
    fn does_not_retry_a_ready_spv_miss_against_trusted() {
        let (adaptive, spv) = adaptive(provider("trusted", 0x11), Network::Testnet);
        let _lease = spv
            .acquire(failing_provider("spv"), Arc::new(|| Ok(true)))
            .unwrap();
        adaptive.set_mode(ContextProviderMode::Auto);

        let error = quorum_key(&adaptive).unwrap_err().to_string();
        assert!(error.contains("spv quorum miss"));
    }

    #[test]
    fn fixed_spv_never_routes_quorums_to_the_auxiliary_provider() {
        let auxiliary_provider = provider("auxiliary", 0x11);
        let auxiliary = Arc::new(AuxiliaryContextProvider::new(auxiliary_provider));
        let controller = Arc::new(SpvSourceController::default());
        let fixed = SpvContextProvider::new(Arc::clone(&controller), auxiliary, Network::Testnet);

        assert!(fixed.get_quorum_public_key(1, [2; 32], 3).is_err());
        let _lease = controller
            .acquire(provider("spv", 0x22), Arc::new(|| Ok(true)))
            .unwrap();
        assert_eq!(
            fixed.get_quorum_public_key(1, [2; 32], 3).unwrap(),
            [0x22; 48]
        );

        let id = Identifier::new([7; 32]);
        assert!(fixed
            .get_data_contract(&id, PlatformVersion::latest())
            .unwrap_err()
            .to_string()
            .contains("auxiliary contract route"));
        assert!(fixed
            .get_token_configuration(&id)
            .unwrap_err()
            .to_string()
            .contains("auxiliary token route"));
    }
}
