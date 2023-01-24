use crate::drive::contract::{all_contracts_global_root_path, contract_root_path};
use crate::drive::defaults::{DEFAULT_HASH_SIZE_U8, ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE};

use crate::drive::flags::StorageFlags;
use crate::drive::{contract_documents_path, Drive};

use dpp::data_contract::{DataContract, DriveContractExt};
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::AllSubtrees;

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

impl Drive {
    pub(crate) fn add_estimation_costs_for_levels_up_to_contract(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we have constructed the top layer so contract/documents tree are at the top
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(0, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // we then need to insert the contract layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(all_contracts_global_root_path()),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllSubtrees(
                    DEFAULT_HASH_SIZE_U8,
                    NoSumTrees,
                    Some(StorageFlags::approximate_size(true, None)),
                ),
            },
        );
    }

    pub(crate) fn add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
        contract: &DataContract,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        Self::add_estimation_costs_for_levels_up_to_contract(estimated_costs_only_with_layer_info);

        let document_type_count = contract.documents.len() as u32;

        // we only store the owner_id storage
        let storage_flags = if contract.can_be_deleted() {
            Some(StorageFlags::approximate_size(true, None))
        } else {
            None
        };

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_root_path(contract.id.as_bytes())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, storage_flags),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_documents_path(contract.id.as_bytes())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: ApproximateElements(document_type_count),
                estimated_layer_sizes: AllSubtrees(
                    ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE,
                    NoSumTrees,
                    storage_flags,
                ),
            },
        );
    }
}
