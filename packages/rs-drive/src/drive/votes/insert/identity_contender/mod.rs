mod add_identity_contender_operations;
mod insert_stored_info_for_identity_contender_vote_poll;
mod open_identity_contender_vote_poll_operations;
mod register_identity_contender_vote_poll_identity_vote;

use crate::drive::votes::paths::{
    vote_identity_contender_active_polls_tree_path_vec,
    vote_identity_contender_polls_tree_path_vec, vote_root_path_vec,
};
use crate::util::type_constants::{DEFAULT_HASH_SIZE_U8, U8_SIZE_U8};
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::EstimatedSumTrees::{NoSumTrees, SomeSumTrees};
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

/// Declares the layers above a poll's own tree for a cost estimation: the GroveDB root, the
/// votes tree, the identity contender branch and its active polls tree. Every batch that
/// writes under a poll needs them, whether it opens the poll or adds a contender to it later,
/// since the estimation cannot read them from state.
pub(in crate::drive::votes) fn add_estimation_costs_for_identity_contender_polls_tree_levels(
    estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
) {
    // Votes sit on the second level of the root tree, next to the balances sum tree
    estimated_costs_only_with_layer_info.insert(
        KeyInfoPath::from_known_path([]),
        EstimatedLayerInformation {
            tree_type: TreeType::NormalTree,
            estimated_layer_count: EstimatedLevel(2, false),
            estimated_layer_sizes: AllSubtrees(
                1,
                SomeSumTrees {
                    sum_trees_weight: 1,
                    big_sum_trees_weight: 0,
                    count_trees_weight: 0,
                    count_sum_trees_weight: 0,
                    non_sum_trees_weight: 2,
                    provable_sum_trees_weight: 0,
                    provable_count_trees_weight: 0,
                    provable_count_sum_trees_weight: 0,
                    provable_count_provable_sum_trees_weight: 0,
                },
                None,
            ),
        },
    );
    // The branch and its two trees, in case this is the first poll
    estimated_costs_only_with_layer_info.insert(
        KeyInfoPath::from_known_owned_path(vote_root_path_vec()),
        EstimatedLayerInformation {
            tree_type: TreeType::NormalTree,
            estimated_layer_count: ApproximateElements(4),
            estimated_layer_sizes: AllSubtrees(U8_SIZE_U8, NoSumTrees, None),
        },
    );
    estimated_costs_only_with_layer_info.insert(
        KeyInfoPath::from_known_owned_path(vote_identity_contender_polls_tree_path_vec()),
        EstimatedLayerInformation {
            tree_type: TreeType::NormalTree,
            estimated_layer_count: ApproximateElements(2),
            estimated_layer_sizes: AllSubtrees(U8_SIZE_U8, NoSumTrees, None),
        },
    );
    estimated_costs_only_with_layer_info.insert(
        KeyInfoPath::from_known_owned_path(vote_identity_contender_active_polls_tree_path_vec()),
        EstimatedLayerInformation {
            tree_type: TreeType::NormalTree,
            estimated_layer_count: PotentiallyAtMaxElements,
            estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
        },
    );
}
