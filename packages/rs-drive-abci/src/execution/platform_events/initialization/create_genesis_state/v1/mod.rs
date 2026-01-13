use crate::error::Error;
use crate::platform_types::platform::Platform;

use drive::dpp::identity::TimestampMillis;

use dpp::block::block_info::BlockInfo;
use dpp::prelude::CoreBlockHeight;
use dpp::system_data_contracts::load_system_data_contract;
use dpp::version::PlatformVersion;
use drive::dpp::system_data_contracts::SystemDataContract;
use drive::query::TransactionArg;
use std::collections::BTreeMap;

impl<C> Platform<C> {
    /// Creates trees and populates them with necessary identities, contracts and documents
    #[inline(always)]
    pub(super) fn create_genesis_state_v1(
        &self,
        genesis_core_height: CoreBlockHeight,
        genesis_time: TimestampMillis,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        //versioned call
        self.drive
            .create_initial_state_structure(transaction, platform_version)?;

        self.drive
            .store_genesis_core_height(genesis_core_height, transaction, platform_version)?;

        let mut operations = vec![];

        // Create system identities and contracts

        let system_data_contracts = &self.drive.cache.system_data_contracts;

        let system_data_contract_types = BTreeMap::from_iter([
            (SystemDataContract::DPNS, system_data_contracts.load_dpns()),
            (
                SystemDataContract::Withdrawals,
                system_data_contracts.load_withdrawals(),
            ),
            (
                SystemDataContract::Dashpay,
                system_data_contracts.load_dashpay(),
            ),
            (
                SystemDataContract::MasternodeRewards,
                system_data_contracts.load_masternode_reward_shares(),
            ),
            (
                SystemDataContract::TokenHistory,
                system_data_contracts.load_token_history(),
            ),
            (
                SystemDataContract::KeywordSearch,
                system_data_contracts.load_keyword_search(),
            ),
        ]);

        for data_contract in system_data_contract_types.values() {
            self.register_system_data_contract_operations(
                data_contract,
                &mut operations,
                platform_version,
            )?;
        }

        let wallet_utils_contract =
            load_system_data_contract(SystemDataContract::WalletUtils, platform_version)?;

        self.register_system_data_contract_operations(
            &wallet_utils_contract,
            &mut operations,
            platform_version,
        )?;

        let dpns_contract = system_data_contracts.load_dpns();

        self.register_dpns_top_level_domain_operations(
            &dpns_contract,
            genesis_time,
            &mut operations,
        )?;

        let block_info = BlockInfo::default_with_time(genesis_time);

        self.drive.apply_drive_operations(
            operations,
            true,
            &block_info,
            transaction,
            platform_version,
            None, // No previous_fee_versions needed for genesis state creation
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    mod create_genesis_state {
        use crate::config::PlatformConfig;
        use crate::test::helpers::setup::TestPlatformBuilder;
        use drive::config::DriveConfig;
        use platform_version::version::{PlatformVersion, INITIAL_PROTOCOL_VERSION};

        #[test]
        pub fn should_create_genesis_state_deterministically() {
            let platform_version = PlatformVersion::first();
            let platform = TestPlatformBuilder::new()
                .with_config(PlatformConfig {
                    drive: DriveConfig {
                        epochs_per_era: 20,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_initial_protocol_version(INITIAL_PROTOCOL_VERSION)
                .build_with_mock_rpc()
                .set_genesis_state();

            let root_hash = platform
                .drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should obtain root hash");

            // This should never be changed
            assert_eq!(
                hex::encode(root_hash),
                "dc5b0d4be407428adda2315db7d782e64015cbe2d2b7df963f05622390dc3c9f"
            )
        }
    }
}
