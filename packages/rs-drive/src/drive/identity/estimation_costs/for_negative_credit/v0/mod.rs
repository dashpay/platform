use crate::drive::defaults::DEFAULT_HASH_SIZE_U8;

use crate::drive::{identity_tree_path, Drive};

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllSubtrees, Mix};

use crate::drive::identity::identity_path_vec;

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

impl Drive {
    /// Adds estimation costs for negative credit for a given identity id for version 0.
    ///
    /// This method operates on the provided HashMap, `estimated_costs_only_with_layer_info`, and adds
    /// new entries to it, representing the estimated costs for different layers of the identity tree related to the specified identity id.
    ///
    /// # Parameters
    /// - `identity_id`: An array of 32 bytes representing the unique identity id.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a HashMap storing the `KeyInfoPath` and `EstimatedLayerInformation`.
    ///
    /// # Returns
    /// - `Ok(())` if successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the method version doesn't match any known versions.
    ///
    /// # Errors
    /// This function will return an error if the method version doesn't match any known versions.
    pub(super) fn add_estimation_costs_for_negative_credit_v0(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
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
        //                   Keys
        //              /         \
        //DataContract Info         Revision
        //       /                   /
        //  Negative Credit      Query Keys

        // we then need to insert the identity layer for fee refunds
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_path_vec(identity_id.as_slice())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(2, false),
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((1, NoSumTrees, None, 2)),
                    items_size: Some((1, 8, None, 1)),
                    references_size: None,
                },
            },
        );
    }
}
