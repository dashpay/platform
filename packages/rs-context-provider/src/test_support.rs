use std::sync::Arc;

use dpp::dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;

use crate::{
    ContextProvider, ContextProviderError, ContractSource, QuorumKeyFuture, SpvSourceLease,
    SpvSourceSlot,
};

/// A quorum source answering every lookup with `[key_byte; 48]`, or failing.
struct TestQuorums {
    name: &'static str,
    key_byte: u8,
    fail: bool,
    network: Option<Network>,
}

impl ContextProvider for TestQuorums {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        if self.fail {
            return Err(ContextProviderError::InvalidQuorum(format!(
                "{} quorum miss",
                self.name
            )));
        }
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
        fail: false,
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
        fail: false,
        network: Some(network),
    })
}

pub(super) fn failing_quorums(name: &'static str) -> Arc<dyn ContextProvider> {
    Arc::new(TestQuorums {
        name,
        key_byte: 0,
        fail: true,
        network: None,
    })
}

pub(super) fn quorum_key(
    provider: &impl ContextProvider,
) -> Result<[u8; 48], ContextProviderError> {
    provider.get_quorum_public_key(1, [2; 32], 3)
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
