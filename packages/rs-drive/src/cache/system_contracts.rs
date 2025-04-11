use crate::error::Error;
use arc_swap::{ArcSwap, Guard};
use dpp::data_contract::DataContract;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use platform_version::version::PlatformVersion;
use std::sync::Arc;

/// System contracts
pub struct SystemDataContracts {
    /// Withdrawal contract
    withdrawals: ArcSwap<DataContract>,
    /// DPNS contract
    dpns: ArcSwap<DataContract>,
    /// Dashpay contract
    dashpay: ArcSwap<DataContract>,
    /// Masternode reward shares contract
    masternode_reward_shares: ArcSwap<DataContract>,
    /// Token history contract
    token_history: ArcSwap<DataContract>,
    /// Search contract
    keyword_search: ArcSwap<DataContract>,
}

impl SystemDataContracts {
    /// load genesis system contracts
    pub fn load_genesis_system_contracts(
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        Ok(Self {
            withdrawals: ArcSwap::from_pointee(load_system_data_contract(
                SystemDataContract::Withdrawals,
                platform_version,
            )?),
            dpns: ArcSwap::from_pointee(load_system_data_contract(
                SystemDataContract::DPNS,
                platform_version,
            )?),
            dashpay: ArcSwap::from_pointee(load_system_data_contract(
                SystemDataContract::Dashpay,
                platform_version,
            )?),
            masternode_reward_shares: ArcSwap::from_pointee(load_system_data_contract(
                SystemDataContract::MasternodeRewards,
                platform_version,
            )?),
            token_history: ArcSwap::from_pointee(load_system_data_contract(
                SystemDataContract::TokenHistory,
                platform_version,
            )?),
            keyword_search: ArcSwap::from_pointee(load_system_data_contract(
                SystemDataContract::KeywordSearch,
                platform_version,
            )?),
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
