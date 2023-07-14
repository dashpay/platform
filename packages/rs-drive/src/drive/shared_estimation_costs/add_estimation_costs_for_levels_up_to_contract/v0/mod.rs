use crate::drive::defaults::{DEFAULT_HASH_SIZE_U8, ESTIMATED_AVERAGE_DOCUMENT_TYPE_NAME_SIZE};

use crate::drive::flags::StorageFlags;
use crate::drive::{contract_documents_path, Drive};

use dpp::data_contract::DataContract;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::AllSubtrees;

use crate::drive::contract::paths::{all_contracts_global_root_path, contract_root_path};
use dpp::version::drive_versions::DriveVersion;
use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_estimation_costs_for_levels_up_to_contract_v0(
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
}
