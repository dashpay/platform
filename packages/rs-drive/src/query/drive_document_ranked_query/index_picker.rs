//! Covering-index picker for the ranked query, plus the shared
//! prefix-value encoding.
//!
//! Pure functions on the document type's index map plus the
//! `(group property, equality pins, axis, aggregate field)` tuple
//! [`super::mode_detection`] resolved. No Drive, no proof — the server
//! and the SDK verifier both call these so they land on the same index
//! (and therefore the same grove path) for the same request.

use super::{DocumentRankedMode, DriveDocumentRankedQuery, RankedAxis};
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::{DocumentTypeRef, Index};
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

/// Find the index that can serve `axis` ranking grouped by
/// `group_by_property` with the given equality pins, aggregating
/// `aggregate_field`.
///
/// An index qualifies when **all** of:
///
/// - it has exactly one more property than there are pins, and its
///   **last** property is `group_by_property` — the ranked secondary's
///   group keys are the values of an index's last property;
/// - every **leading** property is pinned: each appears (by name) among
///   `equality_pin_fields`. Lengths matching plus the pins being
///   distinct (enforced upstream by
///   [`super::mode_detection::equality_pins_from_where_clauses`]) makes
///   this set equality, so no pin is left over either;
/// - it declares the ranking keyword for `axis`
///   ([`RankedAxis::required_index_keyword`]);
/// - for [`RankedAxis::Sum`] / [`RankedAxis::Avg`], its `summable`
///   property is exactly `aggregate_field`. Both axes are derived from
///   the same running sum the index maintains (`Avg` is that sum over the
///   group's count), so summing a *different* field than the one the
///   index accumulates would silently answer about the wrong property.
///
/// With no pins this degenerates to the original single-property rule.
/// A partial pin (some but not all leading properties) matches nothing —
/// the per-prefix secondary lives under one value tree per leading
/// property, so there is no subtree an unpinned prefix could address —
/// and callers turn the `None` into a loud
/// [`crate::error::query::QuerySyntaxError`] naming what is missing.
///
/// Returns `None` when nothing qualifies; callers turn that into
/// [`crate::error::query::QuerySyntaxError::WhereClauseOnNonIndexedProperty`]
/// with a message naming the missing keyword.
///
/// At most one index can qualify for a given `(group property, pins,
/// axis, field)` tuple — rs-dpp rejects two indexes over the same
/// property set on one document type — so "first match wins" is not a
/// tie-break in practice. Should that ever change, the `BTreeMap`
/// iteration order (index name, ascending) keeps the choice
/// deterministic, which is what prover/verifier agreement actually
/// requires: both sides run this same function over the same contract
/// and must land on the same grove path.
///
/// Note that axis availability is decided from the index's `ranked_*`
/// flags, **not** from the element variant the write path laid down: a
/// `rankedCountable` index that also declares `rangeSummable` is stored
/// as a `ProvableCountProvableSumIndexedTree` carrying only the Count
/// axis, so the element variant alone would over-report what is rankable.
pub fn find_ranked_index_for_axis<'b>(
    indexes: &'b BTreeMap<String, Index>,
    group_by_property: &str,
    equality_pin_fields: &[String],
    axis: RankedAxis,
    aggregate_field: &str,
) -> Option<&'b Index> {
    indexes.values().find(|index| {
        // Trailing property is the grouping property; every leading
        // property is pinned exactly once (length equality + distinct
        // pins ⇒ set equality).
        let Some((terminal, leading)) = index.properties.split_last() else {
            return false;
        };
        if terminal.name != group_by_property
            || leading.len() != equality_pin_fields.len()
            || !leading
                .iter()
                .all(|property| equality_pin_fields.iter().any(|f| f == &property.name))
        {
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
    let pin_fields: Vec<String> = mode
        .equality_pins
        .iter()
        .map(|(field, _)| field.clone())
        .collect();
    find_ranked_index_for_axis(
        indexes,
        &mode.group_by_property,
        &pin_fields,
        mode.axis,
        &mode.aggregate_field,
    )
}

/// Resolve a validated [`DocumentRankedMode`] against a document type's
/// indexes into the executable [`DriveDocumentRankedQuery`]: pick the
/// covering index, encode the equality pins into prefix-value path
/// segments, and assemble the query.
///
/// This is the **one** resolution path — the server's executors and the
/// SDK's proof helpers both call it, which is what guarantees a proof
/// and an unproven read (and the client's verification) are about the
/// same subtree.
///
/// `indexes` is threaded in separately rather than read off
/// `document_type` here because
/// [`DocumentTypeV0Getters::indexes`](dpp::data_contract::document_type::accessors::DocumentTypeV0Getters::indexes)
/// borrows its receiver — taking the map from the caller lets the
/// returned query's `&'a Index` outlive this frame. Callers pass
/// `document_type.indexes()`.
///
/// The main failure is "no index covers this", reported with the exact
/// contract keyword (and, for pinned requests, the exact index shape)
/// the request needs, so the caller can act on it without reading the
/// schema spec.
pub fn resolve_ranked_query_for_mode<'a>(
    contract_id: [u8; 32],
    document_type: DocumentTypeRef<'a>,
    document_type_name: String,
    indexes: &'a BTreeMap<String, Index>,
    mode: &DocumentRankedMode,
    platform_version: &PlatformVersion,
) -> Result<DriveDocumentRankedQuery<'a>, Error> {
    let index = find_ranked_index_for_mode(indexes, mode).ok_or_else(|| {
        Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
            no_covering_index_message(
                "ranked",
                mode.axis,
                &mode.group_by_property,
                &mode.equality_pins,
                &mode.aggregate_field,
            ),
        ))
    })?;
    let equality_prefix_values =
        encode_equality_prefix_values(document_type, index, &mode.equality_pins, platform_version)?;
    Ok(DriveDocumentRankedQuery {
        document_type,
        contract_id,
        document_type_name,
        index,
        equality_prefix_values,
        axis: mode.axis,
        descending: mode.descending,
        k: mode.k,
        offset: mode.offset,
    })
}

/// The "no index covers this request" rejection text, shared by the
/// ranked and having-range resolutions (and the SDK's mirrors of them)
/// so a rejected request reads identically everywhere. Names the exact
/// index the request needs: property list (pins first, in request
/// order, then the grouping property), ranking keyword, and `summable`
/// field where applicable.
pub fn no_covering_index_message(
    surface: &str,
    axis: RankedAxis,
    group_by_property: &str,
    equality_pins: &[(String, Value)],
    aggregate_field: &str,
) -> String {
    let pin_fields = || {
        equality_pins
            .iter()
            .map(|(field, _)| field.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let index_shape = if equality_pins.is_empty() {
        format!("a single-property index on `{group_by_property}`")
    } else {
        format!(
            "a compound index on [{}, {group_by_property}] (every leading property pinned \
             by an equality `where` clause, the trailing property grouped over)",
            pin_fields()
        )
    };
    format!(
        "no ranked index covers `group_by = [{group_by_property}]`{} on the {axis:?} axis \
         for this {surface} query: the document type needs {index_shape} declaring `{}`{}",
        if equality_pins.is_empty() {
            String::new()
        } else {
            format!(" with equality pins on [{}]", pin_fields())
        },
        axis.required_index_keyword(),
        if aggregate_field.is_empty() {
            String::new()
        } else {
            format!(" with `summable: \"{aggregate_field}\"`")
        }
    )
}

/// Encode the equality pins into the grove path's prefix-value
/// segments: for each **leading** property of `index`, in index order,
/// the pinned value's index-key bytes
/// (`DocumentType::serialize_value_for_key` — the same encoding the
/// write path used to key that prefix's value tree).
///
/// This is part of the prover/verifier agreement: server executors and
/// the SDK's proof helpers both come through here, so a pinned value
/// can only ever name one subtree, identically on both sides.
///
/// `index` must have been picked by [`find_ranked_index_for_axis`]
/// against these same pins — every leading property is then guaranteed
/// a pin. A value the property's type cannot encode (a string against
/// an integer property, an out-of-range integer) is a caller error,
/// reported as a query-syntax rejection naming the property.
pub fn encode_equality_prefix_values(
    document_type: DocumentTypeRef,
    index: &Index,
    equality_pins: &[(String, Value)],
    platform_version: &PlatformVersion,
) -> Result<Vec<Vec<u8>>, Error> {
    let leading = &index.properties[..index.properties.len().saturating_sub(1)];
    leading
        .iter()
        .map(|property| {
            let (_, value) = equality_pins
                .iter()
                .find(|(field, _)| field == &property.name)
                .ok_or_else(|| {
                    Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                        "internal resolution mismatch: the picked compound ranked index has \
                         a leading property with no equality pin — the index picker and the \
                         prefix encoder disagreed on the pins",
                    ))
                })?;
            document_type
                .serialize_value_for_key(&property.name, value, platform_version)
                .map_err(|e| {
                    Error::Query(QuerySyntaxError::InvalidParameter(format!(
                        "the equality pin on `{}` does not encode as that property's \
                         index key: {e}",
                        property.name
                    )))
                })
        })
        .collect()
}
