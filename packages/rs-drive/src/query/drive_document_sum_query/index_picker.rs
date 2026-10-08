//! Sum-index pickers. Parallels count's `index_picker.rs`.
//!
//! Two pickers:
//! - [`find_summable_index_for_where_clauses`]: returns the index whose summed
//!   value equals the request's `sum_property` and whose properties the
//!   Equal/In where-clause fields *exactly* match, or, on a
//!   `summableOffCountIndex` index, cover as a prefix it can read whole: a
//!   prefix reaching its sum chain (the deepest pin's value tree) or every
//!   property but the last (the last property's tree). None on miss.
//! - [`find_range_summable_index_for_where_clauses`]: returns the
//!   `rangeSummable: true` index whose Equal/In prefix covers the
//!   non-range clauses AND whose last property is the range
//!   terminator. None on miss.
//!
//! Reject-on-miss is the load-bearing contract: callers landing in
//! "no covering index" return `WhereClauseOnNonIndexedProperty` so the
//! prover and verifier reject the same set of inputs (same as count).

use crate::query::drive_document_sum_query::{is_indexable_for_sum, is_range_operator};
use crate::query::ResolvedTimeRange;
use crate::query::{
    index_admissible_for_query, pins_reach_chain, SkipIfAbsentBinding, WhereClause, WhereOperator,
};
use dpp::data_contract::document_type::Index;
use std::collections::{BTreeMap, BTreeSet};

/// Find an index whose summed value equals the request's `sum_property` and
/// whose properties the Equal/In where-clause fields exactly cover, or, on a
/// `summableOffCountIndex` index, cover as a prefix it reads whole (see
/// `find_summable_index_accepted_by`).
///
/// Mirror of count's `find_countable_index_for_where_clauses` with the
/// additional summed-value predicate on top of the coverage match.
///
/// `resolved_time_ranges` names the fields whose equality clause was
/// produced by `IN_TIME_RANGE` resolution (see
/// [`crate::query::resolve_time_range_bucket_clause`]) and gates which indexes
/// are candidates — see [`index_admissible_for_resolved_time_range`](crate::query::index_admissible_for_resolved_time_range).
pub fn find_summable_index_for_where_clauses<'b>(
    indexes: &'b BTreeMap<String, Index>,
    where_clauses: &[WhereClause],
    sum_property: &str,
    resolved_time_ranges: &[ResolvedTimeRange],
) -> Option<&'b Index> {
    find_summable_index_accepted_by(
        indexes,
        where_clauses,
        sum_property,
        resolved_time_ranges,
        |_| true,
    )
}

/// [`find_summable_index_for_where_clauses`] for an average or count-and-sum
/// read: the picked index's read element must also carry a count
/// ([`summable_point_lookup_carries_counts`]). A `summableOffCountIndex` index
/// that cannot answer is passed over, so it never hides, by name order, one
/// that can. A regular index is picked first and judged after, as before
/// protocol version 14, so a query over regular indexes keeps resolving to
/// the index released verifiers rebuild.
pub fn find_summable_index_with_counts_for_where_clauses<'b>(
    indexes: &'b BTreeMap<String, Index>,
    where_clauses: &[WhereClause],
    sum_property: &str,
    resolved_time_ranges: &[ResolvedTimeRange],
) -> Option<&'b Index> {
    find_summable_index_accepted_by(
        indexes,
        where_clauses,
        sum_property,
        resolved_time_ranges,
        |index| {
            !index.is_summable_off_count_index()
                || summable_point_lookup_carries_counts(index, where_clauses)
        },
    )
    // A counter index was judged in the loop already (`accepts`).
    .filter(|index| {
        index.is_summable_off_count_index()
            || summable_point_lookup_carries_counts(index, where_clauses)
    })
}

/// The first index, in name order, that a point sum over `where_clauses`
/// reads and that `accepts`: exactly covering, else through its last
/// property's tree ([`prefix_to_last_sum_reads`]), else through a sum chain
/// (the deepest pin's value tree), in the order count's picker and the sum
/// builder try them. The order is the same but not the test: an average needs
/// the last tree to carry counts, so `prefix_to_last_sum_reads` refuses an
/// index whose last tree does not count (not `rangeCountable`) when its pins
/// reach a sum chain, which count's picker accepts. A `count(*)` and a sum
/// with the same pins may then read different indexes; both totals are the
/// counters' sums under the pins.
fn find_summable_index_accepted_by<'b>(
    indexes: &'b BTreeMap<String, Index>,
    where_clauses: &[WhereClause],
    sum_property: &str,
    resolved_time_ranges: &[ResolvedTimeRange],
    accepts: impl Fn(&Index) -> bool,
) -> Option<&'b Index> {
    // A skip index serves only a query binding every skip property
    // ([`index_admissible_for_skip_if_absent`](crate::query::index_admissible_for_skip_if_absent)): a prefix match may stop
    // above a deep one.
    let skip_bindings = SkipIfAbsentBinding::for_where_clauses(where_clauses);
    // Defense-in-depth: any non-indexable operator immediately disqualifies
    // — the sum point-lookup path can only serve Equal/In.
    if where_clauses
        .iter()
        .any(|wc| !is_indexable_for_sum(wc.operator))
    {
        return None;
    }

    let indexable_fields: BTreeSet<&str> = where_clauses
        .iter()
        .filter(|wc| matches!(wc.operator, WhereOperator::Equal | WhereOperator::In))
        .map(|wc| wc.field.as_str())
        .collect();

    if indexable_fields.is_empty() {
        return None;
    }

    for index in indexes.values() {
        // A time-range index holds one entry per bucket containing the
        // document, keyed by bucket start: summing over it double-counts
        // every document unless the query pins a single bucket, and only a
        // resolution-produced equality does that. Conversely a raw clause
        // must never bind to bucket keys.
        if !index_admissible_for_query(index, resolved_time_ranges, &skip_bindings) {
            continue;
        }
        // Skip if not summable OR if summable property doesn't match. A
        // `summableOffCountIndex` index is addressed by its source index's
        // name, its counters holding that index's entry counts.
        if index.summed_value_name() != Some(sum_property) {
            continue;
        }
        if index.properties.len() != indexable_fields.len() {
            continue;
        }
        let all_covered = index
            .properties
            .iter()
            .all(|prop| indexable_fields.contains(prop.name.as_str()));
        if all_covered && accepts(index) {
            return Some(index);
        }
    }

    // The partial-cover forms, prefix-to-last first as in count's picker:
    // the leading `pin_depth` properties of an index that `form` admits at
    // that depth.
    //
    // Prefix-to-last form, the sum counterpart of count's: a
    // `summableOffCountIndex` index pinned on every property but its last
    // reads the tree of its last property, which sums the counters below the
    // pins ([`prefix_to_last_sum_reads`]), the element a `count(*)` with the
    // same pins reads when both pick that index (see the function doc).
    //
    // Sum-chain value-tree form, the sum counterpart of count's at-chain
    // fallback: on a `summableOffCountIndex` index ranking by sum or average
    // at an earlier level, every value tree from the shallowest such level
    // down sums its whole subtree, so contiguous pins landing at or below
    // that level are servable by reading the deepest pin's value tree
    // element. Pins landing above it stay rejected: those levels are plain
    // trees.
    let pin_depth = indexable_fields.len();
    let first_leading_cover = |form: &dyn Fn(&Index, usize) -> bool| {
        indexes.values().find(|index| {
            index_admissible_for_query(index, resolved_time_ranges, &skip_bindings)
                && index.summed_value_name() == Some(sum_property)
                && form(index, pin_depth)
                && index.properties[..pin_depth]
                    .iter()
                    .all(|prop| indexable_fields.contains(prop.name.as_str()))
                && accepts(index)
        })
    };
    first_leading_cover(&prefix_to_last_sum_reads).or_else(|| {
        first_leading_cover(&|index, depth| {
            pins_reach_chain(index, depth, index.shallowest_sum_chain_position())
        })
    })
}

/// Whether a point sum over `index` with its first `pin_depth` properties
/// pinned reads the tree of its last property, the prefix-to-last form: a
/// `summableOffCountIndex` index pinned on every property but its last, the
/// last unranked (a ranked one is an indexed tree, which grovedb's query
/// dispatch refuses to return). That tree sums the counters below the pins,
/// the source's entries, and when the index is `rangeCountable` it counts
/// them too, the groups: it then answers even pins that reach the sum chain,
/// as count's form does, so an average reads the groups a sum-only chain's
/// value trees do not count. Without `rangeCountable`, pins reaching the sum
/// chain read the deepest pin's value tree instead. The picker and the path
/// builder both ask this, so they agree on the form.
///
/// Only a `summableOffCountIndex` index, which only meta-schema v3 (protocol
/// version 14) admits, takes this form: every other index reads as before.
pub(crate) fn prefix_to_last_sum_reads(index: &Index, pin_depth: usize) -> bool {
    index.is_summable_off_count_index()
        && !index.ranks_its_last_property()
        && pin_depth >= 1
        && pin_depth + 1 == index.properties.len()
        && (index.range_countable
            || !pins_reach_chain(index, pin_depth, index.shallowest_sum_chain_position()))
}

/// Whether the element a sum point lookup on `index` reads for
/// `where_clauses` also carries a count, as an average or count-and-sum read
/// needs: the index is countable, and the read lands on its terminal (every
/// property pinned), on a value tree of its count chain, or, in the
/// prefix-to-last form, on its last property's tree when that tree counts
/// (`rangeCountable`). A sum-chain level that carries no count holds
/// `SumTree`s, whose count would read as one.
fn summable_point_lookup_carries_counts(index: &Index, where_clauses: &[WhereClause]) -> bool {
    if !index.countable.is_countable() {
        return false;
    }
    let pin_depth = index
        .properties
        .iter()
        .take_while(|prop| where_clauses.iter().any(|wc| wc.field == prop.name))
        .count();
    pin_depth == index.properties.len()
        || pins_reach_chain(index, pin_depth, index.shallowest_count_chain_position())
        || (index.range_countable && prefix_to_last_sum_reads(index, pin_depth))
}

/// Find a `rangeSummable: true` index whose properties cover the
/// non-range Equal/In clauses as a prefix AND whose last property is
/// the range terminator. The summed property must match
/// `sum_property`.
///
/// Mirror of count's `find_range_countable_index_for_where_clauses`.
///
/// `resolved_time_ranges` gates the candidate set exactly as in
/// [`find_summable_index_for_where_clauses`]. A resolved field never arrives
/// as a range clause — resolution always produces an equality — so with a
/// non-empty list the only bucketed index this can return is one whose
/// resolved equality is a prefix property and whose range terminator is a
/// different property. That is the intended shape: a range over one property
/// within a single time bucket.
pub fn find_range_summable_index_for_where_clauses<'b>(
    indexes: &'b BTreeMap<String, Index>,
    where_clauses: &[WhereClause],
    sum_property: &str,
    resolved_time_ranges: &[ResolvedTimeRange],
) -> Option<&'b Index> {
    find_range_summable_index_accepted_by(
        indexes,
        where_clauses,
        sum_property,
        resolved_time_ranges,
        |_| true,
    )
}

/// [`find_range_summable_index_for_where_clauses`] for a range average or
/// count-and-sum read: the picked index must also be `rangeCountable`. A
/// `summableOffCountIndex` index that is not is passed over, so it never
/// hides, by name order, one that is. A regular index is picked first and
/// judged after, as before protocol version 14, so a query over regular
/// indexes keeps resolving to the index released verifiers rebuild.
pub fn find_range_summable_index_with_counts_for_where_clauses<'b>(
    indexes: &'b BTreeMap<String, Index>,
    where_clauses: &[WhereClause],
    sum_property: &str,
    resolved_time_ranges: &[ResolvedTimeRange],
) -> Option<&'b Index> {
    find_range_summable_index_accepted_by(
        indexes,
        where_clauses,
        sum_property,
        resolved_time_ranges,
        |index| !index.is_summable_off_count_index() || index.range_countable,
    )
    .filter(|index| index.range_countable)
}

/// The first index, in name order, that a range sum over `where_clauses`
/// reads and that `accepts`.
fn find_range_summable_index_accepted_by<'b>(
    indexes: &'b BTreeMap<String, Index>,
    where_clauses: &[WhereClause],
    sum_property: &str,
    resolved_time_ranges: &[ResolvedTimeRange],
    accepts: impl Fn(&Index) -> bool,
) -> Option<&'b Index> {
    // A skip index serves only a query binding every skip property
    // ([`index_admissible_for_skip_if_absent`](crate::query::index_admissible_for_skip_if_absent)): a prefix match may stop
    // above a deep one.
    let skip_bindings = SkipIfAbsentBinding::for_where_clauses(where_clauses);
    let range_clauses: Vec<&WhereClause> = where_clauses
        .iter()
        .filter(|wc| is_range_operator(wc.operator))
        .collect();
    let (outer_range_field, terminator_range_clause) = match range_clauses.len() {
        1 => (None, range_clauses[0]),
        2 => {
            // Same-field two-sided ranges are flattened into `between*`
            // and arrive as one clause; reject if same-field anyway.
            if range_clauses[0].field == range_clauses[1].field {
                return None;
            }
            (
                Some((
                    range_clauses[0].field.as_str(),
                    range_clauses[1].field.as_str(),
                )),
                range_clauses[0],
            )
        }
        _ => return None,
    };

    // Reject any operator that's neither indexable (Equal/In) nor a
    // range operator — anything else has no defined sum semantics.
    if where_clauses
        .iter()
        .any(|wc| !is_indexable_for_sum(wc.operator) && !is_range_operator(wc.operator))
    {
        return None;
    }

    let prefix_fields: BTreeSet<&str> = where_clauses
        .iter()
        .filter(|wc| matches!(wc.operator, WhereOperator::Equal | WhereOperator::In))
        .map(|wc| wc.field.as_str())
        .collect();

    for index in indexes.values() {
        // Same admissibility rule as the point-lookup picker: bucketed
        // indexes store one entry per containing bucket, so only a query
        // pinned to a single bucket by a resolution-produced equality may
        // walk them, and raw clauses may never bind to bucket keys.
        if !index_admissible_for_query(index, resolved_time_ranges, &skip_bindings) {
            continue;
        }
        if !index.range_summable {
            continue;
        }
        // `range_summable: true` requires `summable: Some(_)` (or a
        // `summableOffCountIndex` source) per the DPP schema; verify it
        // matches the caller's sum_property.
        if index.summed_value_name() != Some(sum_property) {
            continue;
        }

        if let Some((field_a, field_b)) = outer_range_field {
            let terminator = index.properties.last()?;
            let first = index.properties.first()?;
            let (outer_field, _terminator_field) = if terminator.name == field_a {
                (field_b, field_a)
            } else if terminator.name == field_b {
                (field_a, field_b)
            } else {
                continue;
            };
            if first.name != outer_field {
                continue;
            }
            let intermediate_props = &index.properties[1..index.properties.len() - 1];
            let mut intermediate_props_ok = true;
            for prop in intermediate_props {
                if !prefix_fields.contains(prop.name.as_str()) {
                    intermediate_props_ok = false;
                    break;
                }
            }
            // Strict-coverage check: every Equal/In prefix field must
            // appear in the index's intermediate properties. Without
            // this `intermediate_props.len() == prefix_fields.len()`
            // guard, a query with extra prefix fields would silently
            // pick an index that *doesn't* cover them, producing an
            // over-broad result.
            if intermediate_props_ok
                && intermediate_props.len() == prefix_fields.len()
                && accepts(index)
            {
                return Some(index);
            }
            continue;
        }

        // Single-range case.
        let mut prefix_len = 0usize;
        for prop in &index.properties {
            if prefix_fields.contains(prop.name.as_str()) {
                prefix_len += 1;
            } else {
                break;
            }
        }
        if prefix_len < prefix_fields.len() {
            continue;
        }
        if prefix_len + 1 != index.properties.len() {
            continue;
        }
        let range_prop = &index.properties[prefix_len];
        if range_prop.name == terminator_range_clause.field && accepts(index) {
            return Some(index);
        }
    }

    None
}
