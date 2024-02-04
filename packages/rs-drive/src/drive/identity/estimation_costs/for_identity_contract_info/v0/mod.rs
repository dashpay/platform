use crate::drive::defaults::DEFAULT_HASH_SIZE_U8;
use crate::drive::identity::{identity_contract_info_root_path_vec, identity_path_vec};
use crate::drive::{identity_tree_path, Drive};
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_estimation_costs_for_contract_info_v0(
        identity_id: &[u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we have constructed the top layer so contract/documents tree are at the top
        // since balance will be on layer 2, updating will mean we will update 1 sum tree
        // and 1 normal tree, hence we should give an equal weight to both
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // we then need to insert the root identity layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(identity_tree_path()),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        // we then need to insert the identity layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_path_vec(identity_id.as_slice())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(1, false),
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );
        // we then need to insert for the identity contract info
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_contract_info_root_path_vec(identity_id)),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );
    }
}
