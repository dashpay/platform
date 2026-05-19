use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

use crate::drive::document::estimation_costs::estimated_sum_trees_for_value_tree_type::estimated_sum_trees_for_value_tree_type;
use crate::drive::document::unique_event_id;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;

use crate::drive::Drive;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfoV0Methods, PathInfo};

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

use crate::drive::document::paths::contract_document_type_path_vec;
use dpp::version::PlatformVersion;

impl Drive {
    /// Removes indices for the top index level and calls for lower levels.
    #[inline(always)]
    pub(super) fn remove_indices_for_top_index_level_for_contract_operations_v1(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        previous_batch_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let document_type = document_and_contract_info.document_type;
        let index_level = document_type.index_structure();
        let contract = document_and_contract_info.contract;
        let event_id = unique_event_id();
        let storage_flags =
            if document_type.documents_mutable() || contract.config().can_be_deleted() {
                document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_storage_flags_ref()
            } else {
                None //there are no need for storage flags if documents are not mutable and contract can not be deleted
            };

        // we need to construct the path for documents on the contract
        // the path is
        //  * Document andDataContract root tree
        //  *DataContract ID recovered from document
        //  * 0 to signify Documents and notDataContract
        let contract_document_type_path = contract_document_type_path_vec(
            document_and_contract_info.contract.id_ref().as_bytes(),
            document_and_contract_info.document_type.name().as_str(),
        );

        let sub_level_index_count = index_level.sub_levels().len() as u32;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // On this level we will have a 0 and all the top index paths
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(contract_document_type_path.clone()),
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    estimated_layer_count: ApproximateElements(sub_level_index_count + 1),
                    estimated_layer_sizes: AllSubtrees(
                        DEFAULT_HASH_SIZE_U8,
                        NoSumTrees,
                        storage_flags.map(|s| s.serialized_size()),
                    ),
                },
            );
        }

        // next we need to store a reference to the document for each index
        for (name, sub_level) in index_level.sub_levels() {
            // Property-name tree-type composition mirrors the
            // insert side's top-level walker. See
            // `add_indices_for_top_index_level_for_contract_operations_v1`
            // for the four-way dispatch table.
            let sub_level_info = sub_level.has_index_with_type();
            let sub_level_range_countable = sub_level_info
                .map(|info| info.range_countable)
                .unwrap_or(false);
            let sub_level_range_summable = sub_level_info
                .map(|info| info.range_summable)
                .unwrap_or(false);
            let property_name_tree_type =
                match (sub_level_range_countable, sub_level_range_summable) {
                    (true, true) => TreeType::ProvableCountProvableSumTree,
                    (true, false) => TreeType::ProvableCountTree,
                    (false, true) => TreeType::ProvableSumTree,
                    (false, false) => TreeType::NormalTree,
                };

            // Derive `value_tree_type` from the full four-axis flags
            // (mirror of the insert side's matrix). The delete walker
            // doesn't actually write anything, but its
            // `EstimatedLayerInformation` must reflect the value tree
            // type its children HAVE, so dry-run cost estimation
            // accounts for per-node aggregate bytes on summable /
            // rangeSummable / count-sum / PCPS value trees.
            let sub_level_is_countable_terminator = sub_level_info
                .map(|info| info.countable.is_countable())
                .unwrap_or(false);
            let sub_level_is_summable_terminator = sub_level_info
                .map(|info| info.summable.is_some())
                .unwrap_or(false);
            let value_tree_type = match (
                sub_level_is_countable_terminator,
                sub_level_range_countable,
                sub_level_is_summable_terminator,
                sub_level_range_summable,
            ) {
                (true, true, true, true) => TreeType::ProvableCountProvableSumTree,
                (true, false, true, false) => TreeType::CountSumTree,
                (true, true, true, false) => TreeType::ProvableCountSumTree,
                (true, false, true, true) => TreeType::ProvableCountProvableSumTree,
                (true, _, false, false) => TreeType::CountTree,
                (false, false, true, _) => TreeType::SumTree,
                (false, _, false, _) => TreeType::NormalTree,
                _ => TreeType::NormalTree,
            };

            // at this point the contract path is to the contract documents
            // for each index the top index component will already have been added
            // when the contract itself was created
            let mut index_path: Vec<Vec<u8>> = contract_document_type_path.clone();
            index_path.push(Vec::from(name.as_bytes()));

            // with the example of the dashpay contract's first index
            // the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId
            let document_top_field = document_and_contract_info
                .owned_document_info
                .document_info
                .get_raw_for_document_type(
                    name,
                    document_type,
                    document_and_contract_info.owned_document_info.owner_id,
                    Some((sub_level, event_id)),
                    platform_version,
                )?
                .unwrap_or_default();

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                let document_top_field_estimated_size = document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_estimated_size_for_document_type(name, document_type, platform_version)?;

                if document_top_field_estimated_size > u8::MAX as u16 {
                    return Err(Error::Fee(FeeError::Overflow(
                        "document top field is too big for being an index",
                    )));
                }

                // The property-name layer's children are value trees of
                // type `value_tree_type`. v0 emitted `NoSumTrees` here
                // unconditionally — correct only for pre-v12 contracts
                // whose value trees are always NormalTree. v1 maps
                // `value_tree_type` to the matching `SomeSumTrees`
                // weight slot via the shared helper.
                estimated_costs_only_with_layer_info.insert(
                    KeyInfoPath::from_known_owned_path(index_path.clone()),
                    EstimatedLayerInformation {
                        tree_type: property_name_tree_type,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: AllSubtrees(
                            document_top_field_estimated_size as u8,
                            estimated_sum_trees_for_value_tree_type(value_tree_type),
                            storage_flags.map(|s| s.serialized_size()),
                        ),
                    },
                );
            }

            let any_fields_null = document_top_field.is_empty();
            let all_fields_null = document_top_field.is_empty();

            let mut index_path_info = if document_and_contract_info
                .owned_document_info
                .document_info
                .is_document_size()
            {
                // This is a stateless operation
                PathInfo::PathWithSizes(KeyInfoPath::from_known_owned_path(index_path))
            } else {
                PathInfo::PathAsVec::<0>(index_path)
            };

            // we push the actual value of the index path
            index_path_info.push(document_top_field)?;
            // the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>

            self.remove_indices_for_index_level_for_contract_operations(
                document_and_contract_info,
                index_path_info,
                sub_level,
                any_fields_null,
                all_fields_null,
                value_tree_type,
                &storage_flags,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                event_id,
                transaction,
                batch_operations,
                platform_version,
            )?;
        }
        Ok(())
    }
}
