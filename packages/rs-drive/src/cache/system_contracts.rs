use crate::error::Error;
use arc_swap::{ArcSwap, Guard};
use dpp::data_contract::DataContract;
use dpp::prelude::Identifier;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::ProtocolError;
use parking_lot::RwLock;
use platform_version::version::{PlatformVersion, ProtocolVersion};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The protocol version at which the document history contract activates. Its
/// schema uses index aggregation keywords (`averageable`, `rangeCountable`,
/// ...) that earlier document meta-schemas do not recognize, so the contract
/// is always loaded and validated under this version regardless of the
/// version the cache is being (re)loaded at.
pub const DOCUMENT_HISTORY_ACTIVATION_PROTOCOL_VERSION: ProtocolVersion = 13;

/// A wrapper around a system [`DataContract`] that tracks its activation version
/// and cached schema revisions.
///
/// This is used for system data contracts that may be updated over time while
/// tracking the protocol version from which they are considered active.
pub struct ActiveSystemDataContract {
    /// The current active version of the data contract.
    pub contract: ArcSwap<DataContract>,

    /// Materialized contract revisions keyed by protocol version.
    contracts_by_protocol_version: RwLock<BTreeMap<ProtocolVersion, Arc<DataContract>>>,

    /// The protocol version since which this contract is considered active.
    #[allow(unused)]
    pub active_since_protocol_version: ProtocolVersion,
}

impl ActiveSystemDataContract {
    /// Stores a versioned contract revision and makes it the current contract.
    ///
    /// # Arguments
    ///
    /// * `contract` - The new [`DataContract`] to store.
    /// * `protocol_version` - The protocol version used to materialize the contract.
    pub fn store(&self, contract: DataContract, protocol_version: ProtocolVersion) {
        let contract = Arc::new(contract);
        let mut contracts_by_protocol_version = self.contracts_by_protocol_version.write();
        contracts_by_protocol_version.insert(protocol_version, Arc::clone(&contract));
        self.contract.store(contract);
    }

    /// Loads the current data contract.
    ///
    /// Returns a guard that provides shared access to the current [`DataContract`].
    /// The guard keeps the contract alive for the duration of the borrow.
    pub fn load(&self) -> Guard<Arc<DataContract>> {
        self.contract.load()
    }

    /// Loads a cached contract materialized for a protocol version.
    fn load_for_protocol_version(
        &self,
        protocol_version: ProtocolVersion,
    ) -> Option<Arc<DataContract>> {
        self.contracts_by_protocol_version
            .read()
            .get(&protocol_version)
            .cloned()
    }

    /// Creates a new [`ActiveSystemDataContract`] with the given contract and activation version.
    ///
    /// # Arguments
    ///
    /// * `contract` - The initial [`DataContract`] to store.
    /// * `active_since_protocol_version` - The protocol version from which this contract is considered active.
    /// Creates a contract cache with an explicitly versioned initial materialization.
    pub fn new(
        contract: DataContract,
        active_since_protocol_version: ProtocolVersion,
        protocol_version: ProtocolVersion,
    ) -> Self {
        let contract = Arc::new(contract);
        let contracts_by_protocol_version =
            BTreeMap::from([(protocol_version, Arc::clone(&contract))]);

        ActiveSystemDataContract {
            contract: ArcSwap::new(contract),
            contracts_by_protocol_version: RwLock::new(contracts_by_protocol_version),
            active_since_protocol_version,
        }
    }
}

/// System contracts
pub struct SystemDataContracts {
    /// Withdrawal contract
    withdrawals: ActiveSystemDataContract,
    /// DPNS contract
    dpns: ActiveSystemDataContract,
    /// Dashpay contract
    dashpay: ActiveSystemDataContract,
    /// Masternode reward shares contract
    masternode_reward_shares: ActiveSystemDataContract,
    /// Token history contract
    token_history: ActiveSystemDataContract,
    /// Search contract
    keyword_search: ActiveSystemDataContract,
    /// Document history contract
    document_history: ActiveSystemDataContract,
}

impl SystemDataContracts {
    /// Reload **all** core-protocol system contracts for the supplied platform version,
    /// atomically replacing the cached copies held in each `ArcSwap`.
    ///
    /// Call this after you upgrade `PlatformVersion` (e.g. when a protocol bump
    /// introduces new schemas for DPNS, Token History, etc.).
    ///
    /// # Errors
    /// Propagates any `Error` returned by `load_system_data_contract`.
    pub fn reload_system_contracts(&self, platform_version: &PlatformVersion) -> Result<(), Error> {
        use SystemDataContract::*;

        // 1. Load every contract fresh (fail fast on error).
        let withdrawals = load_system_data_contract(Withdrawals, platform_version)?;
        let dpns = load_system_data_contract(DPNS, platform_version)?;
        let dashpay = load_system_data_contract(Dashpay, platform_version)?;
        let masternode_reward_shares =
            load_system_data_contract(MasternodeRewards, platform_version)?;
        let token_history = load_system_data_contract(TokenHistory, platform_version)?;
        let keyword_search = load_system_data_contract(KeywordSearch, platform_version)?;
        let document_history = load_system_data_contract(
            DocumentHistory,
            PlatformVersion::get(DOCUMENT_HISTORY_ACTIVATION_PROTOCOL_VERSION)
                .map_err(ProtocolError::from)?,
        )?;

        // 2. Swap the cached Arcs — each swap is lock-free & O(1).
        let protocol_version = platform_version.protocol_version;
        self.withdrawals.store(withdrawals, protocol_version);
        self.dpns.store(dpns, protocol_version);
        self.dashpay.store(dashpay, protocol_version);
        self.masternode_reward_shares
            .store(masternode_reward_shares, protocol_version);
        self.token_history.store(token_history, protocol_version);
        self.keyword_search.store(keyword_search, protocol_version);
        self.document_history
            .store(document_history, protocol_version);

        Ok(())
    }

    /// load genesis system contracts
    pub fn load_genesis_system_contracts() -> Result<Self, Error> {
        // We should use the version where the contract became active for each data contract
        let first_platform_version = PlatformVersion::first();
        let document_history_platform_version =
            PlatformVersion::get(DOCUMENT_HISTORY_ACTIVATION_PROTOCOL_VERSION)
                .map_err(ProtocolError::from)?;

        Ok(Self {
            withdrawals: ActiveSystemDataContract::new(
                load_system_data_contract(SystemDataContract::Withdrawals, first_platform_version)?,
                1,
                first_platform_version.protocol_version,
            ),
            dpns: ActiveSystemDataContract::new(
                load_system_data_contract(SystemDataContract::DPNS, first_platform_version)?,
                1,
                first_platform_version.protocol_version,
            ),
            dashpay: ActiveSystemDataContract::new(
                load_system_data_contract(SystemDataContract::Dashpay, first_platform_version)?,
                1,
                first_platform_version.protocol_version,
            ),
            masternode_reward_shares: ActiveSystemDataContract::new(
                load_system_data_contract(
                    SystemDataContract::MasternodeRewards,
                    first_platform_version,
                )?,
                1,
                first_platform_version.protocol_version,
            ),
            token_history: ActiveSystemDataContract::new(
                load_system_data_contract(
                    SystemDataContract::TokenHistory,
                    first_platform_version,
                )?,
                9,
                first_platform_version.protocol_version,
            ),
            keyword_search: ActiveSystemDataContract::new(
                load_system_data_contract(
                    SystemDataContract::KeywordSearch,
                    first_platform_version,
                )?,
                9,
                first_platform_version.protocol_version,
            ),
            document_history: ActiveSystemDataContract::new(
                load_system_data_contract(
                    SystemDataContract::DocumentHistory,
                    document_history_platform_version,
                )?,
                DOCUMENT_HISTORY_ACTIVATION_PROTOCOL_VERSION,
                document_history_platform_version.protocol_version,
            ),
        })
    }

    /// Returns withdrawals contract
    pub fn load_withdrawals(&self) -> Guard<Arc<DataContract>> {
        self.withdrawals.load()
    }

    /// Returns token history contract
    pub fn load_token_history(&self) -> Guard<Arc<DataContract>> {
        self.token_history.load()
    }

    /// Returns DPNS contract
    pub fn load_dpns(&self) -> Guard<Arc<DataContract>> {
        self.dpns.load()
    }

    /// Returns Dashpay contract
    pub fn load_dashpay(&self) -> Guard<Arc<DataContract>> {
        self.dashpay.load()
    }

    /// Returns Masternode reward shares contract
    pub fn load_masternode_reward_shares(&self) -> Guard<Arc<DataContract>> {
        self.masternode_reward_shares.load()
    }

    /// Returns the search contract
    pub fn load_keyword_search(&self) -> Guard<Arc<DataContract>> {
        self.keyword_search.load()
    }

    /// Returns the document history contract
    pub fn load_document_history(&self) -> Guard<Arc<DataContract>> {
        self.document_history.load()
    }

    /// Returns the cached system contract whose deterministic identifier matches `id`,
    /// if any. Returns `None` for user contracts, for any system contract whose
    /// definition isn't held in this in-memory cache (e.g. `WalletUtils`, which lives
    /// only in grovedb), and for system contracts that are not yet active at the
    /// given protocol version: before activation the contract does not exist in the
    /// state, so pre-activation lookups must fall through to the billed grovedb
    /// fetch and report it absent exactly like a non-upgraded node would.
    pub fn find_by_id(
        &self,
        id: Identifier,
        protocol_version: ProtocolVersion,
    ) -> Option<Arc<DataContract>> {
        // Compare against each cached system contract's static `id_bytes`. The match
        // is `O(n)` over a small fixed set of variants — cheaper than building a map.
        let platform_version = PlatformVersion::get(protocol_version).ok()?;
        let (active, system_contract) = if id == SystemDataContract::Withdrawals.id() {
            (&self.withdrawals, SystemDataContract::Withdrawals)
        } else if id == SystemDataContract::MasternodeRewards.id() {
            (
                &self.masternode_reward_shares,
                SystemDataContract::MasternodeRewards,
            )
        } else if id == SystemDataContract::DPNS.id() {
            (&self.dpns, SystemDataContract::DPNS)
        } else if id == SystemDataContract::Dashpay.id() {
            (&self.dashpay, SystemDataContract::Dashpay)
        } else if id == SystemDataContract::TokenHistory.id() {
            (&self.token_history, SystemDataContract::TokenHistory)
        } else if id == SystemDataContract::KeywordSearch.id() {
            (&self.keyword_search, SystemDataContract::KeywordSearch)
        } else if id == SystemDataContract::DocumentHistory.id() {
            (&self.document_history, SystemDataContract::DocumentHistory)
        } else {
            return None;
        };
        if active.active_since_protocol_version > protocol_version {
            return None;
        }
        if let Some(contract) = active.load_for_protocol_version(protocol_version) {
            return Some(contract);
        }

        let contract = Arc::new(load_system_data_contract(system_contract, platform_version).ok()?);
        let mut contracts_by_protocol_version = active.contracts_by_protocol_version.write();
        Some(Arc::clone(
            contracts_by_protocol_version
                .entry(protocol_version)
                .or_insert(contract),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

    fn dpns_history_flags(contract: &DataContract) -> (bool, bool, bool) {
        let domain = contract
            .document_type_for_name("domain")
            .expect("DPNS must define the domain document type");

        (
            domain.documents_keep_transfer_history(),
            domain.documents_keep_purchase_history(),
            domain.documents_keep_pricing_history(),
        )
    }

    #[test]
    fn reloading_v13_must_not_make_dpns_v2_visible_to_v12_reads() {
        let contracts =
            SystemDataContracts::load_genesis_system_contracts().expect("load system contracts");

        contracts
            .reload_system_contracts(PlatformVersion::get(12).expect("platform version 12"))
            .expect("load protocol v12 contracts");

        assert_eq!(
            dpns_history_flags(
                &contracts
                    .find_by_id(SystemDataContract::DPNS.id(), 12)
                    .expect("DPNS must be active at protocol v12")
            ),
            (false, false, false)
        );

        contracts
            .reload_system_contracts(PlatformVersion::get(13).expect("platform version 13"))
            .expect("speculatively load protocol v13 contracts");

        assert_eq!(
            dpns_history_flags(
                &contracts
                    .find_by_id(SystemDataContract::DPNS.id(), 12)
                    .expect("DPNS must remain available at protocol v12")
            ),
            (false, false, false),
            "a speculative v13 cache reload must preserve explicit v12 reads"
        );
        assert_eq!(
            dpns_history_flags(
                &contracts
                    .find_by_id(SystemDataContract::DPNS.id(), 13)
                    .expect("DPNS must be active at protocol v13")
            ),
            (true, true, true)
        );
    }

    #[test]
    fn same_feature_version_must_preserve_distinct_protocol_materializations() {
        let contracts =
            SystemDataContracts::load_genesis_system_contracts().expect("load system contracts");
        let platform_version_8 = PlatformVersion::get(8).expect("platform version 8");
        let platform_version_9 = PlatformVersion::get(9).expect("platform version 9");
        let expected_v8 =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version_8)
                .expect("load protocol v8 withdrawals");
        let expected_v9 =
            load_system_data_contract(SystemDataContract::Withdrawals, platform_version_9)
                .expect("load protocol v9 withdrawals");
        assert_ne!(
            expected_v8, expected_v9,
            "the regression requires distinct materialized contracts"
        );

        contracts
            .reload_system_contracts(platform_version_8)
            .expect("load protocol v8 contracts");
        contracts
            .reload_system_contracts(platform_version_9)
            .expect("speculatively load protocol v9 contracts");

        assert_eq!(
            contracts
                .find_by_id(SystemDataContract::Withdrawals.id(), 8)
                .expect("withdrawals must be active at protocol v8")
                .as_ref(),
            &expected_v8,
            "a v9 reload must preserve the protocol v8 materialization"
        );
        assert_eq!(
            contracts
                .find_by_id(SystemDataContract::Withdrawals.id(), 9)
                .expect("withdrawals must be active at protocol v9")
                .as_ref(),
            &expected_v9
        );
    }

    #[test]
    fn document_history_cache_respects_its_activation_version() {
        let contracts =
            SystemDataContracts::load_genesis_system_contracts().expect("load system contracts");

        assert!(contracts
            .find_by_id(SystemDataContract::DocumentHistory.id(), 12)
            .is_none());
        assert!(contracts
            .find_by_id(SystemDataContract::DocumentHistory.id(), 13)
            .is_some());
    }
}
