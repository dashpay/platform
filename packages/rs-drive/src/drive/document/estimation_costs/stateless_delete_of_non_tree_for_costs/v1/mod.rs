use crate::util::estimated_layers::EstimatedLayers;
use crate::util::grove_operations::BatchDeleteUpTreeApplyType;

use crate::drive::Drive;
use crate::error::Error;

use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerSizes, MaybeTree};

impl Drive {
    /// Attempts a stateless deletion of non-tree elements for costs estimation.
    ///
    /// v1 (protocol version 15) gives height `h` the description of the path's first `h + 1`
    /// keys: GroveDB's delete-up-tree estimate prices the deletion at height `h` in the layer
    /// at that path, the deleted key's own layer at the top height. v0 gives each height the
    /// layer above it, so each level was priced with its parent's tree type and sizes.
    #[inline(always)]
    pub(super) fn stateless_delete_of_non_tree_for_costs_v1(
        element_estimated_sizes: EstimatedLayerSizes,
        key_info_path: &KeyInfoPath,
        is_known_to_be_subtree_with_sum: Option<MaybeTree>,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
    ) -> Result<BatchDeleteUpTreeApplyType, Error> {
        Self::stateless_delete_of_non_tree_with_layer_paths(
            element_estimated_sizes,
            key_info_path,
            is_known_to_be_subtree_with_sum,
            estimated_costs_only_with_layer_info,
            |height| height as usize + 1,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::estimated_layers::EstimatedLayers;
    use crate::util::grove_operations::BatchDeleteUpTreeApplyType;
    use grovedb::batch::KeyInfoPath;
    use grovedb::EstimatedLayerCount::EstimatedLevel;
    use grovedb::EstimatedLayerInformation;
    use grovedb::EstimatedLayerSizes::{AllItems, AllSubtrees};
    use grovedb::EstimatedSumTrees::NoSumTrees;
    use grovedb::{MaybeTree, TreeType};

    /// The layer described at `level`, told apart by its level.
    fn layer(level: u32) -> EstimatedLayerInformation {
        EstimatedLayerInformation {
            tree_type: TreeType::NormalTree,
            estimated_layer_count: EstimatedLevel(level, false),
            estimated_layer_sizes: AllSubtrees(1, NoSumTrees, None),
        }
    }

    /// The layers a delete under `[0, 1, 2, 3, 4, 5]` is priced with, by height, when the
    /// estimate describes the path's first `n` keys at level `n`.
    fn priced_levels(v1: bool) -> Vec<(u16, u32, bool)> {
        let path: Vec<Vec<u8>> = (0u8..6).map(|key| vec![key]).collect();
        let mut estimate = EstimatedLayers::new();
        for length in 4..=6 {
            estimate.insert(
                KeyInfoPath::from_known_owned_path(path[..length].to_vec()),
                layer(length as u32),
            );
        }
        let key_info_path = KeyInfoPath::from_known_owned_path(path);
        let deleted = AllItems(1, 1, None);
        let mut estimate = Some(estimate);
        let apply_type = if v1 {
            Drive::stateless_delete_of_non_tree_for_costs_v1(
                deleted,
                &key_info_path,
                Some(MaybeTree::NotTree),
                &mut estimate,
            )
        } else {
            Drive::stateless_delete_of_non_tree_for_costs_v0(
                deleted,
                &key_info_path,
                Some(MaybeTree::NotTree),
                &mut estimate,
            )
        }
        .expect("expected a stateless delete");
        let BatchDeleteUpTreeApplyType::StatelessBatchDelete {
            estimated_layer_info,
        } = apply_type
        else {
            panic!("expected a stateless delete");
        };
        let mut levels: Vec<(u16, u32, bool)> = estimated_layer_info
            .iter()
            .map(|(height, layer)| {
                (
                    height,
                    layer.estimated_layer_count.estimate_levels(),
                    layer.estimated_layer_sizes == deleted,
                )
            })
            .collect();
        levels.sort_unstable();
        levels
    }

    /// GroveDB prices the deletion at height `h` in the layer at the path's first `h + 1` keys,
    /// the deleted key's own layer at the top height, whose sizes are the deleted element's.
    /// v0 gives each height the layer above it.
    #[test]
    fn should_give_each_height_the_layer_grovedb_deletes_in() {
        assert_eq!(priced_levels(true), vec![(4, 5, false), (5, 6, true)]);
        assert_eq!(priced_levels(false), vec![(4, 4, false), (5, 5, true)]);
    }
}
