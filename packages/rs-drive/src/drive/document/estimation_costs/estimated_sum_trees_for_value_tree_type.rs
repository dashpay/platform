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

use crate::drive::document::ranked_index_tree_type::non_indexed_mirror_tree_type;
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
        // Indexed trees (grovedb PR 657) — the ranked variants. grovedb's
        // `EstimatedSumTrees` has no indexed weight slots, so each indexed
        // child is estimated as its non-indexed mirror. That is exact for the
        // per-node cost (an indexed primary reuses the node shape, and so the
        // aggregate byte cost, of the tree it mirrors) but a KNOWN
        // UNDERESTIMATE of the element cost: the indexed *element* carries one
        // length byte plus 33 bytes per axis (a 1-byte tag plus a 32-byte
        // secondary root key) that its mirror does not, i.e. up to
        // 1 + 33*3 = 100 bytes for a three-axis PCPSIT. The layer's
        // `EstimatedLayerSizes` byte budget is what would have to grow to
        // capture that, not these weights — recorded here so the gap is
        // attributable rather than silently absorbed into a fudged weight.
        //
        // Reachability: a value tree is never indexed (only the terminal
        // property-name tree is), so these arms fire only where an indexed
        // tree is described as a *child* of another layer — the doctype layer
        // during contract insertion.
        TreeType::ProvableCountIndexedTree
        | TreeType::ProvableSumIndexedTree
        | TreeType::ProvableCountProvableSumIndexedTree => {
            estimated_sum_trees_for_value_tree_type(non_indexed_mirror_tree_type(value_tree_type))
        }
        // Defensive: any future TreeType not handled here falls back
        // to `NoSumTrees`. Existing variants (NormalTree through the
        // indexed trio) are covered above; this arm only triggers if a new
        // variant is added without updating this helper.
        _ => NoSumTrees,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each indexed variant must land in its mirror's weight slot — and, more
    /// importantly, must NOT fall through to the `NoSumTrees` default, which
    /// is what the helper did before the ranked feature made these variants
    /// reachable and would have under-charged every write touching one.
    #[test]
    fn indexed_tree_weights_land_in_their_mirrors_slot() {
        let expect_slot = |tree_type: TreeType, extract: fn(&EstimatedSumTrees) -> u8| {
            let weights = estimated_sum_trees_for_value_tree_type(tree_type);
            assert_ne!(
                weights, NoSumTrees,
                "{tree_type:?} must not fall through to the NoSumTrees default"
            );
            assert_eq!(extract(&weights), 1, "{tree_type:?} landed in no slot");
        };

        expect_slot(TreeType::ProvableCountIndexedTree, |w| match w {
            SomeSumTrees {
                provable_count_trees_weight,
                ..
            } => *provable_count_trees_weight,
            _ => 0,
        });
        expect_slot(TreeType::ProvableSumIndexedTree, |w| match w {
            SomeSumTrees {
                provable_sum_trees_weight,
                ..
            } => *provable_sum_trees_weight,
            _ => 0,
        });
        expect_slot(TreeType::ProvableCountProvableSumIndexedTree, |w| match w {
            SomeSumTrees {
                provable_count_provable_sum_trees_weight,
                ..
            } => *provable_count_provable_sum_trees_weight,
            _ => 0,
        });
    }
}
