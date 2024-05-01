use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::ApproximateElements;
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllReference, AllSubtrees};

use crate::drive::identity::identity_query_keys_purpose_tree_path_vec;

use crate::drive::identity::estimation_costs::KEY_REFERENCE_SIZE;
use dpp::identity::Purpose;
use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

impl Drive {
    /// Adds estimation costs for a given purpose in key reference tree for a given identity id.
    ///
    /// This method operates on the provided HashMap, `estimated_costs_only_with_layer_info`, and adds
    /// new entries to it, representing the estimated costs for different layers of the identity tree related to the specified identity id and purpose.
    ///
    /// # Parameters
    /// - `identity_id`: An array of 32 bytes representing the unique identity id.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a HashMap storing the `KeyInfoPath` and `EstimatedLayerInformation`.
    /// - `purpose`: A Purpose enum representing the purpose of the identity key.
    ///
    /// # Returns
    /// - `Ok(())` if successful.
    /// - `Err(DriveError::UnknownVersionMismatch)` if the method version doesn't match any known versions.
    ///
    /// # Errors
    /// This function will return an error if the method version doesn't match any known versions.
    pub(super) fn add_estimation_costs_for_purpose_in_key_reference_tree_v0(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        purpose: Purpose,
    ) {
        let estimated_layer_count = match purpose {
            Purpose::AUTHENTICATION => ApproximateElements(4),
            Purpose::ENCRYPTION => {
                return;
            }
            Purpose::DECRYPTION => {
                return;
            }
            Purpose::TRANSFER => ApproximateElements(1),
            Purpose::SYSTEM => ApproximateElements(1),
            Purpose::VOTING => ApproximateElements(1),
        };

        let estimated_layer_sizes = match purpose {
            Purpose::AUTHENTICATION => AllSubtrees(1, NoSumTrees, None),
            Purpose::ENCRYPTION => {
                return;
            }
            Purpose::DECRYPTION => {
                return;
            }
            Purpose::TRANSFER => AllReference(1, KEY_REFERENCE_SIZE, None),
            Purpose::SYSTEM => AllReference(1, KEY_REFERENCE_SIZE, None),
            Purpose::VOTING => AllReference(1, KEY_REFERENCE_SIZE, None),
        };
        // we then need to insert the identity keys layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_query_keys_purpose_tree_path_vec(
                identity_id.as_slice(),
                purpose,
            )),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count, // there are
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes,
            },
        );
    }
}
