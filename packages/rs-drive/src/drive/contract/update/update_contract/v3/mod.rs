use crate::drive::document::paths::{contract_document_type_path, DOCUMENT_HISTORY_TREE_KEY};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DriveKeyInfo::KeyRef;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::serialization::PlatformSerializableWithPlatformVersion;

use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Updates a data contract.
    ///
    /// This function updates a given data contract in the storage. The fee for updating
    /// the contract is also calculated and returned.
    ///
    /// # Arguments
    ///
    /// * `contract` - A reference to the `DataContract` to be updated.
    /// * `block_info` - A `BlockInfo` object containing information about the block where
    ///   the contract is being updated.
    /// * `apply` - A boolean indicating whether the contract update should be applied (`true`) or not (`false`). Passing `false` would only tell the fees but won't interact with the state.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for updating the contract.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - If successful, returns a `FeeResult` representing the fee
    ///   for updating the contract. If an error occurs during the contract update or fee calculation,
    ///   returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the contract update or fee calculation fails.
    #[inline(always)]
    pub(super) fn update_contract_v3(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        if !apply {
            return self.insert_contract(
                contract,
                block_info,
                false,
                transaction,
                platform_version,
            );
        }

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let contract_bytes = contract.serialize_to_bytes_with_platform_version(platform_version)?;

        // Since we can update the contract by definition it already has storage flags
        let storage_flags = Some(StorageFlags::new_single_epoch(
            block_info.epoch.index,
            Some(contract.owner_id().to_buffer()),
        ));

        let contract_element = Element::Item(
            contract_bytes,
            StorageFlags::map_to_some_element_flags(storage_flags.as_ref()),
        );

        let original_contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contract.id().to_buffer(),
                Some(&block_info.epoch),
                true,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "contract should exist",
            )))?;

        if original_contract_fetch_info.contract.config().readonly() {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableContract(
                "original contract is readonly",
            )));
        }

        self.update_contract_element_v3(
            contract_element,
            contract,
            &original_contract_fetch_info.contract,
            &block_info,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        // Update DataContracts cache with the new contract
        let updated_contract_fetch_info = self
            .fetch_contract_and_add_operations(
                contract.id().to_buffer(),
                Some(&block_info.epoch),
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "contract should exist",
            )))?;

        self.cache
            .data_contracts
            .insert_rewritten(updated_contract_fetch_info, transaction.is_some());

        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            previous_fee_versions,
        )
    }

    /// Updates a contract.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn update_contract_element_v3(
        &self,
        contract_element: Element,
        contract: &DataContract,
        original_contract: &DataContract,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>;
        let batch_operations = self.update_contract_operations_v3(
            contract_element,
            contract,
            original_contract,
            block_info,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )
    }

    /// Updates a contract.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub(super) fn update_contract_add_operations_v3(
        &self,
        contract_element: Element,
        contract: &DataContract,
        original_contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations = self.update_contract_operations_v3(
            contract_element,
            contract,
            original_contract,
            block_info,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        drive_operations.extend(batch_operations);
        Ok(())
    }

    /// Collects generation 2 writes and the history trees of newly added document types.
    #[allow(clippy::too_many_arguments)]
    fn update_contract_operations_v3(
        &self,
        contract_element: Element,
        contract: &DataContract,
        original_contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let flags = contract_element.get_flags().clone();
        let storage_flags = StorageFlags::map_some_element_flags_ref(&flags)?;
        let mut operations = vec![];
        self.update_contract_add_operations_v2(
            contract_element,
            contract,
            original_contract,
            block_info,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut operations,
            platform_version,
        )?;
        for (name, document_type) in contract.document_types() {
            if !original_contract.document_types().contains_key(name)
                && document_type.documents_keep_history()
            {
                self.batch_insert_empty_tree(
                    contract_document_type_path(contract.id_ref().as_bytes(), name),
                    KeyRef(&[DOCUMENT_HISTORY_TREE_KEY]),
                    storage_flags.as_ref(),
                    &mut operations,
                    &platform_version.drive,
                )?;
            }
        }
        Ok(operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::contract::paths::DataContractPaths;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::platform_value::platform_value;
    use dpp::tests::fixtures::get_dashpay_contract_fixture;

    #[test]
    fn should_cover_applied_keep_history_contract_updates_with_the_fee_estimate() {
        let version = PlatformVersion::latest();
        for add_type in [false, true] {
            for via_apply_contract in [false, true] {
                let drive = setup_drive_with_initial_state_structure(Some(version));
                let mut contract = get_dashpay_contract_fixture(None, 0, version.protocol_version)
                    .data_contract_owned();
                let schema = |max_length: u64| {
                    platform_value!({
                        "type": "object",
                        "documentsKeepHistory": true,
                        "documentsMutable": true,
                        "canBeDeleted": false,
                        "properties": {
                            "message": { "type": "string", "position": 0, "maxLength": max_length }
                        },
                        "additionalProperties": false,
                    })
                };
                contract
                    .set_document_schema("historyNote", schema(100), true, &mut vec![], version)
                    .expect("add existing retained type");
                drive
                    .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
                    .expect("insert original");
                let original_bytes = contract
                    .serialize_to_bytes_with_platform_version(version)
                    .unwrap();
                contract
                    .set_document_schema("historyNote", schema(101), true, &mut vec![], version)
                    .expect("extend existing retained type");
                if add_type {
                    contract
                        .set_document_schema(
                            "otherHistoryNote",
                            schema(100),
                            true,
                            &mut vec![],
                            version,
                        )
                        .expect("add another retained type");
                }
                contract.increment_version();

                let fee_for = |apply, selected_version: &PlatformVersion| {
                    if via_apply_contract {
                        drive.apply_contract(
                            &contract,
                            BlockInfo::default(),
                            apply,
                            None,
                            None,
                            selected_version,
                        )
                    } else {
                        drive.update_contract(
                            &contract,
                            BlockInfo::default(),
                            apply,
                            None,
                            selected_version,
                            None,
                        )
                    }
                    .expect("calculate update fee")
                };
                let estimate = fee_for(false, version);
                let mut previous_insert_version = version.clone();
                previous_insert_version
                    .drive
                    .methods
                    .contract
                    .insert
                    .insert_contract = 2;
                let previous_estimate = fee_for(false, &previous_insert_version);
                let stored_bytes = || {
                    drive
                        .grove
                        .get_raw(
                            (&contract.root_path()).into(),
                            &[0],
                            None,
                            &version.drive.grove_version,
                        )
                        .value
                        .unwrap()
                        .into_item_bytes()
                        .unwrap()
                };
                assert_eq!(
                    stored_bytes(),
                    original_bytes,
                    "estimating must leave the original stored"
                );
                let applied = fee_for(true, version);
                assert!(estimate.total_base_fee() >= applied.total_base_fee(),
                    "add_type={add_type}, via_apply_contract={via_apply_contract}: estimate {estimate:?}, applied {applied:?}");
                assert_eq!(
                    stored_bytes(),
                    contract
                        .serialize_to_bytes_with_platform_version(version)
                        .unwrap(),
                    "the applied update must be persisted"
                );
                println!("add_type={add_type}, via_apply_contract={via_apply_contract}, previous={previous_estimate:?}, estimate={estimate:?}, applied={applied:?}");
            }
        }
    }

    #[test]
    fn should_create_history_tree_for_a_new_type_only_from_protocol_15() {
        use crate::drive::document::paths::{
            contract_document_type_path_vec, DOCUMENT_HISTORY_TREE_KEY,
        };

        for (protocol, expected) in [(14, false), (15, true)] {
            let version = PlatformVersion::get(protocol).expect("protocol version");
            let drive = setup_drive_with_initial_state_structure(Some(version));
            let mut contract = get_dashpay_contract_fixture(None, 0, version.protocol_version)
                .data_contract_owned();
            drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    version,
                )
                .expect("insert original contract");

            contract
                .set_document_schema(
                    "historyNote",
                    platform_value!({
                        "type": "object",
                        "documentsKeepHistory": true,
                        "documentsMutable": true,
                        "canBeDeleted": false,
                        "properties": {
                            "message": {
                                "type": "string",
                                "position": 0,
                                "maxLength": 100,
                            }
                        },
                        "additionalProperties": false,
                    }),
                    true,
                    &mut vec![],
                    version,
                )
                .expect("add keep-history document type");
            contract.increment_version();
            drive
                .update_contract(&contract, BlockInfo::default(), true, None, version, None)
                .expect("update contract with keep-history document type");

            let type_path =
                contract_document_type_path_vec(contract.id().as_slice(), "historyNote");
            let history_tree = drive
                .grove
                .get_raw(
                    type_path.as_slice().into(),
                    &[DOCUMENT_HISTORY_TREE_KEY],
                    None,
                    &version.drive.grove_version,
                )
                .value;
            assert_eq!(
                history_tree.is_ok(),
                expected,
                "protocol {protocol}: {history_tree:?}"
            );
        }
    }
}
