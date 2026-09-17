use crate::drive::contract_groups::estimated_costs::CONTRACT_GROUP_INFO_ESTIMATED_SIZE;
use crate::drive::contract_groups::paths::{
    contract_group_path, contract_groups_groups_path, contract_groups_root_path,
};
use crate::drive::Drive;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::{ApproximateElements, EstimatedLevel};
use grovedb::EstimatedLayerSizes::{AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{EstimatedLayerInformation, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Adds the layer information common to every write under the `ContractGroups` root tree:
    /// the GroveDB root and the root tree with its two subtrees.
    pub(in crate::drive::contract_groups) fn add_estimation_costs_for_contract_groups_root_layers(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // The GroveDB root holds every root tree; ContractGroups is one normal tree among them.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path([]),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(3, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );

        // [ContractGroups] holds exactly two subtrees: Groups and Members.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_groups_root_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
            },
        );
    }

    pub(super) fn add_estimation_costs_for_insert_contract_group_v0(
        contract_group_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        Self::add_estimation_costs_for_contract_groups_root_layers(
            estimated_costs_only_with_layer_info,
        );

        // [ContractGroups, Groups] holds one subtree per contract group, keyed by id.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_groups_groups_path()),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(10, false),
                estimated_layer_sizes: AllSubtrees(DEFAULT_HASH_SIZE_U8, NoSumTrees, None),
            },
        );

        // The group's own tree: the info item and three member subtrees.
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_group_path(&contract_group_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: ApproximateElements(4),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((1, NoSumTrees, None, 3)),
                    items_size: Some((1, CONTRACT_GROUP_INFO_ESTIMATED_SIZE, None, 1)),
                    references_size: None,
                    items_with_sum_item_size: None,
                    references_with_sum_item_size: None,
                },
            },
        );
    }
}
