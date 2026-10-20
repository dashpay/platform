//! Serving the manager's SPV quorum keys to its SDK's context provider.

use std::sync::{Arc, PoisonError};

use dash_context_provider::routing::SpvSourceLease;
use dash_context_provider::ContextProvider;

use super::PlatformWalletManager;
use crate::changeset::PlatformWalletPersistence;
use crate::spv_context_provider::SpvQuorumSource;

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

    use dash_context_provider::routing::{ContractSource, SpvContextProvider};
    use dash_context_provider::{ContextProvider, ContextProviderError};
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
        let slot = provider.source_slot();

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
}
