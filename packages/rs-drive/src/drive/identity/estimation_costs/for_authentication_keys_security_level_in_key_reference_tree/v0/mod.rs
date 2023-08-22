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
