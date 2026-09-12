use crate::drive::address_funds::estimated_costs::for_address_balance_update::v0::{
    AVERAGE_NONCE_SIZE, PLATFORM_ADDRESS_KEY_SIZE,
};
use crate::drive::constants::AVERAGE_BALANCE_SIZE;
use crate::drive::Drive;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItemsWithSumItem, AllSubtrees};
use grovedb::EstimatedSumTrees::SomeSumTrees;
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Adds estimation costs for address balance updates in version 1.
    ///
    /// Sole behavioral diff vs v0 (and the load-bearing reason for the
    /// version split): the leaves at `CLEAR_ADDRESS_POOL` are
    /// `Element::ItemWithSumItem(nonce_bytes, balance, flags)` — the
    /// `i64` sum_value adds ~10 worst-case varint bytes on top of the
    /// plain-item layout. v0 (consensus-locked to protocol v11 prod)
    /// uses `AllItems(...)` which undercharges; v1 (active at v12+)
    /// uses `AllItemsWithSumItem(...)` which is sum-aware (grovedb
    /// PR #674).
    ///
    /// Everything else — the root and AddressBalances layer estimations
    /// — is byte-identical to v0. Those layers don't carry sum-bearing
    /// leaves; they're `AllSubtrees(...)` with weighted child mixes
    /// that grovedb's `SomeSumTrees` already covers correctly.
    pub(super) fn add_estimation_costs_for_address_balance_update_v1(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // Root level estimation (identical to v0).
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 2, // AddressBalances and Pools are sum trees
                        big_sum_trees_weight: 0,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 0,
                        non_sum_trees_weight: 2, // Other root trees
                        provable_sum_trees_weight: 0,
                        provable_count_trees_weight: 0,
                        provable_count_sum_trees_weight: 0,
                        provable_count_provable_sum_trees_weight: 0,
                    },
                    None,
                ),
            },
        );

        // AddressBalances level estimation (identical to v0).
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(Self::addresses_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::SumTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(
                    1,
                    SomeSumTrees {
                        sum_trees_weight: 1,
                        big_sum_trees_weight: 0,
                        count_trees_weight: 0,
                        count_sum_trees_weight: 1,
                        non_sum_trees_weight: 0,
                        provable_sum_trees_weight: 0,
                        provable_count_trees_weight: 0,
                        provable_count_sum_trees_weight: 0,
                        provable_count_provable_sum_trees_weight: 0,
                    },
                    None,
                ),
            },
        );

        // CLEAR_ADDRESS_POOL level estimation — **diff vs v0** is on this
        // layer. Leaves are ItemWithSumItem (nonce + balance), so use
        // the sum-aware layer-size variant to include the i64 sum_value
        // in the per-element cost.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(Self::clear_addresses_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::CountSumTree,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItemsWithSumItem(
                    PLATFORM_ADDRESS_KEY_SIZE as u8,
                    AVERAGE_NONCE_SIZE + AVERAGE_BALANCE_SIZE,
                    None,
                ),
            },
        );
    }
}
