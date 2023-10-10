use crate::error::Error;
use dpp::data_contract::DataContract;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};

/// System contracts
pub struct SystemContracts {
    /// Withdrawal contract
    pub withdrawal_contract: DataContract,
    /// DPNS contract
    pub dpns_contract: DataContract,
    /// Dashpay contract
    pub dashpay_contract: DataContract,
    /// Masternode reward shares contract
    pub masternode_rewards: DataContract,
}

impl SystemContracts {
    /// load genesis system contracts
    pub fn load_genesis_system_contracts(protocol_version: u32) -> Result<Self, Error> {
        Ok(SystemContracts {
            withdrawal_contract: load_system_data_contract(
                SystemDataContract::Withdrawals,
                protocol_version,
            )?,
            dpns_contract: load_system_data_contract(SystemDataContract::DPNS, protocol_version)?,
            dashpay_contract: load_system_data_contract(
                SystemDataContract::Dashpay,
                protocol_version,
            )?,
            masternode_rewards: load_system_data_contract(
                SystemDataContract::MasternodeRewards,
                protocol_version,
            )?,
        })
    }
}
