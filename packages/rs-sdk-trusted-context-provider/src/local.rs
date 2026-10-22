//! Contracts and token configurations a context provider answers for
//! without a network.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use dash_context_provider::routing::ContractSource;
use dash_context_provider::ContextProviderError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{DataContract, Identifier};
#[cfg(any(
    feature = "dpns-contract",
    feature = "dashpay-contract",
    feature = "withdrawals-contract",
    feature = "wallet-utils-contract",
    feature = "token-history-contract",
    feature = "keywords-contract",
    feature = "document-history-contract",
    feature = "app-connect-contract",
    feature = "moderation-charters-contract",
    feature = "all-system-contracts"
))]
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
#[cfg(any(feature = "app-connect-contract", feature = "all-system-contracts"))]
use dpp::version::feature_initial_protocol_versions::APP_CONNECT_CONTRACT_INITIAL_PROTOCOL_VERSION;
#[cfg(any(
    feature = "moderation-charters-contract",
    feature = "all-system-contracts"
))]
use dpp::version::feature_initial_protocol_versions::MODERATION_CHARTERS_CONTRACT_INITIAL_PROTOCOL_VERSION;
use dpp::version::PlatformVersion;

/// The data contracts and token configurations a context provider can serve
/// without a network: the ones registered with it, and the system contracts
/// this crate is built with, each from the protocol version that introduced it.
///
/// It has no quorum keys. A provider that verifies proofs pairs it with a
/// quorum source; [`TrustedHttpContextProvider`](crate::TrustedHttpContextProvider)
/// holds one, and a provider without a trusted quorum endpoint can use one on
/// its own. Share it through an `Arc`.
#[derive(Debug, Default)]
pub struct LocalContextResolver {
    /// Contracts served before the system contracts.
    known_contracts: Mutex<HashMap<Identifier, Arc<DataContract>>>,
    /// Token configurations needed for proof verification.
    known_token_configurations: Mutex<HashMap<Identifier, TokenConfiguration>>,
}

impl LocalContextResolver {
    /// An empty resolver; it serves only the system contracts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a data contract to the known contracts cache
    pub fn add_known_contract(&self, contract: DataContract) {
        self.add_known_data_contract(Arc::new(contract));
    }

    /// Add a shared data contract to the known contracts cache.
    pub fn add_known_data_contract(&self, contract: Arc<DataContract>) {
        let id = contract.id();
        let mut known = self
            .known_contracts
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        known.insert(id, contract);
    }

    /// Insert a contract only when its version is strictly newer, returning
    /// the retained entry. Equal versions preserve the existing body.
    pub fn add_known_contract_if_newer(&self, contract: DataContract) -> Arc<DataContract> {
        let id = contract.id();
        let mut known = self
            .known_contracts
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(existing) = known.get(&id) {
            if existing.version() >= contract.version() {
                return existing.clone();
            }
        }
        let contract = Arc::new(contract);
        known.insert(id, contract.clone());
        contract
    }

    /// Get a data contract from the known contracts cache
    /// Returns None if the contract is not in the cache
    pub fn get_known_contract(&self, id: &Identifier) -> Option<Arc<DataContract>> {
        let known = self
            .known_contracts
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        known.get(id).cloned()
    }

    /// Remove a data contract from the known contracts cache
    /// Returns true if the contract was present and removed, false otherwise
    pub fn remove_known_contract(&self, id: &Identifier) -> bool {
        let mut known = self
            .known_contracts
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        known.remove(id).is_some()
    }

    /// Add multiple data contracts to the known contracts cache, each through
    /// [`Self::add_known_contract_if_newer`]. A copy preloaded from storage
    /// never replaces a newer version the cache already holds.
    pub fn add_known_contracts(&self, contracts: Vec<DataContract>) {
        for contract in contracts {
            self.add_known_contract_if_newer(contract);
        }
    }

    /// Add a token configuration to the known token configurations cache
    pub fn add_known_token_configuration(&self, token_id: Identifier, config: TokenConfiguration) {
        let mut known = self
            .known_token_configurations
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        known.insert(token_id, config);
    }

    /// Add multiple token configurations to the known token configurations cache
    pub fn add_known_token_configurations(&self, configs: Vec<(Identifier, TokenConfiguration)>) {
        let mut known = self
            .known_token_configurations
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for (token_id, config) in configs {
            known.insert(token_id, config);
        }
    }

    /// The contract registered under `id`, else the system contract with that
    /// id if `platform_version` has it, else `None`.
    pub fn data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        // First check known contracts cache
        if let Some(contract) = self.get_known_contract(id) {
            return Ok(Some(contract));
        }

        // Check if this is a system data contract and the corresponding feature is enabled
        #[cfg(any(
            feature = "dpns-contract",
            feature = "dashpay-contract",
            feature = "withdrawals-contract",
            feature = "wallet-utils-contract",
            feature = "token-history-contract",
            feature = "keywords-contract",
            feature = "document-history-contract",
            feature = "app-connect-contract",
            feature = "moderation-charters-contract",
            feature = "all-system-contracts"
        ))]
        {
            // Check each system contract if its feature is enabled
            #[cfg(any(feature = "dpns-contract", feature = "all-system-contracts"))]
            if *id == SystemDataContract::DPNS.id() {
                return load_system_data_contract(SystemDataContract::DPNS, platform_version)
                    .map(|contract| Some(Arc::new(contract)))
                    .map_err(|e| {
                        ContextProviderError::Generic(format!(
                            "Failed to load DPNS contract: {}",
                            e
                        ))
                    });
            }

            #[cfg(any(feature = "dashpay-contract", feature = "all-system-contracts"))]
            if *id == SystemDataContract::Dashpay.id() {
                return load_system_data_contract(SystemDataContract::Dashpay, platform_version)
                    .map(|contract| Some(Arc::new(contract)))
                    .map_err(|e| {
                        ContextProviderError::Generic(format!(
                            "Failed to load Dashpay contract: {}",
                            e
                        ))
                    });
            }

            #[cfg(any(feature = "withdrawals-contract", feature = "all-system-contracts"))]
            if *id == SystemDataContract::Withdrawals.id() {
                return load_system_data_contract(
                    SystemDataContract::Withdrawals,
                    platform_version,
                )
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load Withdrawals contract: {}",
                        e
                    ))
                });
            }

            #[cfg(any(feature = "wallet-utils-contract", feature = "all-system-contracts"))]
            if *id == SystemDataContract::WalletUtils.id() {
                return load_system_data_contract(
                    SystemDataContract::WalletUtils,
                    platform_version,
                )
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load WalletUtils contract: {}",
                        e
                    ))
                });
            }

            #[cfg(any(feature = "token-history-contract", feature = "all-system-contracts"))]
            if *id == SystemDataContract::TokenHistory.id() {
                return load_system_data_contract(
                    SystemDataContract::TokenHistory,
                    platform_version,
                )
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load TokenHistory contract: {}",
                        e
                    ))
                });
            }

            #[cfg(any(feature = "keywords-contract", feature = "all-system-contracts"))]
            if *id == SystemDataContract::KeywordSearch.id() {
                return load_system_data_contract(
                    SystemDataContract::KeywordSearch,
                    platform_version,
                )
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load KeywordSearch contract: {}",
                        e
                    ))
                });
            }

            #[cfg(any(
                feature = "document-history-contract",
                feature = "all-system-contracts"
            ))]
            if *id == SystemDataContract::DocumentHistory.id() {
                return load_system_data_contract(
                    SystemDataContract::DocumentHistory,
                    platform_version,
                )
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load DocumentHistory contract: {}",
                        e
                    ))
                });
            }

            #[cfg(any(feature = "app-connect-contract", feature = "all-system-contracts"))]
            // Below protocol version 14 the app-connect contract is
            // absent, so the lookup returns `None`.
            if *id == SystemDataContract::AppConnect.id()
                && platform_version.protocol_version
                    >= APP_CONNECT_CONTRACT_INITIAL_PROTOCOL_VERSION
            {
                return load_system_data_contract(SystemDataContract::AppConnect, platform_version)
                    .map(|contract| Some(Arc::new(contract)))
                    .map_err(|e| {
                        ContextProviderError::Generic(format!(
                            "Failed to load AppConnect contract: {}",
                            e
                        ))
                    });
            }

            #[cfg(any(
                feature = "moderation-charters-contract",
                feature = "all-system-contracts"
            ))]
            // Below protocol version 14 the moderation charters contract is absent too.
            if *id == SystemDataContract::ModerationCharters.id()
                && platform_version.protocol_version
                    >= MODERATION_CHARTERS_CONTRACT_INITIAL_PROTOCOL_VERSION
            {
                return load_system_data_contract(
                    SystemDataContract::ModerationCharters,
                    platform_version,
                )
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load ModerationCharters contract: {}",
                        e
                    ))
                });
            }
        }

        // Only the system contracts read the version, and a build may have none.
        let _ = platform_version;
        Ok(None)
    }

    /// The token configuration registered under `token_id`, if any.
    pub fn token_configuration(&self, token_id: &Identifier) -> Option<TokenConfiguration> {
        let known = self
            .known_token_configurations
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        known.get(token_id).cloned()
    }
}

impl ContractSource for LocalContextResolver {
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        self.data_contract(id, platform_version)
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(self.token_configuration(token_id))
    }

    fn register_data_contract(&self, contract: Arc<DataContract>) {
        self.add_known_data_contract(contract)
    }
}

#[cfg(all(test, any(feature = "dpns-contract", feature = "all-system-contracts")))]
mod tests {
    use std::sync::Arc;

    use dash_context_provider::routing::ContractSource;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::prelude::{DataContract, Identifier};
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;

    use super::LocalContextResolver;

    /// The DPNS contract under an id no system contract has.
    fn app_contract() -> DataContract {
        let mut contract =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("DPNS contract");
        contract.set_id(Identifier::new([0x5a; 32]));
        contract
    }

    #[test]
    fn should_serve_registered_and_system_contracts_without_a_network() {
        let resolver = LocalContextResolver::new();
        let contract = app_contract();
        let id = contract.id();
        let version = PlatformVersion::latest();

        assert!(resolver.data_contract(&id, version).unwrap().is_none());
        resolver.add_known_data_contract(Arc::new(contract));
        assert_eq!(
            resolver
                .data_contract(&id, version)
                .unwrap()
                .map(|contract| contract.id()),
            Some(id)
        );

        let dpns = SystemDataContract::DPNS.id();
        assert_eq!(
            resolver
                .data_contract(&dpns, version)
                .unwrap()
                .map(|contract| contract.id()),
            Some(dpns)
        );
    }

    /// Token proofs are verified against the token configuration the
    /// resolver holds, so a registered configuration must be served back.
    #[test]
    fn should_serve_a_registered_token_configuration() {
        use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
        use dpp::data_contract::TokenConfiguration;

        let resolver = LocalContextResolver::new();
        let token_id = Identifier::new([0x7c; 32]);
        assert!(resolver.token_configuration(&token_id).is_none());

        let config = TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
        resolver.add_known_token_configuration(token_id, config.clone());
        assert_eq!(resolver.token_configuration(&token_id), Some(config));
    }

    #[test]
    fn should_keep_a_newer_contract_when_an_older_one_is_offered() {
        let resolver = LocalContextResolver::new();
        let mut newer = app_contract();
        newer.set_version(2);
        let mut older = app_contract();
        older.set_version(1);

        resolver.add_known_contract_if_newer(newer);
        let kept = resolver.add_known_contract_if_newer(older);
        assert_eq!(kept.version(), 2);
    }

    /// Apps preload contracts from their own storage into the cache proof
    /// verification reads. A stored copy older than the one the cache holds,
    /// for example one registered after a proven fetch, must not replace it,
    /// or later proofs and documents would use the obsolete schema.
    #[test]
    fn should_not_downgrade_a_cached_contract_when_preloading_contracts() {
        let resolver = LocalContextResolver::new();
        let mut registered = app_contract();
        registered.set_version(2);
        let id = registered.id();
        resolver.register_data_contract(Arc::new(registered));
        let cached = resolver
            .get_known_contract(&id)
            .expect("registered contract");

        let mut older = app_contract();
        older.set_version(1);
        resolver.add_known_contracts(vec![older]);
        let after_older = resolver.get_known_contract(&id).expect("cached contract");
        assert_eq!(
            after_older.version(),
            2,
            "an older preload downgraded the cache"
        );

        let mut same_version = app_contract();
        same_version.set_version(2);
        resolver.add_known_contracts(vec![same_version]);
        assert!(
            Arc::ptr_eq(
                &cached,
                &resolver.get_known_contract(&id).expect("cached contract")
            ),
            "an equal version must keep the cached body"
        );

        let mut newer = app_contract();
        newer.set_version(3);
        resolver.add_known_contracts(vec![newer]);
        assert_eq!(
            resolver
                .get_known_contract(&id)
                .map(|contract| contract.version()),
            Some(3),
            "a newer preload must still replace the cached contract"
        );

        let mut newest = app_contract();
        newest.set_version(5);
        let mut stale = app_contract();
        stale.set_version(4);
        resolver.add_known_contracts(vec![newest, stale]);
        assert_eq!(
            resolver
                .get_known_contract(&id)
                .map(|contract| contract.version()),
            Some(5),
            "a later, older copy in the same batch must not win"
        );
    }
}
