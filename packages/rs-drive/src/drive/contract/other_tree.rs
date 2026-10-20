//! Cost estimation for a contract's other tree (`[64, id, 2]`): the version item and the
//! moderation list trees. Server only; the paths and the version item's codec are shared with
//! the verifier.

use crate::drive::contract::paths::{contract_other_path, contract_root_path};
use crate::drive::contract::version_item::CONTRACT_VERSION_ITEM_SIZE;
use crate::drive::Drive;
use crate::util::storage_flags::StorageFlags;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerCount::EstimatedLevel;
use grovedb::EstimatedLayerInformation;
use grovedb::EstimatedLayerSizes::{AllSubtrees, Mix};
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::TreeType;
use std::collections::HashMap;

impl Drive {
    /// Adds the layer information of a contract's other tree (`[64, id, 2]`: the version item
    /// and up to three moderation list trees), and of the contract's root subtree above it when
    /// nothing in the batch described that one yet. The root subtree's entry is left alone
    /// when it is there: a document or contract operation of the same batch describes it
    /// better than this does.
    pub(crate) fn add_estimation_costs_for_contract_other_tree(
        contract_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        let flags_size = Some(StorageFlags::approximate_size(true, None));

        estimated_costs_only_with_layer_info
            .entry(KeyInfoPath::from_known_path(contract_root_path(
                &contract_id,
            )))
            .or_insert(EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: AllSubtrees(1, NoSumTrees, flags_size),
            });

        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(contract_other_path(&contract_id)),
            EstimatedLayerInformation {
                tree_type: TreeType::NormalTree,
                estimated_layer_count: EstimatedLevel(1, false),
                estimated_layer_sizes: Mix {
                    subtrees_size: Some((1, NoSumTrees, flags_size, 3)),
                    items_size: Some((1, CONTRACT_VERSION_ITEM_SIZE as u32, flags_size, 1)),
                    items_with_sum_item_size: None,
                    references_size: None,
                    references_with_sum_item_size: None,
                },
            },
        );
    }
}
