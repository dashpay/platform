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
    /// Creates trees and populates them with necessary identities, contracts and documents.
    ///
    /// v2 is v1 (protocol versions 6 to 13) plus the app-connect contract, which activates with
    /// protocol version 14. Both the document history contract (v1 registered it from protocol
    /// version 13 behind a version check) and the app-connect contract are unconditional here:
    /// this generation is only ever selected by the tables of protocol version 14 and later.
    #[inline(always)]
    pub(super) fn create_genesis_state_v2(
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
            (
                SystemDataContract::DPNS,
                system_data_contracts.load_dpns(platform_version)?,
            ),
            (
                SystemDataContract::Withdrawals,
                system_data_contracts.load_withdrawals(platform_version)?,
            ),
            (
                SystemDataContract::Dashpay,
                system_data_contracts.load_dashpay(platform_version)?,
            ),
            (
                SystemDataContract::MasternodeRewards,
                system_data_contracts.load_masternode_reward_shares(platform_version)?,
            ),
            (
                SystemDataContract::TokenHistory,
                system_data_contracts.load_token_history(platform_version)?,
            ),
            (
                SystemDataContract::KeywordSearch,
                system_data_contracts.load_keyword_search(platform_version)?,
            ),
            (
                SystemDataContract::DocumentHistory,
                system_data_contracts.load_document_history(platform_version)?,
            ),
            (
                SystemDataContract::AppConnect,
                system_data_contracts.load_app_connect(platform_version)?,
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

        let dpns_contract = system_data_contracts.load_dpns(platform_version)?;

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
        use crate::test::helpers::setup::TestPlatformBuilder;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contracts::SystemDataContract;
        use platform_version::version::PlatformVersion;

        /// The app-connect contract is part of the genesis state from protocol version 14
        /// on, where the tables select this generation, and absent from the genesis state of
        /// every earlier version, which still runs v1 and which a replaying node must
        /// reproduce byte for byte. Both sides go through the `create_genesis_state`
        /// dispatcher.
        #[test]
        pub fn should_register_the_app_connect_contract_only_from_protocol_version_14() {
            let app_connect_id = SystemDataContract::AppConnect.id();

            for (initial_protocol_version, expected_method_version, expected) in
                [(13, 1, false), (14, 2, true)]
            {
                let platform_version = PlatformVersion::get(initial_protocol_version)
                    .expect("expected a supported platform version");
                assert_eq!(
                    platform_version
                        .drive_abci
                        .methods
                        .initialization
                        .create_genesis_state,
                    expected_method_version,
                    "protocol version {initial_protocol_version} must dispatch to create_genesis_state v{expected_method_version}"
                );

                let platform = TestPlatformBuilder::new()
                    .with_initial_protocol_version(initial_protocol_version)
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let stored = platform
                    .drive
                    .fetch_contract(
                        app_connect_id.to_buffer(),
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .value
                    .expect("expected to query the app-connect contract");

                assert_eq!(
                    stored.is_some(),
                    expected,
                    "app-connect contract presence in a genesis state born at protocol version {initial_protocol_version}"
                );

                if let Some(stored) = stored {
                    assert_eq!(stored.contract.id(), app_connect_id);
                    assert!(stored
                        .contract
                        .document_type_for_name("loginKeyResponse")
                        .is_ok());
                    assert!(stored
                        .contract
                        .document_type_for_name("appManifest")
                        .is_ok());
                }
            }
        }
    }
}
