use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::ApproximateElements;
use grovedb::EstimatedLayerSizes::AllReference;
use grovedb::{EstimatedLayerInformation, TreeType};

use crate::drive::identity::estimation_costs::KEY_REFERENCE_SIZE;
use crate::drive::identity::identity_contract_info_group_path_key_purpose_vec;
use dpp::identity::Purpose;
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_estimation_costs_for_contract_info_group_key_purpose_v0(
        identity_id: &[u8; 32],
        group_id: &[u8],
        key_purpose: Purpose,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we then need to insert for the identity contract info for the contract in question
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_contract_info_group_path_key_purpose_vec(
                identity_id,
                group_id,
                key_purpose,
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(5),
                estimated_layer_sizes: AllReference(1, KEY_REFERENCE_SIZE, None),
            },
        );
    }
}
