use crate::drive::tokens::paths::{
    token_ms_timed_at_time_distributions_path_vec, token_ms_timed_distributions_path_vec,
    token_timed_distributions_path_vec,
};
use crate::drive::Drive;
use crate::util::type_constants::{
    DEFAULT_HASH_SIZE_U32, DEFAULT_HASH_SIZE_U8, U64_SIZE_U8, U8_SIZE_U8,
};
use dpp::prelude::TimestampMillis;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerSizes::{AllReference, AllSubtrees};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_estimation_costs_for_root_token_ms_interval_distribution_v0<'a, I>(
        times: I,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) where
        I: IntoIterator<Item = &'a TimestampMillis>,
    {
        // 1. Insert estimation for the generic timed distributions tree.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(token_timed_distributions_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(0, false), // 0 because ms is on top
                estimated_layer_sizes: AllSubtrees(U8_SIZE_U8, NoSumTrees, None),
            },
        );

        // 2. Insert estimation for the millisecond-timed distributions tree.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(token_ms_timed_distributions_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false), // we can have a lot of times
                estimated_layer_sizes: AllSubtrees(U64_SIZE_U8, NoSumTrees, None),
            },
        );

        // 3. For each provided timestamp, add an estimation entry for the at-time ms distribution sum tree.
        for time in times {
            let key = KeyInfoPath::from_known_owned_path(
                token_ms_timed_at_time_distributions_path_vec(*time),
            );
            estimated_costs_only_with_layer_info.insert(
                key,
                EstimatedLayerInformation {
                    tree_type: TreeType::NormalTree,
                    // We assume a shallow sum tree for the distribution entries at a given time.
                    estimated_layer_count: EstimatedLevel(1, false),
                    // Each distribution entry is estimated with a fixed size.
                    estimated_layer_sizes: AllReference(
                        DEFAULT_HASH_SIZE_U8,
                        DEFAULT_HASH_SIZE_U32 * 2 + 2,
                        None,
                    ),
                },
            );
        }
    }
}
