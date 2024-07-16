use crate::drive::constants::ESTIMATED_NON_UNIQUE_KEY_DUPLICATES;

use crate::drive::{
    non_unique_key_hashes_sub_tree_path_vec, non_unique_key_hashes_tree_path_vec, Drive,
};

use crate::util::type_constants::{DEFAULT_HASH_160_SIZE_U8, DEFAULT_HASH_SIZE_U8};
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

impl Drive {
    /// Adds the estimation costs for the insertion of a non unique
    /// public key hash reference
    pub(super) fn add_estimation_costs_for_insert_non_unique_public_key_hash_reference_v0(
        public_key_hash: [u8; 20],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        let non_unique_key_hashes_path = non_unique_key_hashes_tree_path_vec();

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(non_unique_key_hashes_path),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_160_SIZE_U8, NoSumTrees, None),
            },
        );

        let non_unique_key_hashes_sub_path =
            non_unique_key_hashes_sub_tree_path_vec(public_key_hash);

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(non_unique_key_hashes_sub_path),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: ApproximateElements(ESTIMATED_NON_UNIQUE_KEY_DUPLICATES),
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, 0, None),
            },
        );
    }
}
