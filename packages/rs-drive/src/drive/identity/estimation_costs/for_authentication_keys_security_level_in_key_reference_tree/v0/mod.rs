use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::ApproximateElements;
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::AllItems;

use crate::drive::identity::identity_query_keys_security_level_tree_path_vec;

use crate::drive::identity::estimation_costs::KEY_REFERENCE_SIZE;
use dpp::identity::SecurityLevel;

use std::collections::HashMap;

impl Drive {
<<<<<<< HEAD
    /// Adds estimation costs for authentication keys based on the security level in 
    /// the key reference tree (version 0).
    ///
    /// This function provides a mechanism to estimate the costs of authentication keys 
    /// in the key reference tree based on a given security level.
    ///
    /// # Parameters
    ///
    /// * `identity_id`: A 32-byte array representing the identity ID.
    /// * `estimated_costs_only_with_layer_info`: A mutable reference to a `HashMap` that stores
    ///   estimated layer information based on the key information path.
    /// * `security_level`: The security level associated with the authentication keys.
    ///
    /// # Notes
    ///
    /// The function has a hardcoded estimation of `ApproximateElements(4)`, which implies 
    /// an estimation that each security level will have approximately four keys.
    /// This is a preliminary estimate and might be revisited in future versions or updates.
    ///
    /// # Example
    ///
    /// ```
    /// let drive = Drive::new(...); // Initialize Drive
    /// let identity_id = [0u8; 32]; // Replace with actual identity ID
    /// let mut estimated_info = HashMap::new(); // Estimated layer information map
    /// let security_level = SecurityLevel::...; // Define the security level
    ///
    /// drive.add_estimation_costs_for_authentication_keys_security_level_in_key_reference_tree_v0(
    ///     identity_id,
    ///     &mut estimated_info,
=======
    /// Adds estimated costs for inserting authentication keys at a specific security level in the key reference tree.
    ///
    /// This method is used for estimating the cost of storing authentication keys in the key reference tree of a specific identity.
    /// The estimation is added to a mutable hashmap for layer information.
    ///
    /// # Parameters
    ///
    /// - `identity_id`: A 32-byte array that uniquely identifies the identity for which the keys are being added.
    /// - `estimated_costs_only_with_layer_info`: Mutable reference to a hashmap that will hold layer-specific cost estimation information.
    /// - `security_level`: The security level at which the keys will be stored.
    ///
    /// # Side Effects
    ///
    /// Modifies the `estimated_costs_only_with_layer_info` hashmap by adding an entry for the specific security level and identity.
    /// The entry contains an `EstimatedLayerInformation` object with hardcoded estimated costs (needs to be revisited).
    ///
    /// # Notes
    ///
    /// - The method currently estimates that each security level will only have approximately 4 keys.
    /// - The method assumes that the layer sizes are of fixed sizes (`KEY_REFERENCE_SIZE`).
    /// - TODO: The estimation logic is hardcoded and should be revisited for more accurate estimations.
    ///
    /// # Example
    ///
    /// ```rust
    /// // Assuming all types and constants are defined
    /// let identity_id: [u8; 32] = /* ... */;
    /// let mut estimated_costs: HashMap<KeyInfoPath, EstimatedLayerInformation> = HashMap::new();
    /// let security_level: SecurityLevel = /* ... */;
    ///
    /// Drive::add_estimation_costs_for_authentication_keys_security_level_in_key_reference_tree_v0(
    ///     identity_id,
    ///     &mut estimated_costs,
>>>>>>> 6ac041d9e (feat: add docs)
    ///     security_level,
    /// );
    /// ```
    pub(super) fn add_estimation_costs_for_authentication_keys_security_level_in_key_reference_tree_v0(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        security_level: SecurityLevel,
    ) {
        // we then need to insert the identity keys layer
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_query_keys_security_level_tree_path_vec(
                identity_id.as_slice(),
                security_level,
            )),
            //todo: revisit
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: ApproximateElements(4), //we can estimate that each security level will only have 4 keys
                //We can mark these as all subtrees, because the revision will be under
                estimated_layer_sizes: AllItems(1, KEY_REFERENCE_SIZE, None),
            },
        );
    }
}
