//! The generation-3 [`super::common::RankedIndexStructureCheck`]: the cross-index
//! structural rule the ranked grammar adds on top of the shared parse core.
//!
//! Its own file for the same reason the mode-detection versions have their
//! own files: the rule is one frozen unit of generation-3 grammar, and a
//! later generation that needs a different structural rule supplies its own
//! callback instead of editing this one.

use crate::data_contract::document_type::class_methods::consensus_or_protocol_data_contract_error;
use crate::data_contract::document_type::index::Index;
use crate::data_contract::errors::DataContractError;
use crate::ProtocolError;
use std::collections::BTreeMap;

/// Rejects the compound-ranked shapes the storage layer cannot lay out:
/// a compound ranked index whose **full leading prefix** also terminates
/// a separate countable and/or summable index, and — for prefix-level
/// rankings (`rankedCountable: { at }`) — any other index reaching the
/// ranked level or below, plus the same aggregating-prefix conflict one
/// level above the ranked level (see the second loop below).
///
/// A ranked flag on a compound index `[p1, …, pn]` puts an indexed tree
/// at each prefix's terminal `pn` property-name level — inside the value
/// trees of the `[p1, …, pn-1]` level. When another countable/summable
/// index terminates at exactly that prefix, those value trees are
/// aggregating (`CountTree` / `SumTree` / …), and every continuation
/// subtree inside them must be wrapped in a `NonCounted` / `NotSummed`
/// shell so its contents don't pollute the prefix index's aggregates.
/// grovedb structurally rejects that shell around an indexed tree — the
/// wrapper would neutralize the very aggregates the ranking indexes —
/// so the write path fails closed at document insert. Rejecting the
/// contract here surfaces the conflict at registration instead.
///
/// Only the **exact** `n-1` prefix conflicts. An aggregating index
/// terminating at a shorter prefix wraps a plain intermediate
/// property-name tree (fine), and one extending *past* the ranked
/// terminal lives inside the indexed tree's value trees, which the
/// storage layer supports (see rs-drive's
/// `ranked_terminator_with_a_compound_continuation_gets_both_treatments`).
///
/// Property comparison is by name, positionally: the merged index-level
/// tree keys sub-levels by property name in declaration order, so
/// `[a, b]` and `[b, a]` never share a level and cannot conflict.
///
/// Unconditional (not gated on `full_validation`): the same structural
/// impossibility must reject the contract on every parse path — a
/// contract admitted through a non-validating parse would brick the
/// first document insert under the ranked index.
pub(super) fn validate_no_ranked_prefix_overlap(
    indices: &BTreeMap<String, Index>,
) -> Result<(), ProtocolError> {
    for ranked in indices.values() {
        let is_ranked =
            ranked.ranked_countable || ranked.ranked_summable || ranked.ranked_averageable;
        if !is_ranked || ranked.properties.len() < 2 {
            continue;
        }
        let prefix = &ranked.properties[..ranked.properties.len() - 1];
        for other in indices.values() {
            if other.name == ranked.name {
                continue;
            }
            let terminates_at_prefix = other.properties.len() == prefix.len()
                && other
                    .properties
                    .iter()
                    .zip(prefix.iter())
                    .all(|(a, b)| a.name == b.name);
            let aggregates = other.countable.is_countable() || other.summable.is_some();
            if terminates_at_prefix && aggregates {
                return Err(consensus_or_protocol_data_contract_error(
                    DataContractError::InvalidContractStructure(format!(
                        "compound ranked index `{}` conflicts with index `{}`: the ranked \
                         index's leading prefix [{}] also terminates a countable/summable \
                         index, so the ranked terminal tree would sit inside aggregating \
                         value trees and need a NonCounted/NotSummed shell — which the \
                         storage layer rejects for indexed trees because the wrapper would \
                         neutralize the aggregates the ranking indexes. Drop the ranked \
                         flags, or drop the aggregate flags from the prefix index",
                        ranked.name,
                        other.name,
                        prefix
                            .iter()
                            .map(|p| p.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    )),
                ));
            }
        }
    }

    // Prefix-level rankings (`rankedCountable: { at }`) add two rules of
    // their own, both keyed off the declared `at` level (position `p`):
    //
    // 1. **Exclusivity of the `at` level and everything below.** From `p`
    //    down, the ranked index's levels are one count-propagation chain:
    //    the `at` property-name tree is the indexed tree ranking by
    //    whole-subtree count, and its value trees count exactly one
    //    continuation each. Any other index reaching the `at` level —
    //    terminating there (its member bucket and aggregates would collide
    //    with the grouping tree) or continuing below it (a second
    //    continuation whose members would pollute every subtree total) —
    //    has no coherent layout, whatever its flags.
    //
    // 2. **The same wrapped-indexed impossibility as above, one level up.**
    //    The grouping tree sits inside the value trees of level `p - 1`
    //    (when `p >= 1`); a countable/summable index terminating at exactly
    //    `[p1, …, pp]` makes those value trees aggregating, demanding the
    //    NonCounted/NotSummed shell the storage layer rejects for indexed
    //    trees.
    //
    // Same name-positional comparison and same unconditional (not
    // `full_validation`-gated) reasoning as the terminal rule above.
    for ranked in indices.values() {
        // Key both rules off the SHALLOWEST ranked prefix level: every
        // deeper ranked level sits inside its exclusive range, so nothing
        // else can conflict with them once this one is protected. The
        // parser guarantees each `at` name resolves to a non-terminal
        // property; stay defensive for `Index` values built outside it.
        let Some(at_position) = ranked
            .ranked_countable_at
            .iter()
            .filter_map(|at| ranked.properties.iter().position(|p| &p.name == at))
            .min()
        else {
            continue;
        };
        let at = ranked.properties[at_position].name.as_str();
        for other in indices.values() {
            if other.name == ranked.name {
                continue;
            }
            let shares_at_level = other.properties.len() > at_position
                && other
                    .properties
                    .iter()
                    .zip(ranked.properties.iter())
                    .take(at_position + 1)
                    .all(|(a, b)| a.name == b.name);
            if shares_at_level {
                return Err(consensus_or_protocol_data_contract_error(
                    DataContractError::InvalidContractStructure(format!(
                        "prefix-ranked index `{}` conflicts with index `{}`: the levels from \
                         its rankedCountable.at property (\"{}\") down form the ranking's \
                         count-propagation chain and must belong to it exclusively, but the \
                         other index shares the [{}] level. Diverge the other index before \
                         the `at` property, or move the ranking",
                        ranked.name,
                        other.name,
                        at,
                        ranked.properties[..=at_position]
                            .iter()
                            .map(|p| p.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    )),
                ));
            }
            if at_position >= 1 {
                let prefix = &ranked.properties[..at_position];
                let terminates_at_prefix = other.properties.len() == prefix.len()
                    && other
                        .properties
                        .iter()
                        .zip(prefix.iter())
                        .all(|(a, b)| a.name == b.name);
                let aggregates = other.countable.is_countable() || other.summable.is_some();
                if terminates_at_prefix && aggregates {
                    return Err(consensus_or_protocol_data_contract_error(
                        DataContractError::InvalidContractStructure(format!(
                            "prefix-ranked index `{}` conflicts with index `{}`: the prefix \
                             [{}] above its rankedCountable.at property also terminates a \
                             countable/summable index, so the grouping tree would sit inside \
                             aggregating value trees and need a NonCounted/NotSummed shell — \
                             which the storage layer rejects for indexed trees. Drop the \
                             ranking, or drop the aggregate flags from the prefix index",
                            ranked.name,
                            other.name,
                            prefix
                                .iter()
                                .map(|p| p.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", "),
                        )),
                    ));
                }
            }
        }
    }
    Ok(())
}
