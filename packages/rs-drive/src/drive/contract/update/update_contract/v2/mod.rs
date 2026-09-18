// Protocol 15 generation: keep-history document types a contract update adds
// get their per-type history tree; everything else matches v1.
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::serialization::PlatformSerializableWithPlatformVersion;

use crate::drive::contract_documents_path;
use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::drive::document::ranked_index_tree_type::property_name_tree_type_and_ranked_axes_for_level;
use crate::error::contract::DataContractError;
use crate::util::grove_operations::BatchInsertTreeApplyType;
use crate::util::object_size_info::DriveKeyInfo::KeyRef;
use crate::util::object_size_info::PathKeyInfo::PathFixedSizeKeyRef;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
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
    pub(super) fn update_contract_v2(
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

        self.update_contract_element_v2(
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
    pub(super) fn update_contract_element_v2(
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
        let batch_operations = self.update_contract_operations_v2(
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
    pub(super) fn update_contract_add_operations_v2(
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
        let batch_operations = self.update_contract_operations_v2(
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
    fn update_contract_operations_v2(
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
        let mut batch_operations: Vec<LowLevelDriveOperation> = self
            .update_contract_base_operations_v2(
                contract_element,
                contract,
                original_contract,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?;

        for (token_pos, configuration) in contract.tokens() {
            let token_id = contract.token_id(*token_pos).ok_or(Error::DataContract(
                DataContractError::CorruptedDataContract(format!(
                    "data contract has a token at position {}, but it can not be found",
                    token_pos
                )),
            ))?;

            batch_operations.extend(self.create_token_trees_operations(
                contract.id(),
                *token_pos,
                token_id.to_buffer(),
                configuration.start_as_paused(),
                true,
                &mut None,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        if !contract.groups().is_empty() {
            batch_operations.extend(self.add_new_groups_operations(
                contract.id(),
                contract.groups(),
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        // Skipping an empty keyword set is load-bearing, but it is a shield
        // rather than a fix, and both halves matter to anyone changing it.
        //
        // What it prevents: the keyword update emits its deletes blind to each
        // other in one batch, so several of them jointly emptying the shared
        // `byContractId/<contractId>` group would leave that group tree behind
        // with nothing in it — and emptying the group without refilling it
        // requires exactly this empty-set case.
        //
        // What it costs: the previous keyword documents are not deleted either,
        // so a contract that clears its keywords advertises none while keyword
        // search still returns it under the old ones. Removing this guard to fix
        // that trades a stale index for a stranded group tree; the deletes have
        // to become sibling-aware first. Both halves are pinned —
        // `clearing_a_contracts_keywords_leaves_the_old_ones_indexed` and
        // `clearing_every_keyword_leaves_an_empty_by_contract_id_group_behind`.
        if !contract.keywords().is_empty() {
            batch_operations.extend(self.update_contract_keywords_operations(
                contract.id(),
                contract.owner_id(),
                contract.keywords(),
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        if let Some(description) = contract.description() {
            batch_operations.extend(self.update_contract_description_operations(
                contract.id(),
                contract.owner_id(),
                description,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        Ok(batch_operations)
    }

    #[allow(clippy::too_many_arguments)]
    fn update_contract_base_operations_v2(
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

                let document_type_ref = document_type.as_ref();
                let index_structure = document_type_ref.index_structure();
                // For each type we should insert the indices that are top
                // level — one root sub-level per distinct top tree (plain
                // first properties by name, time-range grids by their
                // qualified `TimeRangeTransform::storage_key`), the same
                // iteration `insert_contract_v0` performs.
                //
                // `batch_insert_empty_tree_if_not_exists` is a no-op when the
                // index already exists, so this loop covers BOTH the
                // pre-existing indexes (no-op, no on-disk change) AND any
                // brand-new top-level indexes the contract update adds to an
                // existing doctype. The latter must materialize with the
                // matching tree variant from the `(range_countable,
                // range_summable)` dispatch — same 4-way table the
                // new-doctype branch below uses, identical to
                // `insert_contract_v0`'s top-level-index dispatch. Without
                // this, adding a new `rangeSummable: true` (or
                // `rangeCountable: true`) index to an existing doctype via
                // contract update silently created a NormalTree, diverging
                // from the layout a fresh insert would have produced and
                // breaking subsequent range-sum / range-count reads.
                for (level_key, level) in index_structure.sub_levels() {
                    {
                        // Meta schema v3 (PV14) additionally upgrades the
                        // chosen variant to its indexed mirror when the index
                        // declares a ranking axis — including the grouping
                        // level of a compound index ranked at its first
                        // property (`rankedCountable: { at }`), which the
                        // level-aware resolver maps to the Count-axis indexed
                        // tree; `ranked_axes` is empty for every pre-v3
                        // contract, making this arm bit-identical to the
                        // previous 4-way dispatch for them.
                        let (target_tree_type, ranked_axes) =
                            property_name_tree_type_and_ranked_axes_for_level(level)?;
                        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                            BatchInsertTreeApplyType::StatefulBatchInsertTree
                        } else {
                            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                                in_tree_type: TreeType::NormalTree,
                                tree_type: target_tree_type,
                                flags_len: element_flags
                                    .as_ref()
                                    .map(|e| e.len() as u32)
                                    .unwrap_or_default(),
                            }
                        };
                        // The generic `batch_insert_empty_index_tree_if_not_exists`
                        // already takes a `TreeType` (plus the ranking axes an
                        // indexed element needs and a `TreeType` cannot carry)
                        // and routes the grovedb insert to the matching
                        // variant — same helper count's non-summable index
                        // path uses. No-op when the path/key already exists,
                        // which is how this branch handles both pre-existing
                        // indexes (unchanged on disk) and brand-new ones
                        // (materialized with the dispatch-chosen variant).
                        self.batch_insert_empty_index_tree_if_not_exists(
                            PathFixedSizeKeyRef((type_path, level_key.as_bytes())),
                            target_tree_type,
                            &ranked_axes,
                            storage_flags.as_ref().map(|flags| flags.as_ref()),
                            apply_type,
                            transaction,
                            &mut None,
                            &mut batch_operations,
                            drive_version,
                        )?;
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

                if document_type.as_ref().documents_keep_history() {
                    self.batch_insert_empty_tree(
                        type_path,
                        KeyRef(&[crate::drive::document::paths::DOCUMENT_HISTORY_TREE_KEY]),
                        storage_flags.as_ref().map(|flags| flags.as_ref()),
                        &mut batch_operations,
                        drive_version,
                    )?;
                }

                // primary key tree — route through the centralized
                // primary_key_tree_type() so contract update, document inserts,
                // deletes, and estimation paths all see the same tree-variant
                // selection (under whichever drive method version is active).
                // Must stay in lock-step with the matching dispatch in
                // `insert_contract_v0::insert_contract_operations_v0`: a fresh
                // insert and a contract-update that adds the same doctype must
                // materialize the same on-disk tree variant, otherwise later
                // sum/range-sum reads + fee logic operate against the wrong
                // tree type for updated contracts.
                let key_info = KeyRef(&[0]);
                match document_type
                    .as_ref()
                    .primary_key_tree_type(platform_version)?
                {
                    TreeType::ProvableCountTree => self.batch_insert_empty_provable_count_tree(
                        type_path,
                        key_info,
                        storage_flags.as_ref().map(|flags| flags.as_ref()),
                        &mut batch_operations,
                        drive_version,
                    )?,
                    TreeType::CountTree => self.batch_insert_empty_count_tree(
                        type_path,
                        key_info,
                        storage_flags.as_ref().map(|flags| flags.as_ref()),
                        &mut batch_operations,
                        drive_version,
                    )?,
                    TreeType::SumTree => self.batch_insert_empty_sum_tree(
                        type_path,
                        key_info,
                        storage_flags.as_ref().map(|flags| flags.as_ref()),
                        &mut batch_operations,
                        drive_version,
                    )?,
                    TreeType::ProvableSumTree => self.batch_insert_empty_provable_sum_tree(
                        type_path,
                        key_info,
                        storage_flags.as_ref().map(|flags| flags.as_ref()),
                        &mut batch_operations,
                        drive_version,
                    )?,
                    TreeType::CountSumTree => self.batch_insert_empty_count_sum_tree(
                        type_path,
                        key_info,
                        storage_flags.as_ref().map(|flags| flags.as_ref()),
                        &mut batch_operations,
                        drive_version,
                    )?,
                    TreeType::ProvableCountSumTree => self
                        .batch_insert_empty_provable_count_sum_tree(
                            type_path,
                            key_info,
                            storage_flags.as_ref().map(|flags| flags.as_ref()),
                            &mut batch_operations,
                            drive_version,
                        )?,
                    TreeType::ProvableCountProvableSumTree => self
                        .batch_insert_empty_provable_count_provable_sum_tree(
                            type_path,
                            key_info,
                            storage_flags.as_ref().map(|flags| flags.as_ref()),
                            &mut batch_operations,
                            drive_version,
                        )?,
                    _ => self.batch_insert_empty_tree(
                        type_path,
                        key_info,
                        storage_flags.as_ref().map(|flags| flags.as_ref()),
                        &mut batch_operations,
                        drive_version,
                    )?,
                }

                let document_type_ref = document_type.as_ref();
                let index_structure = document_type_ref.index_structure();
                // For each type we should insert the indices that are top
                // level — the index structure's root sub-levels, whose keys
                // are grid-qualified for time-range first properties (see
                // `insert_contract_v0`).
                for (level_key, level) in index_structure.sub_levels() {
                    let index_bytes = level_key.as_bytes();
                    {
                        // Top-level index tree variant is selected from the
                        // index's `(range_countable, range_summable)` pair —
                        // identical 4-way dispatch as
                        // `insert_contract_operations_v0`. Without this dispatch
                        // the previous unconditional `batch_insert_empty_tree`
                        // would materialize a plain `NormalTree` for any new
                        // sum- or range-countable top-level index added via
                        // contract update, diverging on-disk layout from
                        // fresh-insert contracts.
                        let (tree_type, ranked_axes) =
                            property_name_tree_type_and_ranked_axes_for_level(level)?;
                        match tree_type {
                            TreeType::ProvableCountProvableSumTree => self
                                .batch_insert_empty_provable_count_provable_sum_tree(
                                    type_path,
                                    KeyRef(index_bytes),
                                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                                    &mut batch_operations,
                                    drive_version,
                                )?,
                            TreeType::ProvableCountTree => self
                                .batch_insert_empty_provable_count_tree(
                                    type_path,
                                    KeyRef(index_bytes),
                                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                                    &mut batch_operations,
                                    drive_version,
                                )?,
                            TreeType::ProvableSumTree => self
                                .batch_insert_empty_provable_sum_tree(
                                    type_path,
                                    KeyRef(index_bytes),
                                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                                    &mut batch_operations,
                                    drive_version,
                                )?,
                            // Ranked (indexed) variants — meta schema v3 / PV14.
                            TreeType::ProvableCountIndexedTree => self
                                .batch_insert_empty_provable_count_indexed_tree(
                                    type_path,
                                    KeyRef(index_bytes),
                                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                                    &mut batch_operations,
                                    drive_version,
                                )?,
                            TreeType::ProvableSumIndexedTree => self
                                .batch_insert_empty_provable_sum_indexed_tree(
                                    type_path,
                                    KeyRef(index_bytes),
                                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                                    &mut batch_operations,
                                    drive_version,
                                )?,
                            TreeType::ProvableCountProvableSumIndexedTree => self
                                .batch_insert_empty_provable_count_provable_sum_indexed_tree(
                                    type_path,
                                    KeyRef(index_bytes),
                                    &ranked_axes,
                                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                                    &mut batch_operations,
                                    drive_version,
                                )?,
                            _ => self.batch_insert_empty_tree(
                                type_path,
                                KeyRef(index_bytes),
                                storage_flags.as_ref().map(|flags| flags.as_ref()),
                                &mut batch_operations,
                                drive_version,
                            )?,
                        }
                    }
                }
            }
        }
        Ok(batch_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::accessors::v1::DataContractV1Setters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::platform_value::platform_value;
    use dpp::prelude::Identifier;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    /// Exercises `update_contract_operations_v2` when the updated contract
    /// gains tokens that weren't in the original. This covers the loop that
    /// calls `create_token_trees_operations` for each token.
    #[test]
    fn test_update_contract_v1_adds_tokens_creates_token_trees() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Original: no tokens.
        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.config_mut().set_readonly(false);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("insert initial contract without tokens");

        // Updated: add a token configuration. The update path exercises the
        // `create_token_trees_operations` call in update_contract_operations_v2.
        let token_config = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
        );
        contract.set_tokens(BTreeMap::from([(0, token_config)]));
        contract.increment_version();

        drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("update adding tokens should succeed");
    }

    /// Exercises `update_contract_operations_v2` where the updated contract
    /// gains groups that weren't in the original. This covers the
    /// `if !contract.groups().is_empty()` true branch inside
    /// `update_contract_operations_v2`, invoking `add_new_groups_operations`.
    #[test]
    fn test_update_contract_v1_adds_groups() {
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
            .expect("insert");

        // Add a group.
        let member = Identifier::random();
        let group = Group::V0(GroupV0 {
            members: BTreeMap::from([(member, 1)]),
            required_power: 1,
        });
        contract.set_groups(BTreeMap::from([(0, group)]));
        contract.increment_version();

        drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("update adding groups should succeed");
    }

    /// Exercises `update_contract_operations_v2`'s keyword-update branch:
    /// update a contract that starts with some keywords to a new set of
    /// keywords (different set), routed through the full `update_contract_v2`
    /// path rather than the dedicated `update_contract_keywords` API.
    #[test]
    fn test_update_contract_v1_keyword_delta_via_update_contract() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Insert the keyword_search system contract first (required because
        // update_contract_v2 calls update_contract_keywords_operations).
        let keyword_search =
            load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)
                .expect("load keyword_search");
        drive
            .apply_contract(
                &keyword_search,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("apply keyword_search");

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_keywords(vec!["initial_a".to_string(), "initial_b".to_string()]);

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("initial insert with keywords");

        // Now change keywords entirely.
        contract.set_keywords(vec!["new_x".to_string(), "new_y".to_string()]);
        contract.increment_version();

        drive
            .update_contract(
                &contract,
                BlockInfo {
                    time_ms: 2000,
                    height: 10,
                    core_height: 5,
                    epoch: Default::default(),
                },
                true,
                None,
                platform_version,
                None,
            )
            .expect("update keyword delta via update_contract should succeed");
    }

    /// The keywords the keyword search index currently returns for `contract_id`.
    fn indexed_keywords(
        drive: &crate::drive::Drive,
        keyword_search: &dpp::prelude::DataContract,
        contract_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Vec<String> {
        use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
        use crate::query::{DriveDocumentQuery, WhereClause, WhereOperator};
        use dpp::document::DocumentV0Getters;
        use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
        use dpp::platform_value::Value;

        let document_type = keyword_search
            .document_type_for_name("contractKeywords")
            .expect("contractKeywords doctype");
        let mut query = DriveDocumentQuery::all_items_query(keyword_search, document_type, None);
        query.internal_clauses.equal_clauses.insert(
            "contractId".to_string(),
            WhereClause {
                field: "contractId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(contract_id.to_buffer()),
            },
        );
        let mut keywords: Vec<String> = drive
            .query_documents(
                query,
                None,
                false,
                None,
                Some(platform_version.protocol_version),
            )
            .expect("the byContractId query must succeed")
            .documents_owned()
            .into_iter()
            .map(|document| {
                document
                    .properties()
                    .get_string("keyword")
                    .expect("every keyword document carries a keyword")
            })
            .collect();
        keywords.sort();
        keywords
    }

    /// **This test asserts a defect, not the desired behaviour**, and it is the
    /// other half of the empty-keyword-set skip above.
    ///
    /// Clearing a contract's keywords does not delete its keyword documents: an
    /// empty set skips the keyword update entirely, so the previous documents
    /// survive and stay indexed. The contract then advertises no keywords while
    /// keyword search still returns it under the old ones, permanently.
    ///
    /// The skip is a shield, not a fix. It is what keeps the deletes from
    /// jointly emptying the shared `byContractId` group and stranding it — see
    /// `clearing_every_keyword_leaves_an_empty_by_contract_id_group_behind` —
    /// so removing it to make this test go green trades a stale index for an
    /// empty group tree. Making the deletes sibling-aware has to come first.
    #[test]
    fn clearing_a_contracts_keywords_leaves_the_old_ones_indexed() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let keyword_search =
            load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)
                .expect("load keyword_search");
        drive
            .apply_contract(
                &keyword_search,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("apply keyword_search");

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_keywords(vec!["alpha".to_string(), "bravo".to_string()]);
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("initial insert with keywords");

        assert_eq!(
            indexed_keywords(&drive, &keyword_search, contract.id(), platform_version),
            vec!["alpha".to_string(), "bravo".to_string()],
            "baseline: both keywords are indexed"
        );

        contract.set_keywords(vec![]);
        contract.increment_version();
        drive
            .update_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("clearing keywords via update_contract should succeed");

        assert_eq!(
            indexed_keywords(&drive, &keyword_search, contract.id(), platform_version),
            vec!["alpha".to_string(), "bravo".to_string()],
            "the old keyword documents are expected to survive: an empty keyword set skips \
             the keyword update rather than performing it"
        );
    }

    /// Exercises `update_contract_operations_v2`'s description-update branch:
    /// changing contract description routes through
    /// `update_contract_description_operations`. Covers the `if let Some(description)`
    /// true branch specifically from the v1 update path (not the dedicated update
    /// description API).
    #[test]
    fn test_update_contract_v1_description_via_update_contract() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let keyword_search =
            load_system_data_contract(SystemDataContract::KeywordSearch, platform_version)
                .expect("load keyword_search");
        drive
            .apply_contract(
                &keyword_search,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("apply keyword_search");

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        contract.set_description(Some("initial description".to_string()));

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("initial insert with description");

        contract.set_description(Some("updated description text".to_string()));
        contract.increment_version();

        drive
            .update_contract(
                &contract,
                BlockInfo {
                    time_ms: 3000,
                    height: 20,
                    core_height: 7,
                    epoch: Default::default(),
                },
                true,
                None,
                platform_version,
                None,
            )
            .expect("update description via update_contract should succeed");
    }

    /// A keep-history type introduced by a contract update uses the legacy
    /// layout at protocol 14 and the per-type history tree from protocol 15.
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
