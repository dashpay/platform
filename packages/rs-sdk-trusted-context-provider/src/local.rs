//! Contracts and token configurations a context provider answers for
//! without a network.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use dash_context_provider::ContextProviderError;
use dash_context_provider::ContractSource;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{DataContract, Identifier};
use dpp::version::PlatformVersion;

use crate::known_contracts::hold_unless_newer_held;
use crate::system_contracts::{is_inactive_system_contract, system_data_contract};

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

    /// Add a data contract to the known contracts cache, as
    /// [`Self::add_known_data_contract`] adds one.
    pub fn add_known_contract(&self, contract: DataContract) {
        self.add_known_data_contract(Arc::new(contract));
    }

    /// Add a shared data contract to the known contracts cache, unless it
    /// holds one of a higher version under the same id, which another
    /// operation promoted: a registration that waited on its fetch, or an
    /// older contract restored from a saved cache, does not replace it.
    pub fn add_known_data_contract(&self, contract: Arc<DataContract>) {
        let mut known = self
            .known_contracts
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        hold_unless_newer_held(&mut known, contract);
    }

    /// Put a data contract in the known contracts cache in place of whatever it
    /// holds under its id, of any version: an explicit replacement by a holder
    /// that means it, such as one rolling back to a contract it bundles.
    /// [`Self::add_known_contract`] keeps a newer one instead.
    pub fn replace_known_contract(&self, contract: DataContract) {
        let id = contract.id();
        let mut known = self
            .known_contracts
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        known.insert(id, Arc::new(contract));
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

    /// Add multiple data contracts to the known contracts cache, each as
    /// [`Self::add_known_contract`] adds one. A copy preloaded from storage
    /// never replaces a newer version the cache already holds.
    pub fn add_known_contracts(&self, contracts: Vec<DataContract>) {
        let mut known = self
            .known_contracts
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for contract in contracts {
            hold_unless_newer_held(&mut known, Arc::new(contract));
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
    /// id if `platform_version` has it, else `None`. Under the id of a system
    /// contract `platform_version` does not have yet it is always `None`.
    pub fn data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        // A system contract not active yet is served under its id from no
        // source, not even a contract preloaded under that id.
        if is_inactive_system_contract(id, platform_version) {
            return Ok(None);
        }

        // First check known contracts cache
        if let Some(contract) = self.get_known_contract(id) {
            return Ok(Some(contract));
        }

        system_data_contract(id, platform_version)
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

    use dash_context_provider::ContractSource;
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

    /// The token market contract does not exist before protocol version 15.
    /// The SPV and adaptive providers read contracts only from the resolver,
    /// so it must refuse a contract preloaded under that id there itself.
    #[test]
    fn should_refuse_a_preloaded_token_market_contract_before_protocol_15() {
        let resolver = LocalContextResolver::new();
        let mut contract = app_contract();
        let id = SystemDataContract::TokenMarket.id();
        contract.set_id(id);
        resolver.add_known_data_contract(Arc::new(contract));
        let pv14 = PlatformVersion::get(14).expect("protocol version 14");
        let pv15 = PlatformVersion::get(15).expect("protocol version 15");

        assert!(resolver.data_contract(&id, pv14).unwrap().is_none());
        assert!(ContractSource::get_data_contract(&resolver, &id, pv14)
            .unwrap()
            .is_none());
        assert_eq!(
            resolver
                .data_contract(&id, pv15)
                .unwrap()
                .map(|contract| contract.id()),
            Some(id)
        );
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
        let after_same = resolver.get_known_contract(&id).expect("cached contract");
        assert_eq!(after_same.version(), 2);
        assert!(
            !Arc::ptr_eq(&cached, &after_same),
            "an equal version replaces the held body, as in the verified provider's cache"
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

    /// The SPV and adaptive providers register contracts only through the
    /// resolver. A registration that waited on its fetch must not replace a
    /// newer contract another operation registered meanwhile, or proofs over
    /// the newer version would be checked against the older one.
    #[test]
    fn should_keep_a_newer_contract_over_a_registered_older_one() {
        let resolver = LocalContextResolver::new();
        let at_version = |version| {
            let mut contract = app_contract();
            contract.set_version(version);
            contract
        };
        let register = |version| {
            ContractSource::register_data_contract(&resolver, Arc::new(at_version(version)))
        };
        let id = app_contract().id();
        let held = || {
            resolver
                .get_known_contract(&id)
                .map(|contract| contract.version())
        };

        register(2);
        register(3);
        assert_eq!(held(), Some(3), "a newer registration replaces");
        register(2);
        assert_eq!(held(), Some(3), "an older registration does not");
        resolver.replace_known_contract(at_version(1));
        assert_eq!(held(), Some(1), "an explicit replacement replaces");
    }
}
