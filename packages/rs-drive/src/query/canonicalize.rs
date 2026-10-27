//! Shared where-clause validation + canonicalization for the aggregate
//! query surfaces (count / sum / average / joint count-and-sum) and
//! their SDK proof verifiers.
//!
//! Lives outside the per-surface dispatcher modules because the shape
//! contract must be identical on every route: the server dispatchers
//! canonicalize before mode detection, and the proof verifiers must run
//! the very same canonicalization before *their* mode detection or a
//! proof the server produced for the canonical shape is rejected
//! client-side (the count dispatcher promises callers "the bounded form
//! and the pre-merged form get equivalent mode detection" — that promise
//! only holds if verifiers canonicalize too).

use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::drive_document_count_query::DriveDocumentCountQuery;
use crate::query::{WhereClause, WhereOperator};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;

/// Key the clauses of a count, sum, average, ranked or having request that read
/// the elements of a typed array (protocol version 14) as the index levels of
/// the array are keyed: `contains` as an equality on one element's key,
/// `containsAny` as an `in` over several. The aggregate surfaces then serve
/// them as any equality or `in`, each element a group of its own. Every other
/// clause passes through unchanged.
///
/// A clause on a typed array must be one of the two, with values that key as
/// its elements, and a clause reading elements must name a typed array
/// ([`WhereClause::validate_array_element_clause`]): the marker is gone once a
/// clause is keyed, so this is where the field's type is judged.
/// `grouped_by_in` says whether the request is grouped by its `In` field
/// alone (a `containsAny` routes as one): only then, and without a range
/// clause, does it answer per element when it reads several
/// ([`answers_per_element`]). A total over several elements would count a
/// document once per element it holds, so it is refused. The ranked and
/// having surfaces and a count sub-query read one element at most, and pass
/// `false`.
pub fn key_array_element_clauses(
    clauses: Vec<WhereClause>,
    document_type: DocumentTypeRef,
    grouped_by_in: bool,
    platform_version: &PlatformVersion,
) -> Result<Vec<WhereClause>, Error> {
    let several_elements_allowed = answers_per_element(grouped_by_in, &clauses);
    clauses
        .into_iter()
        .map(|mut clause| {
            clause.validate_array_element_clause(document_type, platform_version)?;
            if !clause.operator.reads_array_elements() {
                return Ok(clause);
            }
            if clause.operator == WhereOperator::ContainsAny && !several_elements_allowed {
                return Err(Error::Query(QuerySyntaxError::Unsupported(format!(
                    "containsAny on \"{}\" is only answered grouped by \"{}\" alone, one total \
                     per element, and without a range clause: a total over several elements \
                     would count a document once per element it holds; read each element \
                     with contains otherwise",
                    clause.field, clause.field
                ))));
            }
            clause.operator = clause.operator.as_key_operator();
            Ok(clause)
        })
        .collect()
}

/// Whether a count, sum or average request reading several elements of a
/// typed array (`containsAny`) answers one total per element: grouped by the
/// `In` field alone (`containsAny` routes as one, so it is grouped by the
/// array), with no range clause, over which the surfaces fold a group's
/// totals. Applied by [`key_array_element_clauses`] to the clauses it keys,
/// for the aggregate dispatchers and the SDK proof verifiers alike, so a
/// verifier refuses exactly the totals the server refuses.
fn answers_per_element(grouped_by_in: bool, where_clauses: &[WhereClause]) -> bool {
    grouped_by_in
        && !where_clauses
            .iter()
            .any(|clause| DriveDocumentCountQuery::is_range_operator(clause.operator))
}

/// Run the system-wide where-clause validator on a structured
/// `Vec<WhereClause>` and canonicalize same-field range pairs into
/// their `between*` form. Single source of truth for the aggregate
/// shape contract; called by the count / sum / average / joint
/// dispatchers, the legacy CBOR-decoded count entry, and the SDK
/// count / sum / average proof verifiers.
///
/// The validator (`WhereClause::group_clauses`) rejects:
/// - Duplicate `Equal` clauses on the same field
///   (`DuplicateNonGroupableClauseSameField`).
/// - Multiple `In` clauses (`MultipleInClauses`) — rejected here: the
///   shared grammar accepts them for protocol version 14+ document
///   queries, but the aggregate surfaces do not.
/// - Multiple non-groupable range clauses (`MultipleRangeClauses`).
/// - Equality + `In` on the same field, range + equality/In on the
///   same field (`DuplicateNonGroupableClauseSameField` /
///   `InvalidWhereClauseComponents`).
///
/// Without this validation, downstream
/// [`DriveDocumentCountQuery::find_countable_index_for_where_clauses`]
/// collapses repeated fields into a `BTreeSet` and
/// [`DriveDocumentCountQuery::point_lookup_count_path_query`]
/// resolves each index property with a single `.find(...)` — both
/// of which silently pick the first clause on a duplicated field
/// and return a count for an arbitrarily reduced query rather than
/// rejecting the malformed request.
///
/// **Exception**: `MultipleRangeClauses` is intentionally tolerated
/// here. The regular-query parser rejects two ranges on different
/// fields wholesale (its callers expect
/// `(equal_clauses, in_clause, range_clause)` triples), but the
/// count-query path accepts the carrier-aggregate shape
/// (`outer_range + inner_ACOR_range` on different fields, e.g.
/// G8). Structural validation for that shape lives in
/// [`DriveDocumentCountQuery::detect_mode`] (which knows about
/// `CountMode::GroupByRange`-with-two-ranges and routes to
/// `DocumentCountMode::RangeAggregateCarrierProof`); replicating
/// it here would be redundant.
///
/// After validation, [`merge_same_field_range_pairs`] collapses
/// `[field > A, field < B]` (and analogous pairs with `>=` / `<=`)
/// into the canonical `between*` operator that
/// [`DriveDocumentCountQuery::range_clause_to_query_item`] knows
/// how to convert into a single `QueryItem`. The regular-query
/// parser does the same merge before its grouped-triple
/// validation; for aggregate queries we do it explicitly here so
/// callers can pass either the bounded form (e.g.
/// `[brand > A, brand < B]`) or the pre-merged form (e.g.
/// `[brand BetweenExcludeBounds [A, B]]`) and get equivalent
/// mode detection downstream. Without this merge, G8a's natural
/// wire shape (four range clauses, two per field) would slip past
/// the catch-`MultipleRangeClauses` block above and then get
/// rejected by `detect_mode`'s `range_count > 1` structural check.
pub fn validate_and_canonicalize_where_clauses(
    clauses: Vec<WhereClause>,
    platform_version: &PlatformVersion,
) -> Result<Vec<WhereClause>, Error> {
    match WhereClause::group_clauses(&clauses, platform_version) {
        // Multiple `In` clauses are a document-query-only shape (protocol
        // version 14+); the aggregate surfaces keep rejecting them since
        // their mode detection and index pickers assume a single `In`.
        Ok((_, _, in_clauses)) if in_clauses.len() > 1 => {
            return Err(Error::Query(QuerySyntaxError::MultipleInClauses(
                "aggregate queries support at most one in clause",
            )));
        }
        Ok(_) => {}
        Err(Error::Query(QuerySyntaxError::MultipleRangeClauses(_))) => {}
        Err(e) => return Err(e),
    }
    merge_same_field_range_pairs(clauses)
}

/// Collapse `[field > A, field < B]` (and analogous pairs with
/// `>=` / `<=`) into a single `field between* [A, B]` clause per
/// field. Equality / In clauses pass through unchanged.
///
/// Returns an error if a field has more than two range clauses
/// (structurally meaningless — a third bound would either
/// contradict an existing one or be redundant) or if the pair
/// isn't one lower-bound + one upper-bound (e.g. two `>` on the
/// same field).
fn merge_same_field_range_pairs(clauses: Vec<WhereClause>) -> Result<Vec<WhereClause>, Error> {
    use crate::query::conditions::WhereOperator::{
        Between, BetweenExcludeBounds, BetweenExcludeLeft, BetweenExcludeRight, GreaterThan,
        GreaterThanOrEquals, LessThan, LessThanOrEquals,
    };
    use std::collections::BTreeMap;

    let mut by_field: BTreeMap<String, Vec<WhereClause>> = BTreeMap::new();
    let mut non_range: Vec<WhereClause> = Vec::new();
    for wc in clauses {
        if DriveDocumentCountQuery::is_range_operator(wc.operator) {
            by_field.entry(wc.field.clone()).or_default().push(wc);
        } else {
            non_range.push(wc);
        }
    }
    let mut result = non_range;
    for (field, mut ranges) in by_field {
        match ranges.len() {
            0 => {}
            1 => result.push(ranges.remove(0)),
            2 => {
                let (mut lower, mut upper): (Option<WhereClause>, Option<WhereClause>) =
                    (None, None);
                for r in ranges {
                    match r.operator {
                        GreaterThan | GreaterThanOrEquals => {
                            if lower.is_some() {
                                return Err(Error::Query(QuerySyntaxError::MultipleRangeClauses(
                                    "two lower-bound range clauses on the same field cannot be \
                                     merged; combine via `between*` or remove the redundant clause",
                                )));
                            }
                            lower = Some(r);
                        }
                        LessThan | LessThanOrEquals => {
                            if upper.is_some() {
                                return Err(Error::Query(QuerySyntaxError::MultipleRangeClauses(
                                    "two upper-bound range clauses on the same field cannot be \
                                     merged; combine via `between*` or remove the redundant clause",
                                )));
                            }
                            upper = Some(r);
                        }
                        _ => {
                            // The other range operators (Between*,
                            // StartsWith) are themselves bounded
                            // already; a second range clause on the
                            // same field is structurally redundant.
                            return Err(Error::Query(QuerySyntaxError::MultipleRangeClauses(
                                "cannot pair a `between*`/`startsWith` range clause with \
                                 another range on the same field; use the pre-merged form",
                            )));
                        }
                    }
                }
                let lower = lower.ok_or(Error::Query(QuerySyntaxError::MultipleRangeClauses(
                    "two range clauses on the same field require one lower bound (> or >=) \
                     and one upper bound (< or <=)",
                )))?;
                let upper = upper.ok_or(Error::Query(QuerySyntaxError::MultipleRangeClauses(
                    "two range clauses on the same field require one lower bound (> or >=) \
                     and one upper bound (< or <=)",
                )))?;
                let merged_op = match (
                    lower.operator == GreaterThanOrEquals,
                    upper.operator == LessThanOrEquals,
                ) {
                    (true, true) => Between,                // [a, b]
                    (false, false) => BetweenExcludeBounds, // (a, b)
                    (true, false) => BetweenExcludeRight,   // [a, b)
                    (false, true) => BetweenExcludeLeft,    // (a, b]
                };
                result.push(WhereClause {
                    field,
                    operator: merged_op,
                    value: dpp::platform_value::Value::Array(vec![lower.value, upper.value]),
                });
            }
            _ => {
                return Err(Error::Query(QuerySyntaxError::MultipleRangeClauses(
                    "more than two range clauses on the same field are not supported; a \
                     bounded range needs exactly one lower bound and one upper bound",
                )));
            }
        }
    }
    Ok(result)
}
