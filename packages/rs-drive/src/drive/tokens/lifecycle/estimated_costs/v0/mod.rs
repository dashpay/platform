use crate::drive::tokens::lifecycle::estimated_costs::ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES;
use crate::drive::tokens::paths::{token_contract_lifecycles_root_path, tokens_root_path};
use crate::drive::Drive;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::{NoSumTrees, SomeSumTrees};
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Layer estimation for the token contract lifecycle ledger (v0).
    ///
    /// Three layers: the root tree, the tokens root with its mix of one big sum tree and
    /// normal trees, and the ledger itself, a flat map of contract id keys to bincode
    /// encoded records estimated ten levels deep. The two one-byte keys of the ledger (the
    /// destroyed supply scalar and the cleanup queue tree) are folded into the item estimate.
    pub(super) fn add_estimation_costs_for_token_contract_lifecycles_v0(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(tokens_root_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(2, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 0,
                        big_sum_trees_weight: 1,
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

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(token_contract_lifecycles_root_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false),
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    ESTIMATED_TOKEN_CONTRACT_LIFECYCLE_SIZE_BYTES,
                    None,
                ),
            },
        );
    }
}
