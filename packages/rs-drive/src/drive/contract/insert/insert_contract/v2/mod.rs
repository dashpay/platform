// Protocol 14 generation: keep-history document types get their per-type
// history tree at contract insertion; everything else matches v1.
use crate::drive::votes::paths::{
    CONTESTED_DOCUMENT_INDEXES_TREE_KEY, CONTESTED_DOCUMENT_STORAGE_TREE_KEY,
};
use crate::drive::Drive;
use crate::error::contract::DataContractError;
use crate::util::storage_flags::StorageFlags;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;

use crate::drive::balances::total_tokens_root_supply_path_vec;
use crate::drive::contract::paths;
use crate::drive::document::primary_key_tree_type::DocumentTypePrimaryKeyTreeType;
use crate::drive::document::ranked_index_tree_type::property_name_tree_type_and_ranked_axes_for_level;
use crate::drive::tokens::paths::{
    token_balances_path_vec, token_balances_root_path, token_contract_infos_root_path,
    token_identity_infos_root_path, token_statuses_root_path,
};
use crate::drive::{contract_documents_path, votes, RootTree};
use crate::util::object_size_info::DriveKeyInfo::{Key, KeyRef};
use crate::util::object_size_info::PathKeyElementInfo::PathKeyElement;
use crate::util::object_size_info::{DriveKeyInfo, PathKeyElementInfo};
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::serialization::{PlatformSerializable, PlatformSerializableWithPlatformVersion};
use dpp::tokens::contract_info::TokenContractInfo;
use dpp::tokens::status::TokenStatus;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::Element::SumItem;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Insert a contract.
    #[inline(always)]
    pub(super) fn insert_contract_v2(
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

        self.insert_contract_element_v2(
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
    fn insert_contract_element_v2(
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
        let batch_operations = self.insert_contract_operations_v2(
            contract_element,
            contract,
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

    /// The operations for adding a contract.
    /// These operations add a contract to storage using `add_contract_to_storage`
    /// and insert the empty trees which will be necessary to later insert documents.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn insert_contract_add_operations_v2(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let batch_operations = self.insert_contract_operations_v2(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        drive_operations.extend(batch_operations);
        Ok(())
    }

    /// The operations for adding a contract.
    /// These operations add a contract to storage using `add_contract_to_storage`
    /// and insert the empty trees which will be necessary to later insert documents.
    fn insert_contract_operations_v2(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = self
            .insert_contract_base_operations_v2(
                contract_element,
                contract,
                block_info,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;

        if !contract.tokens().is_empty() {
            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Drive::add_estimation_costs_for_token_status_infos(
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;

                Drive::add_estimation_costs_for_token_contract_infos(
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;
            }
        }

        for (token_pos, token_config) in contract.tokens() {
            let token_id = contract.token_id(*token_pos).ok_or(Error::DataContract(
                DataContractError::CorruptedDataContract(format!(
                    "data contract has a token at position {}, but can not find it",
                    token_pos
                )),
            ))?;

            let token_id_bytes = token_id.to_buffer();

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Drive::add_estimation_costs_for_token_balances(
                    token_id_bytes,
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;
                Drive::add_estimation_costs_for_token_identity_infos(
                    token_id_bytes,
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;
                Drive::add_estimation_costs_for_token_total_supply(
                    estimated_costs_only_with_layer_info,
                    &platform_version.drive,
                )?;
            }

            self.batch_insert_empty_sum_tree(
                token_balances_root_path(),
                DriveKeyInfo::KeyRef(token_id_bytes.as_slice()),
                None,
                &mut batch_operations,
                &platform_version.drive,
            )?;

            self.batch_insert_empty_tree(
                token_identity_infos_root_path(),
                DriveKeyInfo::KeyRef(token_id_bytes.as_slice()),
                None,
                &mut batch_operations,
                &platform_version.drive,
            )?;

            if let Some(perpetual_distribution) =
                token_config.distribution_rules().perpetual_distribution()
            {
                self.add_perpetual_distribution(
                    token_id.to_buffer(),
                    perpetual_distribution,
                    estimated_costs_only_with_layer_info,
                    &mut batch_operations,
                    transaction,
                    platform_version,
                )?;
            }

            if token_config.start_as_paused() {
                // no status also means active.
                let starting_status = TokenStatus::new(true, platform_version)?;
                let token_status_bytes = starting_status.serialize_consume_to_bytes()?;

                self.batch_insert(
                    PathKeyElementInfo::PathFixedSizeKeyRefElement::<2>((
                        token_statuses_root_path(),
                        token_id.as_slice(),
                        Element::Item(token_status_bytes, None),
                    )),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            }

            let token_contract_info =
                TokenContractInfo::new(contract.id(), *token_pos, platform_version)?;
            let token_contract_info_bytes = token_contract_info.serialize_consume_to_bytes()?;

            self.batch_insert(
                PathKeyElementInfo::PathFixedSizeKeyRefElement::<2>((
                    token_contract_infos_root_path(),
                    token_id.as_slice(),
                    Element::Item(token_contract_info_bytes, None),
                )),
                &mut batch_operations,
                &platform_version.drive,
            )?;

            if let Some(pre_programmed_distribution) = token_config
                .distribution_rules()
                .pre_programmed_distribution()
            {
                self.add_pre_programmed_distributions(
                    token_id.to_buffer(),
                    contract.owner_id().to_buffer(),
                    pre_programmed_distribution,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    &mut batch_operations,
                    transaction,
                    platform_version,
                )?;
            }

            let path_holding_total_token_supply = total_tokens_root_supply_path_vec();

            if token_config.base_supply() > 0 {
                // We have a base supply that needs to be distributed on contract creation
                let destination_identity_id = token_config
                    .distribution_rules()
                    .new_tokens_destination_identity()
                    .copied()
                    .unwrap_or(contract.owner_id());
                let token_balance_path = token_balances_path_vec(token_id_bytes);

                if token_config.base_supply() > i64::MAX as u64 {
                    return Err(
                        ProtocolError::CriticalCorruptedCreditsCodeExecution(format!(
                            "Token base supply over i64 max, is {}",
                            token_config.base_supply()
                        ))
                        .into(),
                    );
                }
                self.batch_insert::<0>(
                    PathKeyElement((
                        token_balance_path,
                        destination_identity_id.to_vec(),
                        Element::new_sum_item(token_config.base_supply() as i64),
                    )),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
                self.batch_insert::<0>(
                    PathKeyElement((
                        path_holding_total_token_supply,
                        token_id.to_vec(),
                        Element::new_sum_item(token_config.base_supply() as i64),
                    )),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            } else {
                self.batch_insert::<0>(
                    PathKeyElement((
                        path_holding_total_token_supply,
                        token_id.to_vec(),
                        SumItem(0, None),
                    )),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            }
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

        if !contract.keywords().is_empty() {
            batch_operations.extend(self.add_new_contract_keywords_operations(
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
            batch_operations.extend(self.add_new_contract_description_operations(
                contract.id(),
                contract.owner_id(),
                description,
                false,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        Ok(batch_operations)
    }

    #[allow(clippy::too_many_arguments)]
    /// The operations for adding a contract.
    /// These operations add a contract to storage using `add_contract_to_storage`
    /// and insert the empty trees which will be necessary to later insert documents.
    fn insert_contract_base_operations_v2(
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

            if document_type.as_ref().documents_keep_history() {
                self.batch_insert_empty_tree(
                    type_path,
                    KeyRef(&[crate::drive::document::paths::DOCUMENT_HISTORY_TREE_KEY]),
                    storage_flags.as_ref(),
                    &mut batch_operations,
                    &platform_version.drive,
                )?;
            }

            // indexOnly document types have no primary-key tree at all —
            // the index entries are the rows, and nothing is ever addressed
            // by document id, so the `[0]` tree is skipped and only the
            // top-level property-name trees below are created.
            // `index_only()` can only be true on a PV14+ contract (the
            // grammar rejects the keyword below meta-schema v3), so
            // historical contract inserts replay byte-identically.
            if !document_type.as_ref().index_only() {
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
                    TreeType::ProvableCountSumTree => self
                        .batch_insert_empty_provable_count_sum_tree(
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
                    // ordered secondary Merk per axis. This dispatch sees a
                    // TERMINAL level for single-property indexes (whose
                    // top-level property-name tree IS the terminal one) —
                    // a compound index's terminal level lives deeper and is
                    // materialized lazily by the document index walker —
                    // and, since the `rankedCountable: { at }` grammar, a
                    // GROUPING level when a compound index ranks at its
                    // first property: the level-aware resolver then yields
                    // the Count-axis indexed tree, created here so the
                    // ranking secondary exists from registration.
                    let (tree_type, ranked_axes) =
                        property_name_tree_type_and_ranked_axes_for_level(level)?;
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
mod tests {
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::tests::fixtures::get_dashpay_contract_fixture;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    /// Exercises the `base_supply > i64::MAX as u64` overflow branch in
    /// `insert_contract_operations_v2`, which returns
    /// `ProtocolError::CriticalCorruptedCreditsCodeExecution`.
    ///
    /// PR #3516 only covered base_supply==0, base_supply>0 within range, and
    /// base_supply==0 with custom destination identity. This test specifically
    /// drives the `i64::MAX` guard.
    #[test]
    fn test_insert_contract_with_token_base_supply_overflow_fails() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();

        // Set base_supply to u64::MAX, which is > i64::MAX.
        let token_config = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(u64::MAX),
        );
        contract.set_tokens(BTreeMap::from([(0, token_config)]));

        let result = drive.insert_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            platform_version,
        );

        assert!(
            matches!(
                &result,
                Err(crate::error::Error::Protocol(boxed))
                    if matches!(
                        boxed.as_ref(),
                        dpp::ProtocolError::CriticalCorruptedCreditsCodeExecution(_)
                    )
            ),
            "Expected CriticalCorruptedCreditsCodeExecution, got: {:?}",
            result
        );
    }

    /// Exercises the estimated-costs branches in `insert_contract_operations_v2`
    /// when tokens are present. Calling `insert_contract` with `apply=false`
    /// populates `estimated_costs_only_with_layer_info = Some(..)`, causing the
    /// `add_estimation_costs_for_token_*` calls (token_status_infos,
    /// token_contract_infos, token_balances, token_identity_infos,
    /// token_total_supply) to execute. This is a separate branch from the
    /// apply=true path PR #3516 covered.
    #[test]
    fn test_insert_contract_v1_token_estimated_costs_branches() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();

        // Add two tokens so the loop in insert_contract_operations_v2 iterates more
        // than once; this helps exercise estimation-cost paths per token.
        let mut paused_config =
            TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
        paused_config.set_start_as_paused(true);
        let normal_config = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(500),
        );
        contract.set_tokens(BTreeMap::from([(0, paused_config), (1, normal_config)]));

        // apply=false forces the estimation branches.
        let fee = drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                false,
                None,
                platform_version,
            )
            .expect("estimation insert should succeed with tokens");

        assert!(
            fee.processing_fee > 0 || fee.storage_fee > 0,
            "estimation should produce non-zero fees"
        );
    }

    /// Exercises the `insert_contract_v2` early-exit for `contract.groups().is_empty()`:
    /// PR #3516 covered the non-empty groups branch. This test complements by driving
    /// the empty-groups path (false-branch of `if !contract.groups().is_empty()`) while
    /// also asserting token+keyword insertion still works on a separate contract id.
    #[test]
    fn test_insert_contract_v1_empty_groups_with_tokens_and_keywords() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();

        // Tokens yes, groups intentionally empty.
        let token_config = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(42),
        );
        contract.set_tokens(BTreeMap::from([(0, token_config)]));
        assert!(contract.groups().is_empty());

        // Use apply=false so we don't need keyword_search contract in grove yet.
        // This covers the empty-groups-AND-empty-keywords-AND-no-description branches
        // while still exercising the token loop.
        let fee = drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                false,
                None,
                platform_version,
            )
            .expect("should succeed with tokens only");
        assert!(fee.processing_fee > 0 || fee.storage_fee > 0);
    }

    /// Exercises `insert_contract_v2` with a token whose position doesn't round-trip
    /// via `token_id(pos)`. This is hard to actually trigger in practice because
    /// `token_id` hashes `contract.id || pos` deterministically. Instead, we
    /// verify the happy path where two tokens at different positions both get
    /// distinct token_ids and each receives its own balances / contract_infos /
    /// identity_infos trees.
    #[test]
    fn test_insert_contract_v1_two_tokens_distinct_ids_all_trees_created() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let mut contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();

        let c1 = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(10),
        );
        let c2 = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
        );
        contract.set_tokens(BTreeMap::from([(0, c1), (1, c2)]));

        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply with two tokens should succeed");

        // Token positions 0 and 1 must yield different ids.
        let id0 = contract.token_id(0).expect("token 0 id");
        let id1 = contract.token_id(1).expect("token 1 id");
        assert_ne!(
            id0, id1,
            "tokens at different positions must have distinct ids"
        );
    }

    /// The per-type history tree is a protocol 14 fact: the same keep-history
    /// contract inserted through the dispatcher gets the tree at 14 and does
    /// not at 13, where the shipped generation still runs.
    #[test]
    fn should_create_the_history_tree_only_from_protocol_14() {
        use crate::drive::document::paths::{
            contract_document_type_path_vec, DOCUMENT_HISTORY_TREE_KEY,
        };
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::tests::json_document::json_document_to_contract;

        for (protocol, expected) in [(13, false), (14, true)] {
            let version = PlatformVersion::get(protocol).expect("protocol version");
            let drive = setup_drive_with_initial_state_structure(Some(version));
            let contract = json_document_to_contract(
                "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
                false,
                version,
            )
            .expect("keep-history contract fixture");
            drive
                .insert_contract(&contract, BlockInfo::default(), true, None, version)
                .expect("insert contract");
            let type_path = contract_document_type_path_vec(contract.id().as_slice(), "profile");
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
