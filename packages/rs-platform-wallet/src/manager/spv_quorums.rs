//! Serving the manager's SPV quorum keys to its SDK's context provider.

use std::sync::{Arc, PoisonError};

use dash_context_provider::ContextProvider;
use dash_context_provider::SpvSourceLease;
use dash_sdk::Sdk;

use super::PlatformWalletManager;
use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;
use crate::spv_context_provider::SpvQuorumSource;

/// Claim the SPV slot of `sdk`'s current context provider, for a manager to
/// serve its SPV quorum keys through ([`PlatformWalletManager::serve_spv_quorums`]).
///
/// `Ok(None)` when the provider takes no keys from an SPV client. Fails with
/// [`PlatformWalletError::InvalidParameter`] while another manager holds the
/// slot: only one manager serves an SDK. The slot is the current provider's,
/// so a provider set after the SDK was built is the one served.
pub fn claim_spv_quorum_lease(sdk: &Sdk) -> Result<Option<SpvSourceLease>, PlatformWalletError> {
    let Some(slot) = sdk
        .context_provider()
        .and_then(|provider| provider.spv_source_slot())
    else {
        return Ok(None);
    };
    slot.claim()
        .map(Some)
        .map_err(|taken| PlatformWalletError::InvalidParameter(taken.to_string()))
}

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Serve this manager's SPV quorum keys through `lease` until
    /// [`Self::shutdown`].
    ///
    /// The keys come from the manager's own SPV client, read live on every
    /// lookup, and only while that client is running and synced; otherwise
    /// the SDK's lookups fail closed. Starting and stopping SPV therefore
    /// needs nothing more here.
    ///
    /// `lease` is claimed by the caller from the SDK's slot before this
    /// manager is built, so a slot another manager holds is refused before
    /// anything else is set up.
    pub fn serve_spv_quorums(&self, lease: SpvSourceLease) {
        let source = Arc::new(SpvQuorumSource::new(
            Arc::clone(&self.spv_manager),
            self.sdk.network,
        ));
        let readiness = Arc::clone(&source);
        lease.publish(
            source as Arc<dyn ContextProvider>,
            Arc::new(move || readiness.is_ready()),
        );
        *self
            .spv_quorum_lease
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(lease);
    }

    /// Stop serving SPV quorum keys; the SDK's slot is free again.
    pub(super) fn withdraw_spv_quorums(&self) {
        let lease = self
            .spv_quorum_lease
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        drop(lease);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use dash_context_provider::{ContextProvider, ContextProviderError};
    use dash_context_provider::{ContractSource, SpvContextProvider};

    use super::claim_spv_quorum_lease;
    use crate::error::PlatformWalletError;
    use dashcore::sml::llmq_type::network::NetworkLLMQExt;
    use dpp::data_contract::TokenConfiguration;
    use dpp::prelude::{DataContract, Identifier};
    use dpp::version::PlatformVersion;

    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::persister::NoPlatformPersistence;
    use crate::PlatformWalletManager;

    struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    struct NoContracts;
    impl ContractSource for NoContracts {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Ok(None)
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }

        fn register_data_contract(&self, _contract: Arc<DataContract>) {}
    }

    fn manager() -> PlatformWalletManager<NoPlatformPersistence> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        PlatformWalletManager::new(
            sdk,
            Arc::new(NoPlatformPersistence),
            Arc::new(NoopEventHandler) as Arc<dyn PlatformEventHandler>,
        )
    }

    /// A manager holds the SDK's slot from the moment it serves until it shuts
    /// down. Meanwhile no other manager can claim it, and a lookup fails
    /// closed while the manager's SPV client is not running, without blaming
    /// the node.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn should_hold_the_slot_from_serving_until_shutdown() {
        let manager = manager();
        let network = manager.sdk.network;
        let provider = SpvContextProvider::new(Arc::new(NoContracts), network);
        let slot = provider.spv_source_slot().expect("an SPV slot");

        manager.serve_spv_quorums(slot.claim().expect("empty slot"));
        assert!(
            slot.claim().is_err(),
            "a second manager must not serve the same SDK"
        );

        let platform_type = u32::from(u8::from(network.platform_type()));
        let lookup = provider.get_quorum_public_key(platform_type, [7; 32], 1);
        // "not ready" rather than "not active": the manager published its own
        // source, which reports its stopped client as unsynced.
        assert!(
            matches!(
                lookup,
                Err(ContextProviderError::QuorumSourceUnavailable(ref message))
                    if message.contains("not ready")
            ),
            "{lookup:?}"
        );

        let _report = manager.shutdown().await;
        assert!(
            slot.claim().is_ok(),
            "shutdown must free the slot for the next manager"
        );
    }

    /// A manager serves the SPV slot of its SDK's current provider. Native
    /// code may replace the provider an SDK was built with; a manager that
    /// claimed the replaced provider's slot would serve keys nobody reads,
    /// and the current provider's lookups would fail closed for good.
    #[test]
    fn should_claim_the_slot_of_the_sdks_current_provider() {
        let sdk = dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk");
        assert!(
            claim_spv_quorum_lease(&sdk)
                .expect("no slot to take")
                .is_none(),
            "a provider without an SPV slot has nothing to serve"
        );

        let replacement = SpvContextProvider::new(Arc::new(NoContracts), sdk.network);
        let slot = replacement.spv_source_slot().expect("an SPV slot");
        sdk.set_context_provider(replacement)
            .expect("a provider for the mock SDK's network");

        let lease = claim_spv_quorum_lease(&sdk)
            .expect("an empty slot")
            .expect("the current provider's slot");
        assert!(slot.claim().is_err(), "the lease must hold that slot");
        assert!(
            matches!(
                claim_spv_quorum_lease(&sdk),
                Err(PlatformWalletError::InvalidParameter(_))
            ),
            "a second manager must not serve the same SDK"
        );

        drop(lease);
        assert!(slot.claim().is_ok(), "dropping the lease frees the slot");
    }
}
