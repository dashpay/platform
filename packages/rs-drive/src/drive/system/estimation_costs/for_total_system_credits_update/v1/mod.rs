use crate::drive::Drive;
use crate::util::estimated_layers::EstimatedLayers;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::{AllSubtrees, Mix};
use grovedb::{EstimatedLayerInformation, TreeType};

use crate::drive::system::misc_path_vec;

use grovedb::EstimatedSumTrees::SomeSumTrees;

impl Drive {
    /// Adds estimation costs for total system credits update.
    ///
    /// v1 (protocol version 15) describes the root as v0 does, and `Misc` as it is: the documents
    /// expirations tree `E` on top, the total system credits item `D` and the token supplies big
    /// sum tree `T` below it (`layer_shapes.misc` in `grovedb-structure.json`). v0 describes
    /// `Misc` as holding the credits item alone, so a batch that also writes under `T` or `E`
    /// described the layer two ways that disagree on whether it holds subtrees.
    #[inline(always)]
    pub(super) fn add_estimation_costs_for_total_system_credits_update_v1(
        estimated_costs_only_with_layer_info: &mut EstimatedLayers,
    ) {
        // we have constructed the top layer so contract/documents tree are at the top
        // since balance will be on layer 3, updating will mean we will update 1 sum tree
        // and 2 normal trees, hence we should give an equal weight to both
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(
                    12, // about 32 + 1 + 1 / 3
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

        // `Misc`: the credits item sits below the expirations tree, beside the token supplies.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(misc_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((
                        1,
                        SomeSumTrees {
                            sum_trees_weight: 0,
                            big_sum_trees_weight: 1,
                            count_trees_weight: 0,
                            count_sum_trees_weight: 0,
                            non_sum_trees_weight: 1,
                            provable_sum_trees_weight: 0,
                            provable_count_trees_weight: 0,
                            provable_count_sum_trees_weight: 0,
                            provable_count_provable_sum_trees_weight: 0,
                        },
                        None,
                        2,
                    )),
                    items_size: Some((1, 8, None, 1)),
                    references_size: None,
                    items_with_sum_item_size: None,
                    references_with_sum_item_size: None,
                },
            },
        );
    }
}
