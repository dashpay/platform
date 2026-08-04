//! Covering-index picker for the ranked query.
//!
//! Pure function on the document type's index map plus the
//! `(group property, axis, aggregate field)` triple
//! [`super::mode_detection`] resolved. No Drive, no proof — the server
//! and the SDK verifier both call it so they land on the same index (and
//! therefore the same grove path) for the same request.

use super::{DocumentRankedMode, RankedAxis};
use dpp::data_contract::document_type::Index;
use std::collections::BTreeMap;

/// Find the index that can serve `axis` ranking grouped by
/// `group_by_property`, aggregating `aggregate_field`.
///
/// An index qualifies when **all** of:
///
/// - it has exactly one property, and that property is
///   `group_by_property` — the ranked secondary's group keys are the
///   values of an index's *last* property, and rs-dpp only allows
///   single-property ranked indexes, so "last" and "only" coincide;
/// - it declares the ranking keyword for `axis`
///   ([`RankedAxis::required_index_keyword`]);
/// - for [`RankedAxis::Sum`] / [`RankedAxis::Avg`], its `summable`
///   property is exactly `aggregate_field`. Both axes are derived from
///   the same running sum the index maintains (`Avg` is that sum over the
///   group's count), so summing a *different* field than the one the
///   index accumulates would silently answer about the wrong property.
///
/// Returns `None` when nothing qualifies; callers turn that into
/// [`crate::error::query::QuerySyntaxError::WhereClauseOnNonIndexedProperty`]
/// with a message naming the missing keyword.
///
/// At most one index can qualify for a given `(group property, axis,
/// field)` triple — rs-dpp rejects two indexes over the same property set
/// on one document type — so "first match wins" is not a tie-break in
/// practice. Should that ever change, the `BTreeMap` iteration order
/// (index name, ascending) keeps the choice deterministic, which is what
/// prover/verifier agreement actually requires: both sides run this same
/// function over the same contract and must land on the same grove path.
///
/// Note that axis availability is decided from the index's `ranked_*`
/// flags, **not** from the element variant the write path laid down: a
/// `rankedCountable` index that also declares `rangeSummable` is stored
/// as a `ProvableCountProvableSumIndexedTree` carrying only the Count
/// axis, so the element variant alone would over-report what is rankable.
pub fn find_ranked_index_for_axis<'b>(
    indexes: &'b BTreeMap<String, Index>,
    group_by_property: &str,
    axis: RankedAxis,
    aggregate_field: &str,
) -> Option<&'b Index> {
    indexes.values().find(|index| {
        // Single-property, and that property is the grouping property.
        if index.properties.len() != 1 || index.properties[0].name != group_by_property {
            return false;
        }
        match axis {
            RankedAxis::Count => index.ranked_countable,
            RankedAxis::Sum => {
                index.ranked_summable && index.summable.as_deref() == Some(aggregate_field)
            }
            RankedAxis::Avg => {
                index.ranked_averageable && index.summable.as_deref() == Some(aggregate_field)
            }
        }
    })
}

/// [`find_ranked_index_for_axis`] driven straight from a resolved
/// [`DocumentRankedMode`] — the shape every caller actually has.
pub fn find_ranked_index_for_mode<'b>(
    indexes: &'b BTreeMap<String, Index>,
    mode: &DocumentRankedMode,
) -> Option<&'b Index> {
    find_ranked_index_for_axis(
        indexes,
        &mode.group_by_property,
        mode.axis,
        &mode.aggregate_field,
    )
}
