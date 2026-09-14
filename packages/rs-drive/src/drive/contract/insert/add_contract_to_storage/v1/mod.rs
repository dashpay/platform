use crate::drive::contract::paths::{contract_root_path, CONTRACT_VERSION_KEY};
use crate::drive::contract::version_item::encode_contract_version;
use crate::drive::Drive;
use crate::drive::LowLevelDriveOperation;
use crate::error::Error;
use crate::util::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElementSize,
};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Adds a contract to storage as v0 does, then writes the contract's version number as a
    /// four-byte item beside it, at key `2` of the contract's root subtree, whether the
    /// contract keeps history or not.
    ///
    /// The item is written on the first insert and overwritten on every update, so it always
    /// holds the version of the stored contract (for a contract that keeps history, of its
    /// latest revision). It carries the contract element's flags, so its storage is paid for
    /// and refunded with the contract's. `getDataContractsLatestVersions` reads and proves
    /// this item instead of the serialized contract.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contract_to_storage_v1(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        insert_operations: &mut Vec<LowLevelDriveOperation>,
        is_first_insert: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let element_flags = contract_element.get_flags().clone();

        self.add_contract_to_storage_v0(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            insert_operations,
            is_first_insert,
            transaction,
            drive_version,
        )?;

        let version_element =
            Element::Item(encode_contract_version(contract.version()), element_flags);
        let contract_root_path = contract_root_path(contract.id_ref().as_bytes());

        let path_key_element_info = if estimated_costs_only_with_layer_info.is_none() {
            PathFixedSizeKeyRefElement((
                contract_root_path,
                &[CONTRACT_VERSION_KEY],
                version_element,
            ))
        } else {
            PathKeyElementSize((
                KeyInfoPath::from_known_path(contract_root_path),
                KeyInfo::KnownKey(vec![CONTRACT_VERSION_KEY]),
                version_element,
            ))
        };

        self.batch_insert(path_key_element_info, insert_operations, drive_version)
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::DataContract;
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::version::PlatformVersion;

    fn dashpay_contract(keeps_history: bool, platform_version: &PlatformVersion) -> DataContract {
        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.config_mut().set_readonly(false);
        contract.config_mut().set_keeps_history(keeps_history);
        contract
    }

    fn block(time_ms: u64) -> BlockInfo {
        BlockInfo {
            time_ms,
            height: 100,
            core_height: 10,
            epoch: Default::default(),
        }
    }

    fn apply(
        drive: &Drive,
        contract: &DataContract,
        time_ms: u64,
        platform_version: &PlatformVersion,
    ) {
        drive
            .apply_contract(
                contract,
                block(time_ms),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply the contract");
    }

    fn stored_version(
        drive: &Drive,
        contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Option<u32> {
        drive
            .fetch_contract_version(contract.id().to_buffer(), None, platform_version)
            .expect("expected to read the contract version item")
    }

    #[test]
    fn should_write_the_version_item_on_insert_and_keep_it_current_on_update() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        for keeps_history in [false, true] {
            let mut contract = dashpay_contract(keeps_history, platform_version);
            contract.set_id([keeps_history as u8 + 1; 32].into());

            apply(&drive, &contract, 1000, platform_version);
            assert_eq!(
                stored_version(&drive, &contract, platform_version),
                Some(contract.version()),
                "keeps_history={keeps_history}: the item holds the inserted version"
            );

            contract.increment_version();
            apply(&drive, &contract, 2000, platform_version);
            assert_eq!(
                stored_version(&drive, &contract, platform_version),
                Some(contract.version()),
                "keeps_history={keeps_history}: the item follows the update"
            );

            let fetched = drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .value
                .expect("expected to fetch the contract")
                .expect("expected the contract to exist");
            assert_eq!(
                fetched.contract.version(),
                contract.version(),
                "the contract itself was updated too"
            );
        }
    }

    #[test]
    fn should_not_write_the_version_item_before_protocol_version_14() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        let contract = dashpay_contract(false, platform_version);

        apply(&drive, &contract, 1000, platform_version);

        assert_eq!(stored_version(&drive, &contract, platform_version), None);
    }

    #[test]
    fn should_estimate_an_insert_with_the_version_item() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract = dashpay_contract(false, platform_version);

        let estimated_fee = drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                false,
                None,
                platform_version,
            )
            .expect("expected the stateless insert estimate to succeed");
        let actual_fee = drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected the insert to succeed");

        assert!(
            estimated_fee.storage_fee >= actual_fee.storage_fee,
            "the estimate ({}) must not undershoot the actual storage fee ({})",
            estimated_fee.storage_fee,
            actual_fee.storage_fee
        );
        assert_eq!(
            stored_version(&drive, &contract, platform_version),
            Some(contract.version())
        );
    }
}
