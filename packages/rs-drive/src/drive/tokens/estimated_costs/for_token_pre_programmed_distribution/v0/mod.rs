use crate::drive::tokens::paths::{
    token_distributions_root_path_vec, token_pre_programmed_at_time_distribution_path_vec,
    token_pre_programmed_distributions_path_vec, token_root_pre_programmed_distributions_path_vec,
};
use crate::drive::Drive;
use crate::util::type_constants::{DEFAULT_HASH_SIZE_U8, U64_SIZE_U32, U64_SIZE_U8, U8_SIZE_U8};
use dpp::prelude::TimestampMillis;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel};
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
use grovedb::EstimatedSumTrees::{AllSumTrees, NoSumTrees};
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Version 0 of the estimation function for pre-programmed distributions.
    ///
    /// This function adds estimation cost entries for:
    ///   1. The root pre-programmed distributions tree.
    ///   2. The token-specific subtree (using `token_id`).
    ///   3. Each time-specific sum tree for every timestamp in `times`.
    ///
    /// # Parameters
    /// - `token_id`: The identifier for the token.
    /// - `times`: A vector of timestamps (in milliseconds) for which pre-programmed distributions exist.
    /// - `estimated_costs_only_with_layer_info`: A mutable hashmap that holds estimated layer information.
    pub(crate) fn add_estimation_costs_for_token_pre_programmed_distribution_v0<'a, I>(
        token_id: [u8; 32],
        times: Option<I>,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) where
        I: IntoIterator<Item = &'a TimestampMillis> + ExactSizeIterator,
    {
        // 1. Add estimation for the root distributions tree.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(token_distributions_root_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(1, false), // We should be on the first level
                estimated_layer_sizes: AllSubtrees(U8_SIZE_U8, NoSumTrees, None),
            },
        );

        // 2. Add estimation for the root pre-programmed distributions tree.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(token_root_pre_programmed_distributions_path_vec()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false), // Just an estimate
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        // 3. Add estimation for the token-specific pre-programmed distributions subtree.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(token_pre_programmed_distributions_path_vec(
                token_id,
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                // At this level, expect as many children as there are time entries.
                estimated_layer_count: ApproximateElements(
                    times.as_ref().map(|times| times.len()).unwrap_or(128) as u32,
                ),
                estimated_layer_sizes: AllSubtrees(U64_SIZE_U8, AllSumTrees, None),
            },
        );

        if let Some(times) = times {
            // 4. For each provided timestamp, add an estimation for the at-time sum tree.
            for time in times {
                estimated_costs_only_with_layer_info.insert(
                    KeyInfoPath::from_known_owned_path(
                        token_pre_programmed_at_time_distribution_path_vec(token_id, *time),
                    ),
                    EstimatedLayerInformation {
                        tree_type: TreeType::SumTree,
                        estimated_layer_count: EstimatedLevel(3, false), // probably not that many
                        estimated_layer_sizes: AllItems(DEFAULT_HASH_SIZE_U8, U64_SIZE_U32, None),
                    },
                );
            }
        }
    }
}
