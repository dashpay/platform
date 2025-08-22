use crate::error::Error;
use arc_swap::{ArcSwap, Guard};
use dpp::data_contract::DataContract;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use platform_version::version::{PlatformVersion, ProtocolVersion};
use std::sync::Arc;

/// A wrapper around a system [`DataContract`] that tracks its activation version
/// and allows atomic replacement.
///
/// This is used for system data contracts that may be updated over time while
/// tracking the protocol version from which they are considered active.
pub struct ActiveSystemDataContract {
    /// The current active version of the data contract.
    pub contract: ArcSwap<DataContract>,

    /// The protocol version since which this contract is considered active.
    #[allow(unused)]
    pub active_since_protocol_version: ProtocolVersion,
}

impl ActiveSystemDataContract {
    /// Atomically replaces the current data contract with a new one.
    ///
    /// # Arguments
    ///
    /// * `contract` - The new [`DataContract`] to store.
    pub fn store(&self, contract: DataContract) {
        self.contract.store(Arc::new(contract));
    }

    /// Loads the current data contract.
    ///
    /// Returns a guard that provides shared access to the current [`DataContract`].
    /// The guard keeps the contract alive for the duration of the borrow.
    pub fn load(&self) -> Guard<Arc<DataContract>> {
        self.contract.load()
    }

    /// Creates a new [`ActiveSystemDataContract`] with the given contract and activation version.
    ///
    /// # Arguments
    ///
    /// * `contract` - The initial [`DataContract`] to store.
    /// * `active_since_protocol_version` - The protocol version from which this contract is considered active.
    pub fn new(contract: DataContract, active_since_protocol_version: ProtocolVersion) -> Self {
        ActiveSystemDataContract {
            contract: ArcSwap::from_pointee(contract),
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

        // 2. Swap the cached Arcs — each swap is lock-free & O(1).
        self.withdrawals.store(withdrawals);
        self.dpns.store(dpns);
        self.dashpay.store(dashpay);
        self.masternode_reward_shares
            .store(masternode_reward_shares);
        self.token_history.store(token_history);
        self.keyword_search.store(keyword_search);

        Ok(())
    }

    /// load genesis system contracts
    pub fn load_genesis_system_contracts() -> Result<Self, Error> {
        // We should use the version where the contract became active for each data contract
        Ok(Self {
            withdrawals: ActiveSystemDataContract::new(
                load_system_data_contract(
                    SystemDataContract::Withdrawals,
                    PlatformVersion::first(),
                )?,
                1,
            ),
            dpns: ActiveSystemDataContract::new(
                load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::first())?,
                1,
            ),
            dashpay: ActiveSystemDataContract::new(
                load_system_data_contract(SystemDataContract::Dashpay, PlatformVersion::first())?,
                1,
            ),
            masternode_reward_shares: ActiveSystemDataContract::new(
                load_system_data_contract(
                    SystemDataContract::MasternodeRewards,
                    PlatformVersion::first(),
                )?,
                1,
            ),
            token_history: ActiveSystemDataContract::new(
                load_system_data_contract(
                    SystemDataContract::TokenHistory,
                    PlatformVersion::first(),
                )?,
                9,
            ),
            keyword_search: ActiveSystemDataContract::new(
                load_system_data_contract(
                    SystemDataContract::KeywordSearch,
                    PlatformVersion::first(),
                )?,
                9,
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
}
