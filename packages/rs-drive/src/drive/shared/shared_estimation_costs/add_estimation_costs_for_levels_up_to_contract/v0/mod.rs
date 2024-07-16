use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;

use crate::drive::Drive;
use crate::util::storage_flags::StorageFlags;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::AllSubtrees;

use crate::drive::contract::paths::all_contracts_global_root_path;

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

impl Drive {
    /// Adds estimated costs for layers up to the contract level.
    ///
    /// This function populates the `estimated_costs_only_with_layer_info` hashmap with estimated layer information for the top level and the contract layer.
    /// These estimates are useful for optimizing GroveDB operations.
    ///
    /// # Parameters
    ///
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a hashmap that will be populated with estimated layer information for the top level and the contract layer.
    ///
    /// # Estimated Layer Information
    ///
    /// The function estimates two layers:
    ///
    /// 1. The top layer, which is an empty layer.
    /// 2. The contract layer, which contains all global contracts.
    ///
    /// These estimates are useful for optimizing batch insertions, deletions, and other operations in GroveDB.
    ///
    /// # Usage
    ///
    /// This function is intended to be used internally within the Drive implementation.
    ///
    pub(in crate::drive) fn add_estimation_costs_for_levels_up_to_contract_v0(
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
