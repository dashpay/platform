//! Helper: derive [`EstimatedSumTrees`] from a single child
//! [`TreeType`] for `EstimatedLayerSizes::AllSubtrees(...)` layers.
//!
//! Used at the property-name level in the index walker: every child
//! at that layer is a value tree whose type is determined by the
//! sub-level's `summable` / `range_summable` / `countable` /
//! `range_countable` flags. The estimation needs to tell grovedb
//! which aggregate-bearing variant the children carry so the
//! per-node cost on each child accounts for the right element shape
//! (count node has different bytes than sum node, etc.).
//!
//! Pre-v3 contracts only ever produce `NormalTree`-or-count value
//! tree types at this layer (the sum flags didn't exist), so the
//! v0-pinned `NoSumTrees` was correct then — but v0's output is
//! consensus-locked and can't be changed for pre-v12 contracts. This
//! helper is called from the **v1** estimators which dispatch from
//! v12+ drive-version tables.

use grovedb::EstimatedSumTrees::{self, NoSumTrees, SomeSumTrees};
use grovedb::TreeType;

/// All children of an `AllSubtrees(...)` layer share the same
/// `TreeType` (`value_tree_type`). Map that into a homogeneous
/// `EstimatedSumTrees` shortcut — one weight = 1 in the slot
/// matching the variant, every other slot = 0. Returns `NoSumTrees`
/// for `NormalTree` (no aggregation), the appropriate shortcut /
/// breakdown for everything else.
pub(crate) fn estimated_sum_trees_for_value_tree_type(
    value_tree_type: TreeType,
) -> EstimatedSumTrees {
    match value_tree_type {
        TreeType::NormalTree => NoSumTrees,
        TreeType::SumTree => SomeSumTrees {
            sum_trees_weight: 1,
            big_sum_trees_weight: 0,
            count_trees_weight: 0,
            count_sum_trees_weight: 0,
            non_sum_trees_weight: 0,
            provable_sum_trees_weight: 0,
            provable_count_trees_weight: 0,
            provable_count_sum_trees_weight: 0,
            provable_count_provable_sum_trees_weight: 0,
        },
        TreeType::BigSumTree => SomeSumTrees {
            sum_trees_weight: 0,
            big_sum_trees_weight: 1,
            count_trees_weight: 0,
            count_sum_trees_weight: 0,
            non_sum_trees_weight: 0,
            provable_sum_trees_weight: 0,
            provable_count_trees_weight: 0,
            provable_count_sum_trees_weight: 0,
            provable_count_provable_sum_trees_weight: 0,
        },
        TreeType::CountTree => SomeSumTrees {
            sum_trees_weight: 0,
            big_sum_trees_weight: 0,
            count_trees_weight: 1,
            count_sum_trees_weight: 0,
            non_sum_trees_weight: 0,
            provable_sum_trees_weight: 0,
            provable_count_trees_weight: 0,
            provable_count_sum_trees_weight: 0,
            provable_count_provable_sum_trees_weight: 0,
        },
        TreeType::ProvableCountTree => SomeSumTrees {
            sum_trees_weight: 0,
            big_sum_trees_weight: 0,
            count_trees_weight: 0,
            count_sum_trees_weight: 0,
            non_sum_trees_weight: 0,
            provable_sum_trees_weight: 0,
            provable_count_trees_weight: 1,
            provable_count_sum_trees_weight: 0,
            provable_count_provable_sum_trees_weight: 0,
        },
        TreeType::ProvableSumTree => SomeSumTrees {
            sum_trees_weight: 0,
            big_sum_trees_weight: 0,
            count_trees_weight: 0,
            count_sum_trees_weight: 0,
            non_sum_trees_weight: 0,
            provable_sum_trees_weight: 1,
            provable_count_trees_weight: 0,
            provable_count_sum_trees_weight: 0,
            provable_count_provable_sum_trees_weight: 0,
        },
        TreeType::CountSumTree => SomeSumTrees {
            sum_trees_weight: 0,
            big_sum_trees_weight: 0,
            count_trees_weight: 0,
            count_sum_trees_weight: 1,
            non_sum_trees_weight: 0,
            provable_sum_trees_weight: 0,
            provable_count_trees_weight: 0,
            provable_count_sum_trees_weight: 0,
            provable_count_provable_sum_trees_weight: 0,
        },
        TreeType::ProvableCountSumTree => SomeSumTrees {
            sum_trees_weight: 0,
            big_sum_trees_weight: 0,
            count_trees_weight: 0,
            count_sum_trees_weight: 0,
            non_sum_trees_weight: 0,
            provable_sum_trees_weight: 0,
            provable_count_trees_weight: 0,
            provable_count_sum_trees_weight: 1,
            provable_count_provable_sum_trees_weight: 0,
        },
        TreeType::ProvableCountProvableSumTree => SomeSumTrees {
            sum_trees_weight: 0,
            big_sum_trees_weight: 0,
            count_trees_weight: 0,
            count_sum_trees_weight: 0,
            non_sum_trees_weight: 0,
            provable_sum_trees_weight: 0,
            provable_count_trees_weight: 0,
            provable_count_sum_trees_weight: 0,
            provable_count_provable_sum_trees_weight: 1,
        },
        // Defensive: any future TreeType not handled here falls back
        // to `NoSumTrees`. Existing variants (NormalTree through PCPS)
        // are covered above; this arm only triggers if a new variant
        // is added without updating this helper.
        _ => NoSumTrees,
    }
}
