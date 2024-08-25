use crate::drive::identity::{identity_contract_info_root_path_vec, identity_path_vec};
use crate::drive::{identity_tree_path, Drive};
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllSubtrees, Mix};
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

        // In this layer we have
        //                              Keys
        //              /                               \
        //           Nonce                             Revision
        //    /              \                          /
        // DataContract Info   Negative Credit      Query Keys

        // we then need to insert the identity layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_path_vec(identity_id.as_slice())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((1, NoSumTrees, None, 2)), // weight of 2 because 1 for keys and 1 for data contract info
                    items_size: Some((1, 8, None, 1)),
                    references_size: None,
                },
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
