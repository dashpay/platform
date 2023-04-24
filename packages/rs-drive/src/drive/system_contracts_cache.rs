use crate::error::Error;
use dpp::data_contract::DataContract;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};

pub struct SystemContracts {
    /// Withdrawal contract
    pub withdrawal_contract: DataContract,
    /// Masternode reward shares contract
    pub masternode_rewards: DataContract,
}

impl SystemContracts {
    pub fn load_system_contracts() -> Result<Self, Error> {
        Ok(SystemContracts {
            withdrawal_contract: load_system_data_contract(SystemDataContract::Withdrawals)?,
            masternode_rewards: load_system_data_contract(SystemDataContract::MasternodeRewards)?,
        })
    }
}
