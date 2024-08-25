use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;

use crate::drive::{identity_tree_path, Drive};

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllSubtrees, Mix};

use crate::drive::identity::identity_path_vec;

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

impl Drive {
    /// This function adds estimation costs for an updated revision.
    ///
    /// It expects an identity in the form of an array of bytes, and a mutable reference to a HashMap containing
    /// the estimated costs with layer info. Additionally, it takes a reference to the drive version.
    ///
    /// Based on the version of the drive, it calls the appropriate function to handle cost estimation.
    ///
    /// # Parameters
    /// - `identity_id`: A 32-byte array representing the identity id.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a HashMap storing
    ///   the `KeyInfoPath` and `EstimatedLayerInformation`.
    /// - `drive_version`: A reference to the `DriveVersion`.
    ///
    /// # Returns
    /// - `Ok(())` if successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the method version doesn't match any known versions.
    ///
    /// # Errors
    /// This function will return an error if the method version doesn't match any known versions.
    pub(super) fn add_estimation_costs_for_update_nonce_v0(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we need to add the root
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(0, false),
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

        // we then need to insert the identity layer for fee refunds
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_path_vec(identity_id.as_slice())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(1, false),
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((1, NoSumTrees, None, 1)),
                    items_size: Some((1, 8, None, 1)),
                    references_size: None,
                },
            },
        );
    }
}
