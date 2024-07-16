use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;

use crate::drive::{identity_tree_path, Drive};

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};

use crate::drive::identity::{identity_key_tree_path_vec, identity_path_vec};

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

impl Drive {
    /// Adds estimation costs for keys associated with a specific identity ID (version 0).
    ///
    /// This function provides a mechanism to estimate the costs of keys within the drive
    /// based on a given identity ID by updating the provided `HashMap` with layer information
    /// relevant to the keys.
    ///
    /// # Parameters
    ///
    /// * `identity_id`: A 32-byte array representing the identity ID.
    /// * `estimated_costs_only_with_layer_info`: A mutable reference to a `HashMap`
    ///   that stores estimated layer information based on the key information path.
    ///
    /// # Notes
    ///
    /// The function estimates costs for four key layers:
    ///
    /// 1. A top layer where contract/documents trees reside at the top. It's assumed
    ///    to be on layer 2, where updates might involve one sum tree and one normal tree.
    /// 2. The root identity layer.
    /// 3. The specific identity layer for the provided identity ID.
    /// 4. The identity keys layer, where it's estimated that each identity will have
    ///    approximately 50 keys.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let drive = Drive::new(...); // Initialize the drive
    /// let identity_id = [0u8; 32]; // Replace with actual identity ID
    /// let mut estimated_info = HashMap::new(); // Estimated layer information map
    ///
    /// drive.add_estimation_costs_for_keys_for_identity_id_v0(
    ///     identity_id,
    ///     &mut estimated_info,
    /// );
    /// ```
    pub(super) fn add_estimation_costs_for_keys_for_identity_id_v0(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we have constructed the top layer so contract/documents tree are at the top
        // since balance will be on layer 2, updating will mean we will update 1 sum tree
        // and 1 normal tree, hence we should give an equal weight to both
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(1, false),
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

        // we then need to insert the identity layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_path_vec(identity_id.as_slice())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: EstimatedLevel(1, false),
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // we then need to insert the identity keys layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_key_tree_path_vec(identity_id.as_slice())),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: ApproximateElements(50), //we can estimate that an identity will have amount 50 keys
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes: AllItems(1, 42, Some(3)),
            },
        );
    }
}
