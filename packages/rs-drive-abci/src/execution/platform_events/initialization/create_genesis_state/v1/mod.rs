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

        let mut system_data_contract_types = BTreeMap::from_iter([
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
        ]);

        // The document history contract activates with protocol version 13: a
        // chain born (or deterministically replayed) at an earlier version
        // must produce the exact genesis state the pre-13 binaries produced
        if platform_version.protocol_version >= 13 {
            system_data_contract_types.insert(
                SystemDataContract::DocumentHistory,
                system_data_contracts.load_document_history(platform_version)?,
            );
        }

        // The app-connect contract activates with protocol version 14, the same
        // way: a chain born at 14 registers it at genesis, an older chain gets it
        // from `transition_to_version_14`. Mainnet and testnet replay v0, and every
        // chain born at 9 to 13 is an ephemeral network, so the branch changes no
        // genesis a live node reproduces.
        if platform_version.protocol_version >= 14 {
            system_data_contract_types.insert(
                SystemDataContract::AppConnect,
                system_data_contracts.load_app_connect(platform_version)?,
            );
            // The moderation charters contract activates with the same version and branch.
            system_data_contract_types.insert(
                SystemDataContract::ModerationCharters,
                system_data_contracts.load_moderation_charters(platform_version)?,
            );
        }

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
        use crate::config::PlatformConfig;
        use crate::test::helpers::setup::TestPlatformBuilder;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contracts::SystemDataContract;
        use dpp::prelude::Identifier;
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

        /// The app-connect contract is part of the genesis state from protocol version 14
        /// on, where the tables select this generation, and absent from the genesis state of
        /// every earlier version, which still runs v1 and which a replaying node must
        /// reproduce byte for byte. Both sides go through the `create_genesis_state`
        /// dispatcher.
        #[test]
        pub fn should_register_the_app_connect_contract_only_from_protocol_version_14() {
            let app_connect_id = SystemDataContract::AppConnect.id();

            for (platform_version, expected) in [
                (PlatformVersion::get(13).expect("protocol 13"), false),
                (PlatformVersion::latest(), true),
            ] {
                let initial_protocol_version = platform_version.protocol_version;
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
                    assert_eq!(stored.contract.document_types().len(), 1);
                }
            }
        }

        /// The moderation charters contract joins the genesis state at protocol version 14,
        /// behind the same branch as the app-connect contract.
        #[test]
        pub fn should_register_the_moderation_charters_contract_only_from_protocol_version_14() {
            let moderation_charters_id = SystemDataContract::ModerationCharters.id();

            for (platform_version, expected) in [
                (PlatformVersion::get(13).expect("protocol 13"), false),
                (PlatformVersion::latest(), true),
            ] {
                let initial_protocol_version = platform_version.protocol_version;
                let platform = TestPlatformBuilder::new()
                    .with_initial_protocol_version(initial_protocol_version)
                    .build_with_mock_rpc()
                    .set_genesis_state();

                let stored = platform
                    .drive
                    .fetch_contract(
                        moderation_charters_id.to_buffer(),
                        None,
                        None,
                        None,
                        platform_version,
                    )
                    .value
                    .expect("expected to query the moderation charters contract");

                assert_eq!(
                    stored.is_some(),
                    expected,
                    "moderation charters contract presence in a genesis state born at protocol version {initial_protocol_version}"
                );

                if let Some(stored) = stored {
                    assert_eq!(stored.contract.id(), moderation_charters_id);
                    assert_eq!(stored.contract.owner_id(), Identifier::from([0u8; 32]));
                    assert!(stored
                        .contract
                        .document_type_for_name("electedCharter")
                        .is_ok());
                    assert_eq!(stored.contract.document_types().len(), 7);
                }
            }
        }
    }
}
