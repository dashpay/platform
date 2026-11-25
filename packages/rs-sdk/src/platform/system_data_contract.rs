//! Resolving a system data contract for the helpers built on one (DPNS, DashPay, moderation
//! charters).

use crate::platform::{DataContract, Fetch, Identifier};
use crate::{Error, Sdk};
use dash_context_provider::ContextProvider;
use dpp::version::ProtocolVersion;
use std::sync::Arc;

impl Sdk {
    /// Fails unless the network runs `initial_protocol_version` or later, the version
    /// `feature` exists from.
    ///
    /// An unpinned SDK starts mainnet and testnet at 13 and only learns a newer version from a
    /// verified response, so below `initial_protocol_version` the SDK first learns the
    /// network's version; a network still below it does not have the feature.
    pub(crate) async fn require_protocol_version(
        &self,
        initial_protocol_version: ProtocolVersion,
        feature: &str,
    ) -> Result<(), Error> {
        if self.protocol_version_number() < initial_protocol_version {
            let version = self.refresh_protocol_version().await?;
            if version < initial_protocol_version {
                return Err(Error::Generic(format!(
                    "{feature} exists from protocol version {initial_protocol_version}; the SDK \
                     runs protocol version {version}"
                )));
            }
        }
        Ok(())
    }

    /// The system data contract `contract_id`, from the context provider when it holds it at
    /// the SDK's protocol version, fetched and proved otherwise. `None` when the network has no
    /// such contract. A fetched contract is registered with the context provider, so the proof
    /// confirming a transition built against it can be verified.
    ///
    /// Only the provider installed when this method starts, which the fetch verifies with,
    /// receives the contract, and only while it is still installed: after a
    /// [`Sdk::set_context_provider`] during the fetch, neither the replacement, which did not
    /// verify it, nor the replaced provider is written to.
    ///
    /// Limitation: a registered contract is sticky. A provider that checks its id-keyed cache
    /// first keeps returning the registered generation after a protocol upgrade (e.g. DashPay
    /// v1 registered at protocol version 13 still served at 14, without v2's fields), and the
    /// cached generation is not checked against the SDK's protocol version. If a concurrent
    /// request registers a newer contract while this fetch is pending, the provider may keep
    /// that one while this method returns the older snapshot it fetched.
    pub async fn fetch_system_data_contract(
        &self,
        contract_id: Identifier,
    ) -> Result<Option<Arc<DataContract>>, Error> {
        let provider = self.context_provider_arc();
        if let Some(contract) = provider.get_data_contract(&contract_id, self.version())? {
            return Ok(Some(contract));
        }
        let contract = DataContract::fetch(self, contract_id).await?.map(Arc::new);
        // Each `set_context_provider` installs a new `Arc`, so an unchanged pointer means the
        // fetch verified with `provider`.
        if let Some(contract) = &contract {
            if Arc::ptr_eq(&provider, &self.context_provider_arc()) {
                provider.register_data_contract(Arc::clone(contract));
            }
        }
        Ok(contract)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::platform::DataContract;
    use crate::SdkBuilder;
    use dash_context_provider::{ContextProvider, ContextProviderError};
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
    use dpp::data_contract::TokenConfiguration;
    use dpp::identity::SecurityLevel;
    use dpp::platform_value::Value;
    use dpp::prelude::{CoreBlockHeight, Identifier};
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::{PlatformVersion, TryFromPlatformVersioned};
    use std::sync::{Arc, Mutex};

    /// `contract` with its `document_type_name` type requiring a `level` key to sign: a contract
    /// an upgrade could store in place of the one the SDK bundles.
    pub(crate) fn with_document_security_level(
        contract: DataContract,
        document_type_name: &str,
        level: SecurityLevel,
    ) -> DataContract {
        let platform_version = PlatformVersion::latest();
        let mut format = DataContractInSerializationFormat::try_from_platform_versioned(
            contract,
            platform_version,
        )
        .expect("the serialization format")
        .into_schema_form();
        format
            .document_schemas_mut()
            .expect("the schema form")
            .get_mut(document_type_name)
            .expect("the document type's schema")
            .set_value("signatureSecurityLevelRequirement", Value::U8(level as u8))
            .expect("the schema is a map");
        DataContract::try_from_platform_versioned(format, true, &mut vec![], platform_version)
            .expect("a valid contract")
    }

    /// A provider that records the contracts registered with it and, the first time it is
    /// asked for a contract, runs `on_lookup`: the hook that swaps the SDK's provider while
    /// a fetch is in flight.
    #[derive(Default)]
    struct Recording {
        registered: Mutex<Vec<Arc<DataContract>>>,
        on_lookup: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    }

    impl ContextProvider for Recording {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            if let Some(hook) = self.on_lookup.lock().expect("hook lock").take() {
                hook();
            }
            Ok(None)
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            Err(ContextProviderError::InvalidQuorum(
                "no quorum in this test".to_string(),
            ))
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }

        fn register_data_contract(&self, contract: Arc<DataContract>) {
            self.registered.lock().expect("the lock").push(contract);
        }
    }

    /// A provider set with `Sdk::set_context_provider` while a system contract fetch is in
    /// flight receives nothing it did not verify itself, and the provider the fetch started
    /// with, no longer installed, is not written to either.
    #[tokio::test]
    async fn should_register_a_fetched_contract_only_with_the_provider_still_installed() {
        let started_with = Arc::new(Recording::default());
        let mut sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .with_context_provider(Arc::clone(&started_with))
            .build()
            .expect("the mock SDK builds");
        let replacement = Arc::new(Recording::default());
        let setter = sdk.clone();
        let installed = Arc::clone(&replacement);
        *started_with.on_lookup.lock().expect("hook lock") = Some(Box::new(move || {
            setter
                .set_context_provider(installed)
                .expect("the replacement states no requirements");
        }));
        let stored = load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("the DPNS contract loads");
        sdk.mock()
            .expect_fetch(stored.id(), Some(stored.clone()))
            .await
            .expect("the DPNS fetch is expected");

        let fetched = sdk
            .fetch_system_data_contract(stored.id())
            .await
            .expect("the fetch succeeds")
            .expect("the contract exists");

        assert_eq!(fetched.id(), stored.id());
        assert!(
            replacement.registered.lock().expect("the lock").is_empty(),
            "the replacement did not verify the fetched contract"
        );
        assert!(
            started_with.registered.lock().expect("the lock").is_empty(),
            "the provider the fetch started with is no longer installed"
        );
    }
}
