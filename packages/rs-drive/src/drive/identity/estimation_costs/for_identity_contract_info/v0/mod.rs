use crate::drive::defaults::{AVERAGE_BALANCE_SIZE, DEFAULT_HASH_SIZE_U8};

use crate::drive::{identity_tree_path, Drive};

use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllItems, AllReference, AllSubtrees};

use crate::drive::identity::estimation_costs::KEY_REFERENCE_SIZE;
use crate::drive::identity::{
    identity_contract_info_group_path_vec, identity_contract_info_root_path,
    identity_contract_info_root_path_vec,
};
use grovedb::EstimatedSumTrees::{NoSumTrees, SomeSumTrees};
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_estimation_costs_for_contract_info_v0(
        identity_id: &[u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we then need to insert for the identity contract info
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_contract_info_root_path_vec(identity_id)),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );
    }
}
