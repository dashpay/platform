use crate::drive::contract::paths;

use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::drive::document::ranked_index_tree_type::property_name_tree_type_and_ranked_axes;
use crate::drive::{contract_documents_path, votes, Drive, RootTree};
use crate::util::object_size_info::DriveKeyInfo::{Key, KeyRef};
use crate::util::storage_flags::StorageFlags;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use dpp::serialization::PlatformSerializableWithPlatformVersion;

use crate::drive::votes::paths::{
    CONTESTED_DOCUMENT_INDEXES_TREE_KEY, CONTESTED_DOCUMENT_STORAGE_TREE_KEY,
};
use crate::error::contract::DataContractError;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Insert a contract.
    #[inline(always)]
    pub(super) fn insert_contract_v0(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let storage_flags = if contract.config().can_be_deleted() || !contract.config().readonly() {
            Some(StorageFlags::new_single_epoch(
                block_info.epoch.index,
                Some(contract.owner_id().to_buffer()),
            ))
        } else {
            None
        };

        let serialized_contract =
            contract.serialize_to_bytes_with_platform_version(platform_version)?;

        if serialized_contract.len() as u64 > u32::MAX as u64
            || serialized_contract.len() as u32
                > platform_version.dpp.contract_versions.max_serialized_size
        {
            // This should normally be caught by DPP, but there is a rare possibility that the
            // re-serialized size is bigger than the original serialized data contract.
            return Err(Error::DataContract(DataContractError::ContractTooBig(format!("Trying to insert a data contract of size {} that is over the max allowed insertion size {}", serialized_contract.len(), platform_version.dpp.contract_versions.max_serialized_size))));
        }

        let contract_element = Element::Item(
            serialized_contract,
            StorageFlags::map_to_some_element_flags(storage_flags.as_ref()),
        );

        self.insert_contract_element_v0(
            contract_element,
            contract,
            &block_info,
            apply,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )
    }

    /// Adds a contract to storage using `add_contract_to_storage`
    /// and inserts the empty trees which will be necessary to later insert documents.
    #[allow(clippy::too_many_arguments)]
    fn insert_contract_element_v0(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        let batch_operations = self.insert_contract_operations_v0(
            contract_element,
            contract,
            block_info,
            &mut estimated_costs_only_with_layer_info,
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

    /// The operations for adding a contract.
    /// These operations add a contract to storage using `add_contract_to_storage`
    /// and insert the empty trees which will be necessary to later insert documents.
    #[inline(always)]
    pub(super) fn insert_contract_add_operations_v0(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations = self.insert_contract_operations_v0(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            platform_version,
        )?;
        drive_operations.extend(batch_operations);
        Ok(())
    }

    /// The operations for adding a contract.
    /// These operations add a contract to storage using `add_contract_to_storage`
    /// and insert the empty trees which will be necessary to later insert documents.
    pub(in crate::drive::contract::insert::insert_contract) fn insert_contract_operations_v0(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        let storage_flags = StorageFlags::map_some_element_flags_ref(contract_element.get_flags())?;

        self.batch_insert_empty_tree(
            [Into::<&[u8; 1]>::into(RootTree::DataContractDocuments).as_slice()],
            KeyRef(contract.id_ref().as_bytes()),
            storage_flags.as_ref(),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        self.add_contract_to_storage(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            &mut batch_operations,
            true,
            None, // we are not inserting into history, hence the transaction will not be used, we can pass None
            &platform_version.drive,
        )?;

        // the documents
        let contract_root_path = paths::contract_root_path(contract.id_ref().as_bytes());
        let key_info = Key(vec![1]);
        self.batch_insert_empty_tree(
            contract_root_path,
            key_info,
            storage_flags.as_ref(),
            &mut batch_operations,
            &platform_version.drive,
        )?;

        // If the contract happens to contain any contested indexes then we add the contract to the
        //  contested contracts

        let document_types_with_contested_indexes =
            contract.document_types_with_contested_indexes();

        if !document_types_with_contested_indexes.is_empty() {
            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Self::add_estimation_costs_for_contested_document_tree_levels_up_to_contract(
                    contract,
                    None,
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;
            }

            let contested_contract_root_path =
                votes::paths::vote_contested_resource_active_polls_tree_path();

            self.batch_insert_empty_tree(
                contested_contract_root_path,
                KeyRef(contract.id_ref().as_bytes()),
                storage_flags.as_ref(),
                &mut batch_operations,
                &platform_version.drive,
            )?;

            let contested_unique_index_contract_document_types_path =
                votes::paths::vote_contested_resource_active_polls_contract_tree_path(
                    contract.id_ref().as_bytes(),
                );

            for (type_key, _document_type) in document_types_with_contested_indexes.into_iter() {
                self.batch_insert_empty_tree(
                    contested_unique_index_contract_document_types_path,
                    KeyRef(type_key.as_bytes()),
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;

                let type_path = [
                    contested_unique_index_contract_document_types_path[0],
                    contested_unique_index_contract_document_types_path[1],
                    contested_unique_index_contract_document_types_path[2],
                    contested_unique_index_contract_document_types_path[3],
                    type_key.as_bytes(),
                ];

                // primary key tree
                let key_info_storage = Key(vec![CONTESTED_DOCUMENT_STORAGE_TREE_KEY]);
                self.batch_insert_empty_tree(
                    type_path,
                    key_info_storage,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;

                // index key tree
                let key_info_indexes = Key(vec![CONTESTED_DOCUMENT_INDEXES_TREE_KEY]);
                self.batch_insert_empty_tree(
                    type_path,
                    key_info_indexes,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            }
        }

        // next we should store each document type
        // right now we are referring them by name
        // todo: maybe change this to be a reference by index
        let contract_documents_path = contract_documents_path(contract.id_ref().as_bytes());

        for (type_key, document_type) in contract.document_types().iter() {
            self.batch_insert_empty_tree(
                contract_documents_path,
                KeyRef(type_key.as_bytes()),
                storage_flags.as_ref(),
                &mut batch_operations,
                &platform_version.drive,
            )?;

            let type_path = [
                contract_documents_path[0],
                contract_documents_path[1],
                contract_documents_path[2],
                type_key.as_bytes(),
            ];

            // primary key tree — route through the centralized
            // primary_key_tree_type() so contract creation, document inserts,
            // deletes, and estimation paths all see the same tree-variant
            // selection (under whichever drive method version is active).
            let key_info = Key(vec![0]);
            match document_type
                .as_ref()
                .primary_key_tree_type(platform_version)?
            {
                TreeType::ProvableCountTree => self.batch_insert_empty_provable_count_tree(
                    type_path,
                    key_info,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?,
                TreeType::CountTree => self.batch_insert_empty_count_tree(
                    type_path,
                    key_info,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?,
                // Sum-capable variants — route to the matching helper so the
                // doctype's primary-key tree is created with the correct
                // sum-bearing element variant at contract apply time. Without
                // these arms the previous catch-all `_` arm would create a
                // plain `NormalTree`, and subsequent sum-aware document
                // inserts / range proofs would operate on the wrong element
                // type.
                TreeType::SumTree => self.batch_insert_empty_sum_tree(
                    type_path,
                    key_info,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?,
                TreeType::ProvableSumTree => self.batch_insert_empty_provable_sum_tree(
                    type_path,
                    key_info,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?,
                TreeType::ProvableCountSumTree => self.batch_insert_empty_provable_count_sum_tree(
                    type_path,
                    key_info,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?,
                TreeType::ProvableCountProvableSumTree => self
                    .batch_insert_empty_provable_count_provable_sum_tree(
                        type_path,
                        key_info,
                        storage_flags.as_ref(),
                        &mut batch_operations,
                        &platform_version.drive,
                    )?,
                TreeType::CountSumTree => self.batch_insert_empty_count_sum_tree(
                    type_path,
                    key_info,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?,
                _ => self.batch_insert_empty_tree(
                    type_path,
                    key_info,
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?,
            }

            let document_type_ref = document_type.as_ref();
            let index_structure = document_type_ref.index_structure();
            // For each type we should insert the indices that are top level.
            // The index structure's root sub-levels are exactly the distinct
            // top-level trees: one per plain first property, plus one per
            // (property, grid) pair for time-range-transformed first
            // properties, whose keys are already grid-qualified
            // (`TimeRangeTransform::storage_key`). Iterating the map also
            // dedupes indexes sharing a first level for free.
            for (level_key, level) in index_structure.sub_levels() {
                let index_bytes = level_key.as_bytes();
                {
                    // The property-name tree variant (the tree at
                    // `@/contract/0x01/<doctype>/<prop>`) is selected from
                    // the index's `(range_countable, range_summable)`
                    // pair — the same 4-way dispatch table the compound-
                    // index walker uses for nested levels (see
                    // [`Drive::add_indices_for_index_level_for_contract_operations_v0`]
                    // around line 195 of
                    // `add_indices_for_index_level_for_contract_operations/v0/mod.rs`,
                    // where `property_name_tree_type` is computed from the
                    // same two axes for sub-levels). Keeping the two
                    // dispatch tables in lock-step is what lets top-level
                    // single-property indexes share the read-path with
                    // their compound siblings.
                    //
                    // - `range_countable: true` → ProvableCountTree
                    //   (existing): so `AggregateCountOnRange` walks land.
                    // - `range_summable: true` → ProvableSumTree (NEW):
                    //   so `AggregateSumOnRange` walks land. Before the
                    //   fix this path silently fell through to NormalTree
                    //   and any sum-on-range query against a top-level
                    //   `rangeSummable` index errored with
                    //   "AggregateSumOnRange is only valid against
                    //   ProvableSumTree or ProvableCountProvableSumTree,
                    //   got NormalTree".
                    // - both → ProvableCountProvableSumTree (PCPS,
                    //   grovedb PR 670 combined surface): one tree
                    //   carries both metrics per-node.
                    // - neither → NormalTree (default; matches v0).
                    //
                    // Meta schema v3 (PV14) layers the ranking axes on top:
                    // any `ranked*` flag upgrades the chosen variant to its
                    // *indexed* mirror, which additionally carries one
                    // ordered secondary Merk per axis. Note this dispatch
                    // only ever sees a TERMINAL level — `has_index_with_type`
                    // is `Some` exactly for single-property indexes, whose
                    // top-level property-name tree IS the terminal one. A
                    // compound index's terminal level lives deeper and is
                    // materialized lazily by the document index walker.
                    let index_info = level.has_index_with_type();
                    let (tree_type, ranked_axes) =
                        property_name_tree_type_and_ranked_axes(index_info)?;
                    match tree_type {
                        TreeType::ProvableCountProvableSumTree => self
                            .batch_insert_empty_provable_count_provable_sum_tree(
                                type_path,
                                KeyRef(index_bytes),
                                storage_flags.as_ref(),
                                &mut batch_operations,
                                &platform_version.drive,
                            )?,
                        TreeType::ProvableCountTree => self
                            .batch_insert_empty_provable_count_tree(
                                type_path,
                                KeyRef(index_bytes),
                                storage_flags.as_ref(),
                                &mut batch_operations,
                                &platform_version.drive,
                            )?,
                        TreeType::ProvableSumTree => self.batch_insert_empty_provable_sum_tree(
                            type_path,
                            KeyRef(index_bytes),
                            storage_flags.as_ref(),
                            &mut batch_operations,
                            &platform_version.drive,
                        )?,
                        TreeType::ProvableCountIndexedTree => self
                            .batch_insert_empty_provable_count_indexed_tree(
                                type_path,
                                KeyRef(index_bytes),
                                storage_flags.as_ref(),
                                &mut batch_operations,
                                &platform_version.drive,
                            )?,
                        TreeType::ProvableSumIndexedTree => self
                            .batch_insert_empty_provable_sum_indexed_tree(
                                type_path,
                                KeyRef(index_bytes),
                                storage_flags.as_ref(),
                                &mut batch_operations,
                                &platform_version.drive,
                            )?,
                        TreeType::ProvableCountProvableSumIndexedTree => self
                            .batch_insert_empty_provable_count_provable_sum_indexed_tree(
                                type_path,
                                KeyRef(index_bytes),
                                &ranked_axes,
                                storage_flags.as_ref(),
                                &mut batch_operations,
                                &platform_version.drive,
                            )?,
                        // NormalTree, and defensively anything the resolver
                        // could grow later: a plain subtree.
                        _ => self.batch_insert_empty_tree(
                            type_path,
                            KeyRef(index_bytes),
                            storage_flags.as_ref(),
                            &mut batch_operations,
                            &platform_version.drive,
                        )?,
                    }
                }
            }
        }

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_contract_insertion(
                contract,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }

        Ok(batch_operations)
    }
}

#[cfg(test)]
mod tests;
