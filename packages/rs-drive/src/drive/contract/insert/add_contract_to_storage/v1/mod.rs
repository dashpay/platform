use crate::drive::contract::paths::{
    contract_other_path, contract_root_path, CONTRACT_OTHER_KEY, CONTRACT_VERSION_KEY,
};
use crate::drive::contract::version_item::encode_contract_version;
use crate::drive::Drive;
use crate::drive::LowLevelDriveOperation;
use crate::error::Error;
use crate::util::grove_operations::{BatchInsertTreeApplyType, DirectQueryType};
use crate::util::object_size_info::DriveKeyInfo;
use crate::util::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElementSize,
};
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKeyRef;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Adds a contract to storage as v0 does, then writes the contract's version number as a
    /// four-byte item in the contract's other tree (`[64, id, 2] / 64`), whether the contract
    /// keeps history or not.
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

        // The version item lives in the contract's other tree (`[64, id, 2]`). An insertion
        // writes the tree unconditionally, like the contract's documents tree beside it: the
        // contract's root subtree is (re)created in the same batch, so whatever state holds
        // under it is gone anyway. An update finds the tree, from the insertion or from the
        // migration on the first block of protocol version 14, so its estimate writes none;
        // applied, it still inserts the tree `if not exists` rather than trusting that: it must
        // never be replaced, it may hold the moderation lists.
        let storage_flags = StorageFlags::map_some_element_flags_ref(&element_flags)?;
        if is_first_insert {
            self.batch_insert_empty_tree(
                contract_root_path(contract.id_ref().as_bytes()),
                DriveKeyInfo::Key(vec![CONTRACT_OTHER_KEY]),
                storage_flags.as_ref(),
                insert_operations,
                drive_version,
            )?;
        } else if estimated_costs_only_with_layer_info.is_none() {
            let inserted = self.batch_insert_empty_tree_if_not_exists(
                PathFixedSizeKeyRef((
                    contract_root_path(contract.id_ref().as_bytes()),
                    &[CONTRACT_OTHER_KEY],
                )),
                TreeType::NormalTree,
                storage_flags.as_ref(),
                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                transaction,
                &mut None,
                insert_operations,
                drive_version,
            )?;
            // The 4.2 betas wrote the version item itself at this key (`[64, id] / 2`), before
            // the other tree existed. A contract stored by one of them still holds that item,
            // and the write below needs a tree there, so the tree takes the item's place. The
            // look is not billed: no contract stored by a release has the item, and an update
            // must cost the same whether or not a beta ever ran on the network.
            if !inserted
                && self.contract_other_key_holds_the_beta_version_item(
                    contract.id_ref().as_bytes(),
                    transaction,
                    drive_version,
                )?
            {
                self.batch_insert_empty_tree(
                    contract_root_path(contract.id_ref().as_bytes()),
                    DriveKeyInfo::Key(vec![CONTRACT_OTHER_KEY]),
                    storage_flags.as_ref(),
                    insert_operations,
                    drive_version,
                )?;
            }
        }
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_contract_other_tree(
                contract.id_ref().to_buffer(),
                estimated_costs_only_with_layer_info,
            );
        }

        let version_element =
            Element::Item(encode_contract_version(contract.version()), element_flags);
        let contract_other_path = contract_other_path(contract.id_ref().as_bytes());

        let path_key_element_info = if estimated_costs_only_with_layer_info.is_none() {
            PathFixedSizeKeyRefElement((
                contract_other_path,
                &[CONTRACT_VERSION_KEY],
                version_element,
            ))
        } else {
            PathKeyElementSize((
                KeyInfoPath::from_known_path(contract_other_path),
                KeyInfo::KnownKey(vec![CONTRACT_VERSION_KEY]),
                version_element,
            ))
        };

        self.batch_insert(path_key_element_info, insert_operations, drive_version)
    }

    /// Whether key `2` of the contract's root subtree holds an item rather than the other tree:
    /// the layout of the 4.2 betas, which kept the version item there.
    fn contract_other_key_holds_the_beta_version_item(
        &self,
        contract_id: &[u8; 32],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<bool, Error> {
        let element = self.grove_get_raw_optional(
            (&contract_root_path(contract_id)).into(),
            &[CONTRACT_OTHER_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            drive_version,
        )?;
        Ok(matches!(element, Some(Element::Item(..))))
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
    fn should_give_a_contract_stored_by_a_beta_its_other_tree_on_update() {
        use crate::drive::contract::paths::{
            contract_other_path, contract_root_path, CONTRACT_OTHER_KEY, CONTRACT_VERSION_KEY,
        };
        use crate::drive::contract::version_item::encode_contract_version;
        use grovedb::Element;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let grove_version = &platform_version.drive.grove_version;

        let mut contract = dashpay_contract(false, platform_version);
        apply(&drive, &contract, 1000, platform_version);

        // Back to the layout the 4.2 betas wrote: the version item itself at `[64, id] / 2`.
        let contract_id = contract.id().to_buffer();
        drive
            .grove
            .delete(
                &contract_other_path(&contract_id),
                &[CONTRACT_VERSION_KEY],
                None,
                None,
                grove_version,
            )
            .unwrap()
            .expect("expected to delete the version item");
        drive
            .grove
            .delete(
                &contract_root_path(&contract_id),
                &[CONTRACT_OTHER_KEY],
                None,
                None,
                grove_version,
            )
            .unwrap()
            .expect("expected to delete the other tree");
        drive
            .grove
            .insert(
                &contract_root_path(&contract_id),
                &[CONTRACT_OTHER_KEY],
                Element::new_item(encode_contract_version(contract.version())),
                None,
                None,
                grove_version,
            )
            .unwrap()
            .expect("expected to write the beta version item");

        // Until the contract is updated, a read of its version item goes through a key that is
        // not a tree, and fails. Only `getDataContractsLatestVersions` reads it, not consensus.
        assert!(drive
            .fetch_contract_version(contract_id, None, platform_version)
            .is_err());

        contract.increment_version();
        apply(&drive, &contract, 2000, platform_version);

        assert_eq!(
            stored_version(&drive, &contract, platform_version),
            Some(contract.version()),
            "the update put the other tree in the item's place and the item under it"
        );
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
