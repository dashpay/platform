use crate::drive::identity::identity_contract_info_group_path_vec;
use crate::drive::Drive;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::ApproximateElements;
use grovedb::EstimatedLayerSizes::Mix;
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_estimation_costs_for_contract_info_group_v0(
        identity_id: &[u8; 32],
        group_id: &[u8],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we then need to insert for the identity contract info for the contract in question
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_contract_info_group_path_vec(
                identity_id,
                group_id,
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(2),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((1, NoSumTrees, None, 1)),
                    items_size: Some((1, 1, None, 1)),
                    references_size: None,
                },
            },
        );
    }
}
