use crate::drive::document::expiration::paths::{
    documents_expirations_at_time_path_vec, documents_expirations_path_vec,
};
use crate::drive::system::misc_path_vec;
use crate::drive::Drive;
use crate::util::type_constants::{DEFAULT_HASH_SIZE_U8, U64_SIZE_U8};
use dpp::prelude::TimestampMillis;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::{NoSumTrees, SomeSumTrees};
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    pub(super) fn add_estimation_costs_for_document_expiration_v0(
        expires_at_ms: TimestampMillis,
        entry_value_size: u32,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // The root: `Misc` sits on the fourth level of the root tree, below `Votes`, and the
        // root holds sum trees beside plain ones. This is the estimate every write under `Misc`
        // uses (`add_estimation_costs_for_total_system_credits_update`). It replaces the level 0
        // estimate a document write puts there (`DataContract_Documents` is the root's top
        // node), which only raises the batch's estimate: a document with a time to live
        // writes below both.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(
                    12,
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

        // `Misc`: a handful of single-byte keys, items and trees.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(misc_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(4),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // The expirations tree: one tree per distinct expiry time. A block every five
        // seconds expiring documents of one time to live, for a year of the longest time to
        // live, bounds it from above.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(documents_expirations_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(6_307_200),
                estimated_layer_sizes: AllSubtrees(U64_SIZE_U8, NoSumTrees, None),
            },
        );

        // The documents expiring at one time: those one block created in one document type,
        // or several types of one time to live.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(documents_expirations_at_time_path_vec(
                expires_at_ms,
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(16),
                estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, entry_value_size, None),
            },
        );
    }
}
