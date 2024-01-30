/// For testing only
use crate::platform_types::platform::Platform;
use dpp::block::block_info::BlockInfo;

use dpp::data_contract::DataContract;

use drive::drive::flags::StorageFlags;

use drive::grovedb::TransactionArg;

use dpp::data_contracts::SystemDataContract;
use dpp::system_data_contracts::load_system_data_contract;
use dpp::version::PlatformVersion;
use std::borrow::Cow;

impl<C> Platform<C> {
    /// A function to create and apply the masternode reward shares contract.
    pub fn create_mn_shares_contract(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let contract =
            load_system_data_contract(SystemDataContract::MasternodeRewards, &platform_version)
                .expect("should load masternode reward contract");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        self.drive
            .apply_contract(
                &contract,
                BlockInfo::genesis(),
                true,
                storage_flags,
                transaction,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        contract
    }
}
