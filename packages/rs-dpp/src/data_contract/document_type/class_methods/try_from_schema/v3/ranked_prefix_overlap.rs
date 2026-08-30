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
/// Level comparison is positional and by **level key**
/// ([`Index::level_key`]), not by declared name: a time-range index's
/// first property is stored under its grid-qualified key, so a plain
/// index and a bucketed index with identical declared properties fork
/// into sibling subtrees and never conflict — exactly the identity the
/// index-level derivation uses. (`[a, b]` and `[b, a]` never share a
/// level either way.)
///
/// Unconditional (not gated on `full_validation`): the same structural
/// impossibility must reject the contract on every parse path — a
/// contract admitted through a non-validating parse would brick the
/// first document insert under the ranked index.
/// Whether `other`'s first `depth` levels occupy the same GroveDB
/// subtrees as `ranked`'s. Level identity is [`Index::level_key`] — the
/// grid-qualified storage key for a time-range first property, the bare
/// name otherwise — so a bucketed sibling whose declared names match a
/// plain index's does NOT share its levels. `false` when either index
/// has fewer than `depth` properties.
fn shares_leading_levels(ranked: &Index, other: &Index, depth: usize) -> bool {
    ranked.properties.len() >= depth
        && other.properties.len() >= depth
        && (0..depth).all(|position| {
            ranked.level_key(position, &ranked.properties[position].name)
                == other.level_key(position, &other.properties[position].name)
        })
}

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
                && shares_leading_levels(ranked, other, prefix.len());
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
    // 1. **Exclusivity of the `at` level and everything below — except
    //    plain continuing siblings.** From `p` down, the ranked index's
    //    levels are one count-propagation chain: the `at` property-name
    //    tree is the indexed tree ranking by whole-subtree count, and its
    //    value trees count their chain continuation. Another index reaching
    //    the `at` level is only coherent when it is PLAIN (no countable /
    //    summable / range / ranked flags, nothing contested) and CONTINUES
    //    below the `at` level: its branch trees are then wrapped
    //    `Element::NonCounted` inside the chain's value trees (the same
    //    demotion range-countable value trees apply to their sibling
    //    continuations), so they read normally and contribute zero to every
    //    subtree total. An aggregating/ranked sibling (its aggregates need
    //    the counts the wrapper suppresses) or one terminating exactly at
    //    the `at` level (member bucket + aggregates on the grouping tree
    //    itself) still has no coherent layout.
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
                && shares_leading_levels(ranked, other, at_position + 1);
            if shares_at_level {
                // One shape is admissible inside the chain's range: a PLAIN
                // sibling — no countable/summable/range/ranked flags, whose
                // semantics would all attach at-or-below the shared levels —
                // that CONTINUES below the `at` level. Its branch
                // (property-name) trees are laid out `Element::NonCounted`
                // inside the chain's value trees by the rs-drive walkers,
                // so the branch reads and proves normally while
                // contributing zero to every subtree total the ranking
                // keys on — the same demotion the range-countable value
                // trees already apply to their sibling continuations.
                // Everything else keeps failing closed:
                // - an aggregating/ranked sibling would need the very
                //   counts the wrapper suppresses;
                // - a sibling terminating EXACTLY at the `at` level would
                //   put its member bucket and aggregates on the grouping
                //   tree itself (its plain spelling is left to a possible
                //   follow-up; no known consumer needs it);
                // - a contested sibling's layout is owned by the votes
                //   subsystem and is excluded conservatively.
                let sibling_is_plain = !other.countable.is_countable()
                    && other.summable.is_none()
                    && !other.range_countable
                    && !other.range_summable
                    && !other.ranked_countable
                    && !other.ranked_summable
                    && !other.ranked_averageable
                    && other.ranked_countable_at.is_empty()
                    && other.contested_index.is_none();
                let continues_below_at = other.properties.len() > at_position + 1;
                if !(sibling_is_plain && continues_below_at) {
                    return Err(consensus_or_protocol_data_contract_error(
                        DataContractError::InvalidContractStructure(format!(
                            "prefix-ranked index `{}` conflicts with index `{}`: the levels from \
                             its rankedCountable.at property (\"{}\") down form the ranking's \
                             count-propagation chain, and the other index shares the [{}] level. \
                             Only a plain sibling (no countable/summable/ranked flags) that \
                             continues past the `at` property can be laid out count-exempt \
                             beside the ranking; diverge the other index before the `at` \
                             property, mark it plain, or move the ranking",
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
            }
            if at_position >= 1 {
                let prefix = &ranked.properties[..at_position];
                let terminates_at_prefix = other.properties.len() == prefix.len()
                    && shares_leading_levels(ranked, other, prefix.len());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::document_type::index::{IndexCountability, IndexProperty};
    use crate::data_contract::document_type::TimeRangeTransform;

    fn index(name: &str, properties: &[&str]) -> Index {
        Index {
            name: name.to_string(),
            properties: properties
                .iter()
                .map(|property| IndexProperty {
                    name: property.to_string(),
                    ascending: true,
                })
                .collect(),
            unique: false,
            null_searchable: true,
            contested_index: None,
            countable: IndexCountability::NotCountable,
            range_countable: false,
            summable: None,
            range_summable: false,
            ranked_countable: false,
            ranked_countable_at: vec![],
            ranked_summable: false,
            ranked_averageable: false,
            time_range: None,
            terminal: None,
            preallocated: false,
            skip_if_absent: false,
        }
    }

    fn count_ranked(name: &str, properties: &[&str]) -> Index {
        let mut index = index(name, properties);
        index.countable = IndexCountability::Countable;
        index.range_countable = true;
        index.ranked_countable = true;
        index
    }

    fn countable(name: &str, properties: &[&str]) -> Index {
        let mut index = index(name, properties);
        index.countable = IndexCountability::Countable;
        index
    }

    fn hourly() -> TimeRangeTransform {
        TimeRangeTransform {
            source: "$createdAt".to_string(),
            range_seconds: 3_600,
            step_seconds: 3_600,
            phase_seconds: 0,
        }
    }

    fn run(indices: Vec<Index>) -> Result<(), ProtocolError> {
        validate_no_ranked_prefix_overlap(
            &indices
                .into_iter()
                .map(|index| (index.name.clone(), index))
                .collect(),
        )
    }

    /// Level identity is the grid-qualified level key, not the declared
    /// name: a bucketed countable index whose declared property equals a
    /// terminal-ranked compound's prefix lives in a sibling subtree and
    /// must coexist, while the plain (bare-key) spelling of the same
    /// shape stays rejected.
    #[test]
    fn a_bucketed_prefix_sibling_coexists_with_a_terminal_ranking() {
        let ranked = count_ranked("byTimePost", &["$createdAt", "postId"]);
        let mut bucketed = countable("byHour", &["$createdAt"]);
        bucketed.time_range = Some(hourly());
        run(vec![ranked.clone(), bucketed])
            .expect("a grid-qualified sibling shares no level and must coexist");

        let plain = countable("byTime", &["$createdAt"]);
        assert!(
            run(vec![ranked, plain]).is_err(),
            "the bare-key spelling terminates at the real prefix level and must stay rejected"
        );
    }

    /// Same identity rule for the prefix-level exclusivity: a bucketed
    /// index reaching the `at` level's declared name forks at its
    /// qualified first key and coexists whatever its flags; the bare-key
    /// spelling of an aggregating sibling really shares the level and
    /// stays rejected. (The bare-key PLAIN continuing spelling is now the
    /// admitted count-exempt sibling — covered below.)
    #[test]
    fn a_bucketed_sibling_coexists_with_a_prefix_ranking() {
        let mut ranked = index("byTimeTagPost", &["$createdAt", "tag", "postId"]);
        ranked.countable = IndexCountability::Countable;
        ranked.range_countable = true;
        ranked.ranked_countable_at = vec!["$createdAt".to_string()];

        let mut bucketed = countable("byHourTag", &["$createdAt", "tag"]);
        bucketed.time_range = Some(hourly());
        run(vec![ranked.clone(), bucketed])
            .expect("a grid-qualified sibling shares no level with the at chain");

        let aggregating = countable("byTimeTag", &["$createdAt", "tag"]);
        assert!(
            run(vec![ranked, aggregating]).is_err(),
            "the bare-key spelling reaches the at level and, aggregating, must stay rejected"
        );
    }

    /// A ranked index `[postAuthor, postId]` with `at: postAuthor`.
    fn author_ranked() -> Index {
        let mut ranked = index("byAuthorPost", &["postAuthor", "postId"]);
        ranked.countable = IndexCountability::Countable;
        ranked.range_countable = true;
        ranked.ranked_countable_at = vec!["postAuthor".to_string()];
        ranked
    }

    /// The motivating shape: a plain lookup sibling that shares the `at`
    /// level and continues below it is admitted — its branch is laid out
    /// count-exempt (`Element::NonCounted`) by the storage layer.
    #[test]
    fn a_plain_continuing_sibling_coexists_with_a_prefix_ranking() {
        let sibling = index("byAuthorTimePost", &["postAuthor", "day", "postId"]);
        run(vec![author_ranked(), sibling])
            .expect("a plain sibling continuing below the at level must be admitted");
    }

    /// A plain sibling sharing the WHOLE ranked property list and extending
    /// past the ranked index's terminal is the same admissible shape — its
    /// branch hangs inside the terminal level's value trees.
    #[test]
    fn a_plain_sibling_past_the_ranked_terminal_coexists() {
        let sibling = index("byAuthorPostTime", &["postAuthor", "postId", "day"]);
        run(vec![author_ranked(), sibling])
            .expect("a plain sibling extending past the ranked terminal must be admitted");
    }

    /// Deeper `at` positions admit plain siblings the same way, wherever
    /// the sibling diverges inside the chain's range.
    #[test]
    fn a_plain_sibling_diverging_at_a_propagating_level_coexists() {
        let mut ranked = index("byTagRegionPost", &["tag", "region", "postId"]);
        ranked.countable = IndexCountability::Countable;
        ranked.range_countable = true;
        ranked.ranked_countable_at = vec!["tag".to_string()];
        let sibling = index("byTagRegionDay", &["tag", "region", "day"]);
        run(vec![ranked, sibling])
            .expect("a plain sibling diverging below the propagating level must be admitted");
    }

    /// A sibling terminating EXACTLY at the `at` level stays rejected even
    /// when plain: its member bucket would sit on the grouping tree itself
    /// (the plain spelling is possible follow-up work, not admitted here).
    #[test]
    fn a_sibling_terminating_exactly_at_the_at_level_stays_rejected() {
        let terminator = index("byAuthor", &["postAuthor"]);
        assert!(
            run(vec![author_ranked(), terminator]).is_err(),
            "an exact-at terminator must stay rejected"
        );
    }

    /// Aggregating and ranked siblings stay rejected — their own aggregates
    /// would need exactly the counts the NonCounted wrapper suppresses.
    #[test]
    fn aggregating_and_ranked_siblings_stay_rejected() {
        let countable_sibling = {
            let mut sibling = countable("byAuthorTime", &["postAuthor", "day", "postId"]);
            sibling.countable = IndexCountability::Countable;
            sibling
        };
        assert!(
            run(vec![author_ranked(), countable_sibling]).is_err(),
            "a countable sibling must stay rejected"
        );

        let range_countable_sibling = {
            let mut sibling = index("byAuthorTime", &["postAuthor", "day", "postId"]);
            sibling.countable = IndexCountability::Countable;
            sibling.range_countable = true;
            sibling
        };
        assert!(
            run(vec![author_ranked(), range_countable_sibling]).is_err(),
            "a range-countable sibling must stay rejected"
        );

        let summable_sibling = {
            let mut sibling = index("byAuthorTime", &["postAuthor", "day", "postId"]);
            sibling.summable = Some("score".to_string());
            sibling
        };
        assert!(
            run(vec![author_ranked(), summable_sibling]).is_err(),
            "a summable sibling must stay rejected"
        );

        let ranked_sibling = {
            let mut sibling = index("byAuthorTime", &["postAuthor", "day", "postId"]);
            sibling.countable = IndexCountability::Countable;
            sibling.range_countable = true;
            sibling.ranked_countable_at = vec!["day".to_string()];
            sibling
        };
        assert!(
            run(vec![author_ranked(), ranked_sibling]).is_err(),
            "a sibling carrying its own prefix ranking must stay rejected"
        );

        let terminal_ranked_sibling = {
            let mut sibling = count_ranked("byAuthorTime", &["postAuthor", "day", "postId"]);
            sibling.name = "byAuthorTimeRanked".to_string();
            sibling
        };
        assert!(
            run(vec![author_ranked(), terminal_ranked_sibling]).is_err(),
            "a terminal-ranked sibling must stay rejected"
        );
    }

    /// The carve-out is scoped to the second loop's at-level rule: an
    /// aggregating index terminating at the exact prefix ABOVE a deeper
    /// `at` level still hits the wrapped-indexed impossibility.
    #[test]
    fn aggregating_prefix_above_the_at_level_stays_rejected() {
        let mut ranked = index("byTagRegionPost", &["tag", "region", "postId"]);
        ranked.countable = IndexCountability::Countable;
        ranked.range_countable = true;
        ranked.ranked_countable_at = vec!["region".to_string()];
        let aggregating_prefix = countable("byTag", &["tag"]);
        assert!(
            run(vec![ranked, aggregating_prefix]).is_err(),
            "the rule one level above the at stays untouched"
        );
    }
}
