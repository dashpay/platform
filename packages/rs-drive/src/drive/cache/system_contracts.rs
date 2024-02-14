use crate::error::Error;
use dpp::data_contract::DataContract;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use platform_version::version::PlatformVersion;
use std::sync::{RwLock, RwLockReadGuard};

/// System contracts
pub struct SystemDataContracts {
    /// Withdrawal contract
    withdrawals: RwLock<DataContract>,
    /// DPNS contract
    dpns: RwLock<DataContract>,
    /// Dashpay contract
    dashpay: RwLock<DataContract>,
    /// Masternode reward shares contract
    masternode_reward_shares: RwLock<DataContract>,
}

impl SystemDataContracts {
    /// load genesis system contracts
    pub fn load_genesis_system_contracts(
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        Ok(Self {
            withdrawals: RwLock::new(load_system_data_contract(
                SystemDataContract::Withdrawals,
                platform_version,
            )?),
            dpns: RwLock::new(load_system_data_contract(
                SystemDataContract::DPNS,
                platform_version,
            )?),
            dashpay: RwLock::new(load_system_data_contract(
                SystemDataContract::Dashpay,
                platform_version,
            )?),
            masternode_reward_shares: RwLock::new(load_system_data_contract(
                SystemDataContract::MasternodeRewards,
                platform_version,
            )?),
        })
    }

    /// Returns withdrawals contract
    pub fn withdrawals(&self) -> RwLockReadGuard<DataContract> {
        self.withdrawals.read().unwrap()
    }

    /// Returns DPNS contract
    pub fn dpns(&self) -> RwLockReadGuard<DataContract> {
        self.dpns.read().unwrap()
    }

    /// Returns Dashpay contract
    pub fn dashpay(&self) -> RwLockReadGuard<DataContract> {
        self.dashpay.read().unwrap()
    }

    /// Returns Masternode reward shares contract
    pub fn masternode_reward_shares(&self) -> RwLockReadGuard<DataContract> {
        self.masternode_reward_shares.read().unwrap()
    }
}
