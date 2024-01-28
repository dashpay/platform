use crate::error::Error;
use dpp::data_contract::DataContract;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use platform_version::version::PlatformVersion;

/// System contracts
pub struct SystemDataContracts {
    /// Withdrawal contract
    pub withdrawals: DataContract,
    /// DPNS contract
    pub dpns: DataContract,
    /// Dashpay contract
    pub dashpay: DataContract,
    /// Masternode reward shares contract
    pub masternode_reward_shares: DataContract,
}

impl SystemDataContracts {
    /// load genesis system contracts
    pub fn load_genesis_system_contracts(
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        Ok(Self {
            withdrawals: load_system_data_contract(
                SystemDataContract::Withdrawals,
                platform_version,
            )?,
            dpns: load_system_data_contract(SystemDataContract::DPNS, platform_version)?,
            dashpay: load_system_data_contract(SystemDataContract::Dashpay, platform_version)?,
            masternode_reward_shares: load_system_data_contract(
                SystemDataContract::MasternodeRewards,
                platform_version,
            )?,
        })
    }
}
