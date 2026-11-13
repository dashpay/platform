use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use dpp::dashcore::sml::llmq_type::network::NetworkLLMQExt;
use dpp::dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;

use crate::{
    ContextProvider, ContextProviderError, ContractSource, QuorumKeyFuture, SpvSourceLease,
    SpvSourceSlot,
};

/// A quorum source answering every lookup with `[key_byte; 48]`.
struct TestQuorums {
    name: &'static str,
    key_byte: u8,
    network: Option<Network>,
}

impl ContextProvider for TestQuorums {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        Ok([self.key_byte; 48])
    }

    fn fetch_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _height: u32,
    ) -> Option<QuorumKeyFuture> {
        let key = [self.key_byte; 48];
        Some(Box::pin(async move { Ok(Some(key)) }))
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

    fn network(&self) -> Option<Network> {
        self.network
    }
}

/// A contract source whose reads fail with a recognisable message.
pub(super) struct TestContracts;

impl ContractSource for TestContracts {
    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Err(ContextProviderError::Generic(
            "local contract route".to_string(),
        ))
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Err(ContextProviderError::Generic(
            "local token route".to_string(),
        ))
    }

    fn register_data_contract(&self, _contract: Arc<DataContract>) {}
}

pub(super) fn quorums(name: &'static str, key_byte: u8) -> Arc<dyn ContextProvider> {
    Arc::new(TestQuorums {
        name,
        key_byte,
        network: None,
    })
}

/// Like [`quorums`], serving the quorums of `network` only.
pub(super) fn network_quorums(
    name: &'static str,
    key_byte: u8,
    network: Network,
) -> Arc<dyn ContextProvider> {
    Arc::new(TestQuorums {
        name,
        key_byte,
        network: Some(network),
    })
}

/// The quorum type the tests' lookups name: Platform's on testnet, the
/// network the tests' providers serve.
pub(super) fn platform_quorum_type() -> u32 {
    u32::from(u8::from(Network::Testnet.platform_type()))
}

pub(super) fn quorum_key(
    provider: &impl ContextProvider,
) -> Result<[u8; 48], ContextProviderError> {
    provider.get_quorum_public_key(platform_quorum_type(), [2; 32], 3)
}

/// What a [`TestSpv`] answers a lookup with.
#[derive(Clone, Copy)]
pub(super) enum SpvAnswer {
    /// `[0x22; 48]`.
    Key,
    /// The error the function builds.
    Fails(fn() -> ContextProviderError),
}

/// An SPV quorum source whose answer a test sets, and can change while a
/// fetch is pending.
pub(super) struct TestSpv {
    answer: Mutex<SpvAnswer>,
}

impl TestSpv {
    pub(super) fn new(answer: SpvAnswer) -> Arc<Self> {
        Arc::new(Self {
            answer: Mutex::new(answer),
        })
    }

    pub(super) fn answer(&self, answer: SpvAnswer) {
        *self.answer.lock().expect("test state lock") = answer;
    }
}

impl ContextProvider for TestSpv {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        match *self.answer.lock().expect("test state lock") {
            SpvAnswer::Key => Ok([0x22; 48]),
            SpvAnswer::Fails(error) => Err(error()),
        }
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(999)
    }

    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }
}

/// SPV's answer for a quorum it does not hold.
pub(super) fn spv_miss() -> ContextProviderError {
    ContextProviderError::Generic("SPV error: quorum not found".to_string())
}

/// SPV's answer for a quorum it holds but has not verified.
pub(super) fn spv_unverified() -> ContextProviderError {
    ContextProviderError::QuorumSourceUnavailable("quorum is not verified by SPV yet".to_string())
}

/// An SPV source that cannot serve keys as configured.
pub(super) fn spv_misconfigured() -> ContextProviderError {
    ContextProviderError::Config("SPV source misconfigured".to_string())
}

/// SPV's refusal of a quorum type other than Platform's.
pub(super) fn spv_wrong_type() -> ContextProviderError {
    ContextProviderError::InvalidQuorum("not the Platform quorum".to_string())
}

/// What a [`TestTrusted`] fetch resolves to.
#[derive(Clone, Copy, Debug)]
pub(super) enum TrustedFetch {
    /// `[0x11; 48]`, which the lookup then returns as well.
    Key,
    /// The service answered and does not list the quorum.
    Unlisted,
    /// The service gave no answer.
    Unreachable,
    /// The provider cannot fetch.
    Cannot,
}

/// A trusted quorum source with a cache of one key, `[0x11; 48]`, which
/// counts the lookups it is asked.
pub(super) struct TestTrusted {
    cached: Arc<AtomicBool>,
    fetch: TrustedFetch,
    lookups: AtomicUsize,
}

impl TestTrusted {
    pub(super) fn new(cached: bool, fetch: TrustedFetch) -> Arc<Self> {
        Arc::new(Self {
            cached: Arc::new(AtomicBool::new(cached)),
            fetch,
            lookups: AtomicUsize::new(0),
        })
    }

    pub(super) fn lookups(&self) -> usize {
        self.lookups.load(Ordering::Acquire)
    }
}

impl ContextProvider for TestTrusted {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        self.lookups.fetch_add(1, Ordering::AcqRel);
        if self.cached.load(Ordering::Acquire) {
            Ok([0x11; 48])
        } else {
            Err(ContextProviderError::InvalidQuorum(
                "trusted cache miss".to_string(),
            ))
        }
    }

    fn fetch_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _height: u32,
    ) -> Option<QuorumKeyFuture> {
        let result = match self.fetch {
            TrustedFetch::Key => Ok(Some([0x11; 48])),
            TrustedFetch::Unlisted => Ok(None),
            TrustedFetch::Unreachable => Err(ContextProviderError::QuorumSourceUnavailable(
                "trusted service unreachable".to_string(),
            )),
            TrustedFetch::Cannot => return None,
        };
        let cached = Arc::clone(&self.cached);
        Some(Box::pin(async move {
            if matches!(result, Ok(Some(_))) {
                cached.store(true, Ordering::Release);
            }
            result
        }))
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(999)
    }

    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }
}

/// Claim `slot` and publish `provider` into it, with `ready` as its
/// readiness.
pub(super) fn publish(
    slot: &Arc<SpvSourceSlot>,
    provider: Arc<dyn ContextProvider>,
    ready: bool,
) -> SpvSourceLease {
    let lease = slot.claim().expect("empty slot");
    lease.publish(provider, Arc::new(move || Ok(ready)));
    lease
}

/// The SPV slot `provider` takes quorum keys from.
pub(super) fn slot_of(provider: &impl ContextProvider) -> Arc<SpvSourceSlot> {
    provider.spv_source_slot().expect("an SPV slot")
}

pub(super) fn is_unavailable(result: Result<[u8; 48], ContextProviderError>) -> bool {
    matches!(
        result,
        Err(ContextProviderError::QuorumSourceUnavailable(_))
    )
}
