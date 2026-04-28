use crate::drive::{contract_documents_path, Drive};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::DriveKeyInfo::KeyRef;
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKeyRef;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::serialization::PlatformSerializableWithPlatformVersion;

use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::{HashMap, HashSet};

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
    pub(super) fn update_contract_v0(
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

        self.update_contract_element_v0(
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
            .insert(updated_contract_fetch_info, transaction.is_some());

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
    pub(super) fn update_contract_element_v0(
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
        let batch_operations = self.update_contract_operations_v0(
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
    pub(super) fn update_contract_add_operations_v0(
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
        let batch_operations = self.update_contract_operations_v0(
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

    /// operations for updating a contract.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::drive::contract::update::update_contract) fn update_contract_operations_v0(
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
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        let drive_version = &platform_version.drive;

        if original_contract.config().readonly() {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableContract(
                "contract is readonly",
            )));
        }

        if contract.config().readonly() {
            return Err(Error::Drive(DriveError::ChangingContractToReadOnly(
                "contract can not be changed to readonly",
            )));
        }

        if contract.config().keeps_history() ^ original_contract.config().keeps_history() {
            return Err(Error::Drive(DriveError::ChangingContractKeepsHistory(
                "contract can not change whether it keeps history",
            )));
        }

        if contract.config().documents_keep_history_contract_default()
            ^ original_contract
                .config()
                .documents_keep_history_contract_default()
        {
            return Err(Error::Drive(
                DriveError::ChangingContractDocumentsKeepsHistoryDefault(
                    "contract can not change the default of whether documents keeps history",
                ),
            ));
        }

        if contract.config().documents_mutable_contract_default()
            ^ original_contract
                .config()
                .documents_mutable_contract_default()
        {
            return Err(Error::Drive(
                DriveError::ChangingContractDocumentsMutabilityDefault(
                    "contract can not change the default of whether documents are mutable",
                ),
            ));
        }

        let element_flags = contract_element.get_flags().clone();

        // this will override the previous contract if we do not keep history
        self.add_contract_to_storage(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            &mut batch_operations,
            false,
            transaction,
            drive_version,
        )?;

        let storage_flags = StorageFlags::map_cow_some_element_flags_ref(&element_flags)?;

        let contract_documents_path = contract_documents_path(contract.id_ref().as_bytes());
        for (type_key, document_type) in contract.document_types().iter() {
            let original_document_type = &original_contract.document_types().get(type_key);
            if let Some(original_document_type) = original_document_type {
                if original_document_type.documents_mutable() ^ document_type.documents_mutable() {
                    return Err(Error::Drive(DriveError::ChangingDocumentTypeMutability(
                        "contract can not change whether a specific document type is mutable",
                    )));
                }
                if original_document_type.documents_keep_history()
                    ^ document_type.documents_keep_history()
                {
                    return Err(Error::Drive(DriveError::ChangingDocumentTypeKeepsHistory(
                        "contract can not change whether a specific document type keeps history",
                    )));
                }

                let type_path = [
                    contract_documents_path[0],
                    contract_documents_path[1],
                    contract_documents_path[2],
                    type_key.as_bytes(),
                ];

                let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                    BatchInsertTreeApplyType::StatefulBatchInsertTree
                } else {
                    BatchInsertTreeApplyType::StatelessBatchInsertTree {
                        in_tree_type: TreeType::NormalTree,
                        tree_type: TreeType::NormalTree,
                        flags_len: element_flags
                            .as_ref()
                            .map(|e| e.len() as u32)
                            .unwrap_or_default(),
                    }
                };

                let mut index_cache: HashSet<&[u8]> = HashSet::new();
                // for each type we should insert the indices that are top level
                for index in document_type.as_ref().top_level_indices() {
                    // toDo: we can save a little by only inserting on new indexes
                    let index_bytes = index.name.as_bytes();
                    if !index_cache.contains(index_bytes) {
                        self.batch_insert_empty_tree_if_not_exists(
                            PathFixedSizeKeyRef((type_path, index.name.as_bytes())),
                            TreeType::NormalTree,
                            storage_flags.as_ref().map(|flags| flags.as_ref()),
                            apply_type,
                            transaction,
                            &mut None,
                            &mut batch_operations,
                            drive_version,
                        )?;
                        index_cache.insert(index_bytes);
                    }
                }
            } else {
                // We can just insert this directly because the original document type already exists
                self.batch_insert_empty_tree(
                    contract_documents_path,
                    KeyRef(type_key.as_bytes()),
                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                    &mut batch_operations,
                    drive_version,
                )?;

                let type_path = [
                    contract_documents_path[0],
                    contract_documents_path[1],
                    contract_documents_path[2],
                    type_key.as_bytes(),
                ];

                // primary key tree
                self.batch_insert_empty_tree(
                    type_path,
                    KeyRef(&[0]),
                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                    &mut batch_operations,
                    drive_version,
                )?;

                let mut index_cache: HashSet<&[u8]> = HashSet::new();
                // for each type we should insert the indices that are top level
                for index in document_type.as_ref().top_level_indices() {
                    // toDo: change this to be a reference by index
                    let index_bytes = index.name.as_bytes();
                    if !index_cache.contains(index_bytes) {
                        self.batch_insert_empty_tree(
                            type_path,
                            KeyRef(index.name.as_bytes()),
                            storage_flags.as_ref().map(|flags| flags.as_ref()),
                            &mut batch_operations,
                            drive_version,
                        )?;
                        index_cache.insert(index_bytes);
                    }
                }
            }
        }
        Ok(batch_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::platform_value::platform_value;
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::version::PlatformVersion;

    /// Exercises the `if original_contract.config().readonly() { ... }` branch
    /// inside `update_contract_operations_v0`. Note that the earlier readonly
    /// check in `update_contract_v0`/`v1` (line 97-100) triggers on the
    /// ORIGINAL fetched contract's readonly flag. PR #3516's
    /// `test_update_contract_errors_on_changing_to_readonly` only covers the
    /// "changing TO readonly" branch on a mutable original.
    ///
    /// This test covers a different branch: inserting a readonly contract
    /// first, then attempting to update it. The `update_contract_v0`/`v1`
    /// short-circuit at line 97 returns `UpdatingReadOnlyImmutableContract`.
    /// This guards the in-storage readonly flag check path that differs from
    /// `ChangingContractToReadOnly`.
    #[test]
    fn test_update_contract_v0_readonly_original_errors() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Insert a readonly contract first.
        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.config_mut().set_readonly(true);
        contract.config_mut().set_can_be_deleted(true); // keep default flags

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("insert readonly contract");

        // Attempt to update it — since it's readonly in storage, this should fail.
        contract.increment_version();
        let result = drive.update_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        );

        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableContract(
                    _
                )))
            ),
            "Expected UpdatingReadOnlyImmutableContract, got: {:?}",
            result
        );
    }

    /// Exercises the `else` branch in `update_contract_operations_v0` where
    /// the update introduces a NEW document type not present in the original
    /// contract. That branch (lines ~334-376) performs:
    /// - batch_insert_empty_tree(contract_documents_path, key=new_type_name)
    /// - batch_insert_empty_tree(type_path, primary_key_tree[0])
    /// - for each top-level index: batch_insert_empty_tree(type_path, index_name)
    ///
    /// PR #3516 test_update_contract_errors_on_changing_document_type_* tests
    /// cover mutations of EXISTING document types only. No existing test
    /// exercises adding an entirely new document type.
    #[test]
    fn test_update_contract_v0_adds_new_document_type_creates_trees() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("initial insert");

        let original_type_count = contract.document_types().len();

        // Add a brand-new document type that did NOT exist before.
        let new_schema = platform_value!({
            "type": "object",
            "properties": {
                "label": {
                    "type": "string",
                    "position": 0,
                    "maxLength": 50,
                }
            },
            "additionalProperties": false,
        });
        contract
            .set_document_schema(
                "brandNewDocType",
                new_schema,
                true,
                &mut vec![],
                platform_version,
            )
            .expect("set new schema");
        contract.increment_version();

        // The update path will hit the `else` branch because "brandNewDocType"
        // is not in the original's document_types map.
        drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("update with new doc type should succeed");

        // Verify the new type is now present.
        let fetched = drive
            .get_contract_with_fetch_info(contract.id().to_buffer(), true, None, platform_version)
            .expect("fetch")
            .expect("contract exists");
        assert_eq!(
            fetched.contract.document_types().len(),
            original_type_count + 1,
            "updated contract should have one additional document type"
        );
        assert!(
            fetched
                .contract
                .document_types()
                .contains_key("brandNewDocType"),
            "new document type must be present"
        );
    }

    /// Exercises the `update_contract_v0/v1` apply=false path where the
    /// contract ALREADY EXISTS in storage (unlike PR #3516's
    /// `test_update_contract_apply_false_delegates_to_insert_on_missing_contract`
    /// which uses a non-existent contract). When apply=false and the contract
    /// exists, `update_contract_v0/v1` short-circuits to `insert_contract(apply=false)`
    /// BEFORE the existing-contract fetch, so the estimation goes through
    /// `insert_contract` rather than `update_contract_operations_v0`.
    ///
    /// This is a distinct test because the behavior is: update(apply=false)
    /// on an existing contract returns insert-estimate semantics, not
    /// update-estimate semantics. That's an important behavioral pin.
    #[test]
    fn test_update_contract_v0_apply_false_on_existing_contract_delegates_to_insert() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("insert original");

        // update with apply=false should delegate to insert_contract(false),
        // regardless of whether the contract already exists.
        let fee = drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                false,
                None,
                platform_version,
                None,
            )
            .expect("apply=false on existing should succeed via insert delegation");

        assert!(fee.processing_fee > 0 || fee.storage_fee > 0);

        // The original contract state should remain unchanged and fetchable.
        let fetched = drive
            .get_contract_with_fetch_info(contract.id().to_buffer(), false, None, platform_version)
            .expect("fetch")
            .expect("contract should still exist");
        assert_eq!(fetched.contract.id(), contract.id());
    }
}
