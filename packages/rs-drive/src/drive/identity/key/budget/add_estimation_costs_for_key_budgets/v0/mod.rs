use crate::drive::identity::key::budget::{identity_key_budgets_path_vec, KEY_BUDGET_SIZE};
use crate::drive::Drive;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::ApproximateElements;
use grovedb::EstimatedLayerSizes::AllItems;
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn add_estimation_costs_for_key_budgets_v0(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_owned_path(identity_key_budgets_path_vec(
                identity_id.as_slice(),
            )),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                // Budgeted keys are handed to applications; a handful per identity is typical.
                estimated_layer_count: ApproximateElements(8),
                estimated_layer_sizes: AllItems(1, KEY_BUDGET_SIZE, None),
            },
        );
    }
}
