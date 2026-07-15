//! Context Provider FFI bindings
//!
//! This module provides FFI bindings for configuring context providers,
//! allowing the Platform SDK to connect to Core SDK for proof verification.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

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
pub enum ContextProviderSource {
    Trusted = 0,
    Spv = 1,
}

struct SpvContextSource {
    provider: Arc<dyn ContextProvider>,
    is_ready: Arc<dyn Fn() -> bool + Send + Sync>,
}

/// Context provider whose identity remains fixed while quorum routing adapts.
pub struct AdaptiveContextProvider {
    trusted: Arc<dyn ContextProvider>,
    spv: ArcSwapOption<SpvContextSource>,
    mode: AtomicU8,
    network: Network,
}

impl AdaptiveContextProvider {
    pub fn new(trusted: Arc<dyn ContextProvider>, network: Network) -> Self {
        Self {
            trusted,
            spv: ArcSwapOption::empty(),
            mode: AtomicU8::new(ContextProviderMode::Auto as u8),
            network,
        }
    }

    /// Populate the SPV source exactly once.
    pub fn set_spv_source(
        &self,
        provider: Arc<dyn ContextProvider>,
        is_ready: Arc<dyn Fn() -> bool + Send + Sync>,
    ) -> Result<(), &'static str> {
        let source = Arc::new(SpvContextSource { provider, is_ready });
        let previous = self
            .spv
            .compare_and_swap(std::ptr::null::<SpvContextSource>(), Some(source));
        if previous.is_some() {
            Err("SPV context source is already populated")
        } else {
            Ok(())
        }
    }

    pub fn set_mode(&self, mode: ContextProviderMode) {
        self.mode.store(mode as u8, Ordering::Release);
    }

    pub fn mode(&self) -> ContextProviderMode {
        ContextProviderMode::try_from(self.mode.load(Ordering::Acquire))
            .expect("context-provider mode is only written from validated values")
    }

    pub fn active_source(&self) -> ContextProviderSource {
        let spv = self.spv.load();
        match self.mode() {
            ContextProviderMode::Trusted => ContextProviderSource::Trusted,
            ContextProviderMode::Spv if spv.is_some() => ContextProviderSource::Spv,
            ContextProviderMode::Auto if spv.as_ref().is_some_and(|source| (source.is_ready)()) => {
                ContextProviderSource::Spv
            }
            ContextProviderMode::Auto | ContextProviderMode::Spv => ContextProviderSource::Trusted,
        }
    }

    fn ready_spv_source(&self) -> Result<Arc<SpvContextSource>, ContextProviderError> {
        let source = self.spv.load_full().ok_or_else(|| {
            ContextProviderError::Generic("SPV context source is not populated".to_string())
        })?;
        if !(source.is_ready)() {
            return Err(ContextProviderError::Generic(
                "SPV context source is not ready".to_string(),
            ));
        }
        Ok(source)
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
            ContextProviderMode::Spv => self.ready_spv_source()?.provider.get_quorum_public_key(
                quorum_type,
                quorum_hash,
                core_chain_locked_height,
            ),
            ContextProviderMode::Auto => match self.spv.load_full() {
                Some(source) if (source.is_ready)() => source.provider.get_quorum_public_key(
                    quorum_type,
                    quorum_hash,
                    core_chain_locked_height,
                ),
                _ => self.trusted.get_quorum_public_key(
                    quorum_type,
                    quorum_hash,
                    core_chain_locked_height,
                ),
            },
        }
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        match self.network {
            Network::Mainnet => Ok(2_132_092),
            Network::Testnet => Ok(1_090_319),
            Network::Devnet | Network::Regtest => Ok(1),
        }
    }

    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        self.trusted.get_data_contract(id, platform_version)
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        self.trusted.get_token_configuration(token_id)
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

    use super::{AdaptiveContextProvider, ContextProviderMode, ContextProviderSource};
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

    #[test]
    fn routes_quorum_lookups_by_mode_and_readiness() {
        let adaptive = AdaptiveContextProvider::new(provider("trusted", 0x11), Network::Testnet);

        assert_eq!(quorum_key(&adaptive).unwrap(), [0x11; 48]);
        assert_eq!(adaptive.active_source(), ContextProviderSource::Trusted);

        adaptive.set_mode(ContextProviderMode::Spv);
        assert!(
            quorum_key(&adaptive).is_err(),
            "SPV without a source must fail"
        );

        let ready = Arc::new(AtomicBool::new(false));
        let ready_for_callback = Arc::clone(&ready);
        adaptive
            .set_spv_source(
                provider("spv", 0x22),
                Arc::new(move || ready_for_callback.load(Ordering::Acquire)),
            )
            .unwrap();
        assert!(
            quorum_key(&adaptive).is_err(),
            "unready SPV must fail closed"
        );
        assert_eq!(adaptive.active_source(), ContextProviderSource::Spv);

        ready.store(true, Ordering::Release);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x22; 48]);

        adaptive.set_mode(ContextProviderMode::Trusted);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x11; 48]);

        adaptive.set_mode(ContextProviderMode::Auto);
        ready.store(false, Ordering::Release);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x11; 48]);
        assert_eq!(adaptive.active_source(), ContextProviderSource::Trusted);
        ready.store(true, Ordering::Release);
        assert_eq!(quorum_key(&adaptive).unwrap(), [0x22; 48]);
        assert_eq!(adaptive.active_source(), ContextProviderSource::Spv);
    }

    #[test]
    fn routes_contracts_and_tokens_to_trusted_for_every_mode() {
        let adaptive = AdaptiveContextProvider::new(provider("trusted", 0x11), Network::Mainnet);
        let ready = Arc::new(|| true);
        adaptive
            .set_spv_source(provider("spv", 0x22), ready)
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
    fn rejects_replacing_the_spv_source() {
        let adaptive = AdaptiveContextProvider::new(provider("trusted", 0x11), Network::Regtest);
        adaptive
            .set_spv_source(provider("spv", 0x22), Arc::new(|| true))
            .unwrap();
        assert!(adaptive
            .set_spv_source(provider("other", 0x33), Arc::new(|| true))
            .is_err());
    }

    #[test]
    fn does_not_retry_a_ready_spv_miss_against_trusted() {
        let adaptive = AdaptiveContextProvider::new(provider("trusted", 0x11), Network::Testnet);
        adaptive
            .set_spv_source(failing_provider("spv"), Arc::new(|| true))
            .unwrap();
        adaptive.set_mode(ContextProviderMode::Auto);

        let error = quorum_key(&adaptive).unwrap_err().to_string();
        assert!(error.contains("spv quorum miss"));
    }
}
