use crate::error::Error;
use dpp::data_contract::DataContract;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use platform_version::version::PlatformVersion;

// TODO: Use ArcSwap
/// System contracts
pub struct SystemDataContracts {
    /// Withdrawal contract
    withdrawals: parking_lot::RwLock<DataContract>,
    /// DPNS contract
    dpns: parking_lot::RwLock<DataContract>,
    /// Dashpay contract
    dashpay: parking_lot::RwLock<DataContract>,
    /// Masternode reward shares contract
    masternode_reward_shares: parking_lot::RwLock<DataContract>,
}

impl SystemDataContracts {
    /// load genesis system contracts
    pub fn load_genesis_system_contracts(
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        Ok(Self {
            withdrawals: parking_lot::RwLock::new(load_system_data_contract(
                SystemDataContract::Withdrawals,
                platform_version,
            )?),
            dpns: parking_lot::RwLock::new(load_system_data_contract(
                SystemDataContract::DPNS,
                platform_version,
            )?),
            dashpay: parking_lot::RwLock::new(load_system_data_contract(
                SystemDataContract::Dashpay,
                platform_version,
            )?),
            masternode_reward_shares: parking_lot::RwLock::new(load_system_data_contract(
                SystemDataContract::MasternodeRewards,
                platform_version,
            )?),
        })
    }

    /// Returns withdrawals contract
    pub fn read_withdrawals(&self) -> parking_lot::RwLockReadGuard<DataContract> {
        self.withdrawals.read()
    }

    /// Returns DPNS contract
    pub fn read_dpns(&self) -> parking_lot::RwLockReadGuard<DataContract> {
        self.dpns.read()
    }

    /// Returns Dashpay contract
    pub fn read_dashpay(&self) -> parking_lot::RwLockReadGuard<DataContract> {
        self.dashpay.read()
    }

    /// Returns Masternode reward shares contract
    pub fn read_masternode_reward_shares(&self) -> parking_lot::RwLockReadGuard<DataContract> {
        self.masternode_reward_shares.read()
    }
}
