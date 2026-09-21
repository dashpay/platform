use crate::error::Error;
use crate::platform_types::platform::Platform;

use drive::dpp::identity::TimestampMillis;

use dpp::data_contract::DataContract;
use dpp::prelude::CoreBlockHeight;
use dpp::version::PlatformVersion;
use drive::dpp::system_data_contracts::SystemDataContract;
use drive::query::TransactionArg;
use std::collections::BTreeMap;
use std::sync::Arc;

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
        self.create_genesis_state_with_system_data_contracts(
            genesis_core_height,
            genesis_time,
            self.genesis_v1_system_data_contracts(platform_version)?,
            transaction,
            platform_version,
        )
    }

    /// The system contracts genesis generation v1 registers, in registration order: the
    /// four original contracts plus tokens and keyword search, and from protocol version
    /// 13 the document history contract. Later generations extend this list.
    pub(super) fn genesis_v1_system_data_contracts(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<SystemDataContract, Arc<DataContract>>, Error> {
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

        Ok(system_data_contract_types)
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
