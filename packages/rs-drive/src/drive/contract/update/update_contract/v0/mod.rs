use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::drive::document::ranked_index_tree_type::property_name_tree_type_and_ranked_axes_for_level;
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

use dpp::serialization::PlatformSerializableWithPlatformVersion;

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
    use crate::drive::{Drive, RootTree};
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::grove_operations::DirectQueryType;
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::platform_value::{platform_value, Value};
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    fn label_document_schema(documents_countable: bool, range_countable: bool) -> Value {
        let mut schema = platform_value!({
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

        let schema_map = schema.as_map_mut().expect("schema should be a map");
        if documents_countable {
            schema_map.push((
                Value::Text("documentsCountable".to_string()),
                Value::Bool(true),
            ));
        }
        if range_countable {
            schema_map.push((Value::Text("rangeCountable".to_string()), Value::Bool(true)));
        }

        schema
    }

    /// Sum-bearing doctype: a single integer property `score` listed in
    /// `required`, exposed via `documentsSummable`. Adding `rangeSummable`
    /// also turns it into a range-sum doctype, which under
    /// `primary_key_tree_type()` resolves to `ProvableSumTree`.
    fn score_document_schema(documents_summable: bool, range_summable: bool) -> Value {
        let mut schema = platform_value!({
            "type": "object",
            "properties": {
                "score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100,
                    "position": 0,
                },
            },
            "required": ["score"],
            "additionalProperties": false,
        });
        let schema_map = schema.as_map_mut().expect("schema should be a map");
        if documents_summable {
            schema_map.push((
                Value::Text("documentsSummable".to_string()),
                Value::Text("score".to_string()),
            ));
        }
        if range_summable {
            schema_map.push((Value::Text("rangeSummable".to_string()), Value::Bool(true)));
        }
        schema
    }

    /// Sum-bearing doctype via the `documentsAverageable` shorthand —
    /// desugars to `documentsCountable: true + documentsSummable: "score"`,
    /// so `primary_key_tree_type()` resolves to `CountSumTree`. Adding
    /// `rangeAverageable: true` promotes to
    /// `ProvableCountProvableSumTree` (range-count + range-sum carrier).
    fn averageable_document_schema(range_averageable: bool) -> Value {
        let mut schema = platform_value!({
            "type": "object",
            "properties": {
                "score": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100,
                    "position": 0,
                },
            },
            "required": ["score"],
            "additionalProperties": false,
            "documentsAverageable": "score",
        });
        if range_averageable {
            let schema_map = schema.as_map_mut().expect("schema should be a map");
            schema_map.push((
                Value::Text("rangeAverageable".to_string()),
                Value::Bool(true),
            ));
        }
        schema
    }

    /// Document type carrying a single top-level `indices` entry whose
    /// `summable`/`rangeSummable`/`countable`/`rangeCountable` knobs are
    /// configurable. The integer property `amount` is summed; the
    /// indexed property `userId` is what we walk through to read the
    /// per-user tree under `[..doctype, "byUser"]`.
    fn schema_with_indexed_summable(
        index_summable: bool,
        index_range_summable: bool,
        index_countable: bool,
        index_range_countable: bool,
    ) -> Value {
        let mut index_map: Vec<(Value, Value)> = vec![
            (
                Value::Text("name".to_string()),
                Value::Text("byUser".to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![Value::Map(vec![(
                    Value::Text("userId".to_string()),
                    Value::Text("asc".to_string()),
                )])]),
            ),
        ];
        if index_summable {
            index_map.push((
                Value::Text("summable".to_string()),
                Value::Text("amount".to_string()),
            ));
        }
        if index_range_summable {
            index_map.push((Value::Text("rangeSummable".to_string()), Value::Bool(true)));
        }
        if index_countable {
            index_map.push((
                Value::Text("countable".to_string()),
                Value::Text("countable".to_string()),
            ));
        }
        if index_range_countable {
            index_map.push((Value::Text("rangeCountable".to_string()), Value::Bool(true)));
        }
        let mut schema = platform_value!({
            "type": "object",
            "properties": {
                "userId": {
                    "type": "array",
                    "byteArray": true,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "minItems": 32,
                    "maxItems": 32,
                    "position": 0,
                },
                "amount": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 1000000,
                    "position": 1,
                },
            },
            "required": ["userId", "amount"],
            "additionalProperties": false,
        });
        let schema_map = schema.as_map_mut().expect("schema should be a map");
        schema_map.push((
            Value::Text("indices".to_string()),
            Value::Array(vec![Value::Map(index_map)]),
        ));
        schema
    }

    /// Read a top-level index tree element from
    /// `[..doctype, "<index_name>"]`.
    fn read_top_level_index_tree(
        drive: &Drive,
        contract: &dpp::prelude::DataContract,
        document_type_name: &str,
        index_name: &str,
    ) -> Element {
        let platform_version = PlatformVersion::latest();
        let contract_id = contract.id();
        let path: [&[u8]; 4] = [
            &[RootTree::DataContractDocuments as u8],
            contract_id.as_bytes(),
            &[1],
            document_type_name.as_bytes(),
        ];

        drive
            .grove_get_raw(
                (&path).into(),
                index_name.as_bytes(),
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected grove_get_raw to succeed")
            .expect("top-level index tree element should exist")
    }

    fn update_contract_with_new_document_type(
        document_type_name: &str,
        new_schema: Value,
    ) -> (Drive, dpp::prelude::DataContract, usize) {
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

        contract
            .set_document_schema(
                document_type_name,
                new_schema,
                true,
                &mut vec![],
                platform_version,
            )
            .expect("set new schema");
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
            .expect("update with new doc type should succeed");

        (drive, contract, original_type_count)
    }

    fn read_primary_key_tree(
        drive: &Drive,
        contract: &dpp::prelude::DataContract,
        document_type_name: &str,
    ) -> Element {
        let platform_version = PlatformVersion::latest();
        let contract_id = contract.id();
        let path: [&[u8]; 4] = [
            &[RootTree::DataContractDocuments as u8],
            contract_id.as_bytes(),
            &[1],
            document_type_name.as_bytes(),
        ];

        drive
            .grove_get_raw(
                (&path).into(),
                &[0],
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected grove_get_raw to succeed")
            .expect("primary key tree element should exist")
    }

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
        let platform_version = PlatformVersion::latest();
        let (drive, contract, original_type_count) = update_contract_with_new_document_type(
            "brandNewDocType",
            label_document_schema(false, false),
        );

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

        let elem = read_primary_key_tree(&drive, &contract, "brandNewDocType");
        assert!(
            matches!(elem, Element::Tree(..)),
            "new non-countable document type should use a NormalTree primary key tree, got {:?}",
            elem
        );
    }

    #[test]
    fn test_update_contract_v0_adds_new_documents_countable_type_creates_count_tree() {
        let (drive, contract, _) = update_contract_with_new_document_type(
            "brandNewCountedDocType",
            label_document_schema(true, false),
        );

        let elem = read_primary_key_tree(&drive, &contract, "brandNewCountedDocType");
        match elem {
            Element::CountTree(_, count, _) => {
                assert_eq!(count, 0, "freshly created CountTree should have count 0");
            }
            other => panic!(
                "new documentsCountable document type should use a CountTree primary key tree, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_update_contract_v0_adds_new_range_countable_type_creates_provable_count_tree() {
        let (drive, contract, _) = update_contract_with_new_document_type(
            "brandNewRangeCountedDocType",
            label_document_schema(false, true),
        );

        let elem = read_primary_key_tree(&drive, &contract, "brandNewRangeCountedDocType");
        match elem {
            Element::ProvableCountTree(_, count, _) => {
                assert_eq!(
                    count, 0,
                    "freshly created ProvableCountTree should have count 0"
                );
            }
            other => panic!(
                "new rangeCountable document type should use a ProvableCountTree primary key tree, got {:?}",
                other
            ),
        }
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

    /// `documentsSummable: "score"` on a freshly-added doctype must
    /// materialize a `SumTree` at the primary-key tree position.
    /// Regression for the pre-fix `_` catch-all in
    /// `update_contract_operations_v0` that silently fell through to
    /// `NormalTree`, diverging from `insert_contract_v0`'s dispatch.
    #[test]
    fn test_update_contract_v0_adds_new_documents_summable_type_creates_sum_tree() {
        let (drive, contract, _) = update_contract_with_new_document_type(
            "brandNewSummableDocType",
            score_document_schema(true, false),
        );

        let elem = read_primary_key_tree(&drive, &contract, "brandNewSummableDocType");
        match elem {
            Element::SumTree(_, sum, _) => {
                assert_eq!(sum, 0, "freshly created SumTree should have sum 0");
            }
            other => panic!(
                "new documentsSummable doctype must materialize a SumTree primary-key tree, got {:?}",
                other
            ),
        }
    }

    /// `documentsSummable + rangeSummable` resolves to
    /// `ProvableSumTree` at the doctype level — the variant
    /// `verify_range_sum_*` walks. Regression for the same `_`
    /// catch-all that would have produced `NormalTree`.
    #[test]
    fn test_update_contract_v0_adds_new_range_summable_type_creates_provable_sum_tree() {
        let (drive, contract, _) = update_contract_with_new_document_type(
            "brandNewRangeSummableDocType",
            score_document_schema(true, true),
        );

        let elem = read_primary_key_tree(&drive, &contract, "brandNewRangeSummableDocType");
        match elem {
            Element::ProvableSumTree(_, sum, _) => {
                assert_eq!(sum, 0, "freshly created ProvableSumTree should have sum 0");
            }
            other => panic!(
                "new rangeSummable doctype must materialize a ProvableSumTree primary-key tree, \
                 got {:?}",
                other
            ),
        }
    }

    /// `documentsAverageable: "score"` desugars to count + sum →
    /// the primary-key tree must be `CountSumTree` (count and sum
    /// aggregates fused on one tree). Regression for the same `_`
    /// catch-all that pre-fix produced `NormalTree`.
    #[test]
    fn test_update_contract_v0_adds_new_averageable_type_creates_count_sum_tree() {
        let (drive, contract, _) = update_contract_with_new_document_type(
            "brandNewAverageableDocType",
            averageable_document_schema(false),
        );

        let elem = read_primary_key_tree(&drive, &contract, "brandNewAverageableDocType");
        match elem {
            Element::CountSumTree(_, count, sum, _) => {
                assert_eq!(
                    (count, sum),
                    (0, 0),
                    "freshly created CountSumTree should have count=0 and sum=0"
                );
            }
            other => panic!(
                "new documentsAverageable doctype must materialize a CountSumTree primary-key \
                 tree, got {:?}",
                other
            ),
        }
    }

    /// `rangeAverageable: true` promotes both range axes →
    /// primary-key tree must be `ProvableCountProvableSumTree`
    /// (PCPS — the combined provable variant from grovedb #670).
    /// Regression for the same `_` catch-all.
    #[test]
    fn test_update_contract_v0_adds_new_range_averageable_type_creates_pcps_tree() {
        let (drive, contract, _) = update_contract_with_new_document_type(
            "brandNewRangeAverageableDocType",
            averageable_document_schema(true),
        );

        let elem = read_primary_key_tree(&drive, &contract, "brandNewRangeAverageableDocType");
        match elem {
            Element::ProvableCountProvableSumTree(_, count, sum, _) => {
                assert_eq!(
                    (count, sum),
                    (0, 0),
                    "freshly created PCPS tree should have count=0 and sum=0"
                );
            }
            other => panic!(
                "new rangeAverageable doctype must materialize a ProvableCountProvableSumTree \
                 primary-key tree, got {:?}",
                other
            ),
        }
    }

    /// Top-level index dispatch on a freshly added doctype: an
    /// index with `summable: "amount"` (no rangeSummable / range-
    /// countable) — the top-level property-name tree (at
    /// `[..doctype, "userId"]`) stays `NormalTree`. `summable` only
    /// affects the value-tree at the terminator level under the
    /// userId-keyed branch; per the 4-way `(range_countable,
    /// range_summable)` dispatch, the top-level structure under
    /// `(false, false)` is NormalTree. This pins the un-promoted
    /// top-level shape; deeper-level summable dispatch is exercised
    /// by `add_indices_for_index_level_for_contract_operations`
    /// tests.
    #[test]
    fn test_update_contract_v0_summable_only_top_level_index_stays_normal_tree() {
        let (drive, contract, _) = update_contract_with_new_document_type(
            "brandNewIndexedSummable",
            schema_with_indexed_summable(true, false, false, false),
        );

        let elem =
            read_top_level_index_tree(&drive, &contract, "brandNewIndexedSummable", "userId");
        assert!(
            matches!(elem, Element::Tree(..)),
            "summable-only index without rangeSummable keeps top-level NormalTree (point-lookup \
             sum lives at the terminator); got {:?}",
            elem
        );
    }

    /// Index with `rangeSummable: true` on a non-key range field
    /// must materialize a `ProvableSumTree` at the property-name
    /// level (`[..doctype, "userId"]`). Regression for the pre-fix
    /// `batch_insert_empty_tree` unconditional NormalTree at the
    /// top-level-index step.
    #[test]
    fn test_update_contract_v0_adds_new_range_summable_top_level_index_creates_provable_sum_tree() {
        let (drive, contract, _) = update_contract_with_new_document_type(
            "brandNewIndexedRangeSummable",
            schema_with_indexed_summable(true, true, false, false),
        );

        let elem =
            read_top_level_index_tree(&drive, &contract, "brandNewIndexedRangeSummable", "userId");
        match elem {
            Element::ProvableSumTree(_, sum, _) => {
                assert_eq!(
                    sum, 0,
                    "freshly created top-level ProvableSumTree should have sum 0"
                );
            }
            other => panic!(
                "rangeSummable top-level index must materialize a ProvableSumTree at the \
                 property-name level, got {:?}",
                other
            ),
        }
    }

    /// Index with both `rangeCountable` and `rangeSummable` →
    /// `ProvableCountProvableSumTree` (PCPS) at the property-name
    /// level. Regression for the missing `(true, true)` dispatch
    /// arm pre-fix.
    #[test]
    fn test_update_contract_v0_adds_new_range_count_and_summable_top_level_index_creates_pcps() {
        let (drive, contract, _) = update_contract_with_new_document_type(
            "brandNewIndexedRangeCountSummable",
            schema_with_indexed_summable(true, true, true, true),
        );

        let elem = read_top_level_index_tree(
            &drive,
            &contract,
            "brandNewIndexedRangeCountSummable",
            "userId",
        );
        match elem {
            Element::ProvableCountProvableSumTree(_, count, sum, _) => {
                assert_eq!(
                    (count, sum),
                    (0, 0),
                    "freshly created top-level PCPS tree should have count=0 and sum=0"
                );
            }
            other => panic!(
                "rangeCountable + rangeSummable top-level index must materialize a \
                 ProvableCountProvableSumTree at the property-name level, got {:?}",
                other
            ),
        }
    }

    /// Index with `rangeCountable: true` (only) → property-name
    /// tree must be `ProvableCountTree`. Pins the (true, false)
    /// arm of the 4-way dispatch so a refactor that consolidates
    /// arms can't silently regress.
    #[test]
    fn test_update_contract_v0_adds_new_range_countable_top_level_index_creates_provable_count_tree(
    ) {
        let (drive, contract, _) = update_contract_with_new_document_type(
            "brandNewIndexedRangeCountable",
            schema_with_indexed_summable(false, false, true, true),
        );

        let elem =
            read_top_level_index_tree(&drive, &contract, "brandNewIndexedRangeCountable", "userId");
        match elem {
            Element::ProvableCountTree(_, count, _) => {
                assert_eq!(
                    count, 0,
                    "freshly created top-level ProvableCountTree should have count 0"
                );
            }
            other => panic!(
                "rangeCountable top-level index must materialize a ProvableCountTree at the \
                 property-name level, got {:?}",
                other
            ),
        }
    }

    /// Insert a doctype FIRST without any range-countable index, then
    /// add a `rangeSummable`/`rangeCountable` index via a SECOND
    /// `apply_contract` call — exercises the existing-doctype branch
    /// in `update_contract_operations_v0` (the `if let Some(...)`
    /// arm at the top of the loop). The dispatch must materialize
    /// the property-name tree with the matching tree variant, NOT
    /// the unconditional `NormalTree` the pre-fix code used.
    ///
    /// Returns `(drive, contract_after_update)` so each per-shape
    /// test can read the materialized tree element it cares about.
    fn update_existing_doctype_with_new_indexed_index(
        document_type_name: &str,
        index_summable: bool,
        index_range_summable: bool,
        index_countable: bool,
        index_range_countable: bool,
    ) -> (Drive, dpp::prelude::DataContract) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Step 1: add the doctype with NO indices.
        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let bare_schema = platform_value!({
            "type": "object",
            "properties": {
                "userId": {
                    "type": "array",
                    "byteArray": true,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "minItems": 32,
                    "maxItems": 32,
                    "position": 0,
                },
                "amount": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 1000000,
                    "position": 1,
                },
            },
            "required": ["userId", "amount"],
            "additionalProperties": false,
        });
        contract
            .set_document_schema(
                document_type_name,
                bare_schema,
                true,
                &mut vec![],
                platform_version,
            )
            .expect("set bare schema");

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("initial insert with bare doctype");

        // Step 2: update the EXISTING doctype to add a top-level
        // index. This is the branch under test — the doctype
        // already exists, so we hit the `if let Some(original)` arm
        // not the `else` (new doctype) arm.
        let new_schema = schema_with_indexed_summable(
            index_summable,
            index_range_summable,
            index_countable,
            index_range_countable,
        );
        contract
            .set_document_schema(
                document_type_name,
                new_schema,
                true,
                &mut vec![],
                platform_version,
            )
            .expect("set updated schema");
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
            .expect("update existing doctype with new index");

        (drive, contract)
    }

    /// Existing-doctype branch: adding a `rangeSummable: true`
    /// index to a doctype that already exists must materialize the
    /// property-name tree as `ProvableSumTree`. Regression for the
    /// pre-fix `batch_insert_empty_tree_if_not_exists(...,
    /// TreeType::NormalTree, ...)` unconditional NormalTree at the
    /// top-level-index step in the existing-doctype branch — the
    /// new-doctype branch was already fixed in 64051f3f, but the
    /// existing-doctype branch was missed.
    #[test]
    fn test_update_contract_v0_adds_range_summable_index_to_existing_doctype_creates_provable_sum_tree(
    ) {
        let (drive, contract) = update_existing_doctype_with_new_indexed_index(
            "existingDoctypeRangeSummable",
            true,
            true,
            false,
            false,
        );

        let elem =
            read_top_level_index_tree(&drive, &contract, "existingDoctypeRangeSummable", "userId");
        match elem {
            Element::ProvableSumTree(_, sum, _) => {
                assert_eq!(
                    sum, 0,
                    "freshly created top-level ProvableSumTree (existing-doctype branch) \
                     should have sum 0"
                );
            }
            other => panic!(
                "rangeSummable index added to EXISTING doctype must materialize a \
                 ProvableSumTree at the property-name level, got {:?}",
                other
            ),
        }
    }

    /// Existing-doctype branch: adding a `rangeCountable: true`
    /// index to a doctype that already exists must materialize
    /// `ProvableCountTree`. Mirror of the previous test on the
    /// count axis — pins that the existing-doctype dispatch
    /// covers all four `(range_countable, range_summable)` arms.
    #[test]
    fn test_update_contract_v0_adds_range_countable_index_to_existing_doctype_creates_provable_count_tree(
    ) {
        let (drive, contract) = update_existing_doctype_with_new_indexed_index(
            "existingDoctypeRangeCountable",
            false,
            false,
            true,
            true,
        );

        let elem =
            read_top_level_index_tree(&drive, &contract, "existingDoctypeRangeCountable", "userId");
        match elem {
            Element::ProvableCountTree(_, count, _) => {
                assert_eq!(
                    count, 0,
                    "freshly created top-level ProvableCountTree (existing-doctype branch) \
                     should have count 0"
                );
            }
            other => panic!(
                "rangeCountable index added to EXISTING doctype must materialize a \
                 ProvableCountTree at the property-name level, got {:?}",
                other
            ),
        }
    }

    /// Existing-doctype branch: `rangeCountable + rangeSummable`
    /// → PCPS at the property-name level. Pins the `(true, true)`
    /// arm of the existing-doctype dispatch.
    #[test]
    fn test_update_contract_v0_adds_pcps_index_to_existing_doctype_creates_pcps_tree() {
        let (drive, contract) = update_existing_doctype_with_new_indexed_index(
            "existingDoctypePcps",
            true,
            true,
            true,
            true,
        );

        let elem = read_top_level_index_tree(&drive, &contract, "existingDoctypePcps", "userId");
        match elem {
            Element::ProvableCountProvableSumTree(_, count, sum, _) => {
                assert_eq!(
                    (count, sum),
                    (0, 0),
                    "freshly created top-level PCPS (existing-doctype branch) should have \
                     count=0 and sum=0"
                );
            }
            other => panic!(
                "rangeCountable + rangeSummable index added to EXISTING doctype must \
                 materialize a ProvableCountProvableSumTree at the property-name level, \
                 got {:?}",
                other
            ),
        }
    }

    /// Existing-doctype branch: unflagged index → `NormalTree`.
    /// Pins that the `(false, false)` default arm still routes
    /// through the dispatch — without this test a future refactor
    /// could fall through to a wrong default without tripping any
    /// of the sum/count-flagged tests.
    #[test]
    fn test_update_contract_v0_adds_unflagged_index_to_existing_doctype_keeps_normal_tree() {
        let (drive, contract) = update_existing_doctype_with_new_indexed_index(
            "existingDoctypeUnflagged",
            false,
            false,
            false,
            false,
        );

        let elem =
            read_top_level_index_tree(&drive, &contract, "existingDoctypeUnflagged", "userId");
        assert!(
            matches!(elem, Element::Tree(..)),
            "unflagged index on existing doctype must stay NormalTree; got {:?}",
            elem
        );
    }
}
