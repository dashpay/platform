use crate::drive::{unique_key_hashes_tree_path_vec, Drive};

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::PotentiallyAtMaxElements;
use grovedb::EstimatedLayerSizes::AllItems;

use crate::util::type_constants::{DEFAULT_HASH_160_SIZE_U8, DEFAULT_HASH_SIZE_U32};
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds the estimation costs for the insertion of a unique public key hash reference
    pub(super) fn add_estimation_costs_for_insert_unique_public_key_hash_reference_v0(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        let unique_key_hashes_path = unique_key_hashes_tree_path_vec();

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(unique_key_hashes_path),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_160_SIZE_U8,
                    DEFAULT_HASH_SIZE_U32,
                    None,
                ),
            },
        );
    }
}
