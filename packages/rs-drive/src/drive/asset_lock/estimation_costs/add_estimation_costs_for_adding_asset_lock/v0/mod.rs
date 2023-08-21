use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};

use crate::drive::asset_lock::asset_lock_storage_path;
use grovedb::EstimatedSumTrees::SomeSumTrees;
use std::collections::HashMap;

impl Drive {
    pub(crate) fn add_estimation_costs_for_adding_asset_lock_v0(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        //todo: verify (this is wrong)
        // we have constructed the top layer so contract/documents tree are at the top
        // since balance will be on layer 2, updating will mean we will update 1 sum tree
        // and 1 normal tree, hence we should give an equal weight to both
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        non_sum_trees_weight: 1,
                    },
                    None,
                ),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(asset_lock_storage_path()),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(
                    36, //The size of an outpoint
                    0, None,
                ),
            },
        );
    }
}
