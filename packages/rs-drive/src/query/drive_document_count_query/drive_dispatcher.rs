//! Top-level dispatcher for the unified `GetDocumentsCount` request.
//!
//! Owns the whole pipeline: CBOR-decode → mode detection →
//! per-mode executor (see [`super::executors`]) → response
//! wrapping. The drive-abci handler builds a
//! [`DocumentCountRequest`] and calls
//! [`Drive::execute_document_count_request`]; everything past
//! contract lookup lives in drive.
//!
//! Both `DocumentCountRequest` and `DocumentCountResponse` are
//! the ABI for this dispatcher — they're public so drive-abci can
//! name the input/output types without reaching into the
//! executor surface.
//!
//! Module is gated `feature = "server"` via the parent's
//! `pub mod drive_dispatcher;` declaration.

use super::super::conditions::WhereClause;
use super::super::ordering::OrderClause;
use super::execute_range_count::RangeCountOptions;
use super::{DocumentCountMode, DriveDocumentCountQuery, SplitCountEntry};
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

// `impl Drive { ... per-mode executors ... }` lives in
// [`super::executors`] — it's a deliberate physical split between
// "dispatcher routes" (this file) and "executors execute" (sibling).
// All per-mode executor methods this file calls
// (`execute_document_count_total_no_proof` etc.) are reachable via
// the shared `Drive` type from there.

/// All inputs required for the unified document-count entry point
/// [`Drive::execute_document_count_request`]. Built by the gRPC
/// handler from a `GetDocumentsRequestV1` after wire-decoding +
/// contract lookup; drive owns everything past this point including
/// mode-detection-from-clauses, index picking, and per-mode dispatch.
///
/// `where_clauses` and `order_clauses` arrive already structured —
/// the v1 ABCI handler converts proto `repeated WhereClause` /
/// `repeated OrderClause` upstream; benches and tests that want a
/// `Value`-shape fixture call [`where_clauses_from_value`] /
/// [`order_clauses_from_value`] to parse before constructing the
/// request. The dispatcher entry point runs
/// [`validate_and_canonicalize_where_clauses`] on the input so
/// shape-validation rejection (duplicate equal, multiple In, …)
/// and the `> AND <` → `between*` canonicalization happen
/// regardless of upstream path.
pub struct DocumentCountRequest<'a> {
    /// Live contract (already loaded by the handler).
    pub contract: &'a dpp::data_contract::DataContract,
    /// Resolved document type within `contract`.
    pub document_type: DocumentTypeRef<'a>,
    /// Structured `where` clauses. The dispatcher runs the same
    /// [`WhereClause::group_clauses`] validator + same-field
    /// range-pair merge the regular document-query path runs (see
    /// [`validate_and_canonicalize_where_clauses`]'s docstring for
    /// the catalog of rejections this enables and the In/range +
    /// `between*` canonicalization rules) before mode detection.
    pub where_clauses: Vec<WhereClause>,
    /// The fields among `where_clauses` whose equality clause was produced by
    /// `IN_TIME_RANGE` resolution rather than written by the caller. Same
    /// contract and same purpose as
    /// [`crate::query::DriveDocumentQuery::resolved_time_range_fields`]:
    /// it is what gates which indexes the count pickers may select.
    pub resolved_time_range_fields: Vec<String>,
    /// Structured `order_by` clauses. The first clause's direction
    /// governs split-mode entry ordering (per-`In`-value /
    /// per-distinct-value-in-range) and, on the
    /// `RangeDistinctProof` prove path, is part of the path-query
    /// bytes the SDK reconstructs to verify the proof.
    /// `PointLookupProof` and the no-proof `Total` / `PerInValue`
    /// paths don't read order_by. Empty list → ascending default
    /// for split-mode response ordering.
    pub order_clauses: Vec<OrderClause>,
    /// SQL-shaped output mode — the caller's `(select, group_by)`
    /// contract resolved into one of four shapes (Aggregate,
    /// GroupByIn, GroupByRange, GroupByCompound). The dispatcher
    /// uses this to distinguish e.g. "aggregate count with In
    /// fan-out" (which does NOT accept `limit`) from "per-In-value
    /// entries" (which does) — they're otherwise indistinguishable
    /// from the where clauses alone. See [`CountMode`] for the
    /// per-variant where-clause and `limit` invariants.
    pub mode: super::CountMode,
    /// Limit cap from the request. Callers SHOULD pre-clamp against
    /// their server-side `max_query_limit` policy, but Drive also
    /// enforces a defense-in-depth clamp before forwarding to the
    /// distinct-mode walk: an `Option::None` here is normalized to
    /// `drive_config.default_query_limit` and any `Some(value)` is
    /// reduced to `drive_config.max_query_limit` if larger. After
    /// dispatch, the limit forwarded to
    /// [`RangeCountOptions::limit`] is always `Some(_)` ≤ system cap.
    pub limit: Option<u32>,
    /// Whether to produce a proof (vs. raw counts).
    pub prove: bool,
    /// Drive-side query config — only consumed by the materialize-and-
    /// count fallback.
    pub drive_config: &'a crate::config::DriveConfig,
}

/// Output shape of [`Drive::execute_document_count_request`]. Three
/// variants mirror the proto's `CountResults.variant` oneof (for
/// no-proof responses) plus the outer `Proof` arm:
///
/// - `Aggregate(u64)` — total-count modes (`Total` and
///   `RangeNoProof` under [`super::CountMode::Aggregate`]). The abci
///   handler maps this to `CountResults.aggregate_count`.
/// - `Entries(Vec<SplitCountEntry>)` — per-key modes (`PerInValue`
///   and `RangeNoProof` under [`super::CountMode::GroupByRange`] /
///   [`super::CountMode::GroupByCompound`]). The abci handler maps
///   this to `CountResults.entries`.
/// - `Proof(Vec<u8>)` — grovedb proof bytes the client verifies via
///   either `verify_aggregate_count_query` (for `RangeProof`),
///   `verify_distinct_count_proof` (for `RangeDistinctProof`), or
///   the `DriveDocumentQuery` proof verifier (for
///   `PointLookupProof`).
#[derive(Debug, Clone)]
pub enum DocumentCountResponse {
    /// Single aggregate count — total across the matching set.
    Aggregate(u64),
    /// Per-key entries.
    Entries(Vec<SplitCountEntry>),
    /// Grovedb proof bytes.
    Proof(Vec<u8>),
}

/// Parse the decoded `where` value into structured [`WhereClause`]s.
///
/// Mirrors the per-clause loop the regular `query_documents_v0`
/// handler delegates to `DriveDocumentQuery::from_decomposed_values`:
/// the abci layer just CBOR-decodes the wire bytes into a `Value` and
/// hands the raw value down. Drive owns the parsing so a future
/// per-clause validation (e.g. forbidding operators in distinct mode)
/// can live next to the executors instead of being scattered across
/// abci handlers.
///
/// `Value::Null` (empty `where` field) → no clauses. Any other shape
/// must be an outer array of inner arrays-of-components.
///
/// After component parsing, the resulting clause list is run through
/// [`WhereClause::group_clauses`] — the same validator the regular
/// document-query path uses — to reject malformed shapes the count
/// path otherwise silently reduces:
///
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
/// rejecting the malformed request. `group_clauses` is the single
/// source of truth for what shapes the query stack as a whole
/// accepts; running it here aligns the count endpoint with the
/// regular document-query path's rejection contract.
///
/// Only the validation side-effect is consumed — the dispatcher
/// continues to operate on the parsed `Vec<WhereClause>` directly,
/// since the count-specific mode detection and index pickers
/// expect a flat list, not the equal-clauses/in-clause/range-clause
/// triple that `group_clauses` returns. (The regular query path's
/// `InternalClauses::extract_from_clauses` uses the triple; the
/// count path doesn't.)
pub fn where_clauses_from_value(
    value: &dpp::platform_value::Value,
    platform_version: &PlatformVersion,
) -> Result<Vec<WhereClause>, Error> {
    let clauses: Vec<WhereClause> = match value {
        dpp::platform_value::Value::Null => Vec::new(),
        dpp::platform_value::Value::Array(clauses) => clauses
            .iter()
            .map(|wc| match wc {
                dpp::platform_value::Value::Array(components) => {
                    WhereClause::from_components(components)
                }
                _ => Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                    "where clause must be an array".to_string(),
                ))),
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => {
            return Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                "where clause must be an array".to_string(),
            )));
        }
    };

    validate_and_canonicalize_where_clauses(clauses, platform_version)
}

/// Run the system-wide where-clause validator on a structured
/// `Vec<WhereClause>` and canonicalize same-field range pairs into
/// their `between*` form. Single source of truth for the
/// count-endpoint shape contract; called both from the legacy
/// CBOR-decoded entry [`where_clauses_from_value`] and from the
/// dispatcher's typed entry, [`Drive::execute_document_count_request`].
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
/// validation; for count queries we do it explicitly here so
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

/// Parse the decoded `order_by` value into structured [`OrderClause`]s.
///
/// Same shape as [`where_clauses_from_value`] for `order_by`:
/// `Value::Null` (empty `order_by` field on the wire) → no clauses;
/// any other shape must be an outer array of `[field, direction]`
/// inner arrays. Direction is `"asc"` / `"desc"` per
/// `OrderClause::from_components`.
pub fn order_clauses_from_value(
    value: &dpp::platform_value::Value,
) -> Result<Vec<OrderClause>, Error> {
    match value {
        dpp::platform_value::Value::Null => Ok(Vec::new()),
        dpp::platform_value::Value::Array(clauses) => clauses
            .iter()
            .map(|oc| match oc {
                dpp::platform_value::Value::Array(components) => {
                    // `OrderClause::from_components` returns
                    // `grovedb::Error`; wrap as drive's query-syntax
                    // error so the dispatcher's error contract stays
                    // uniform with the where-clause parser above.
                    OrderClause::from_components(components).map_err(|_e| {
                        Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                            "order_by clause must have [field, \"asc\"|\"desc\"] shape".to_string(),
                        ))
                    })
                }
                _ => Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
                    "order_by clause must be an array".to_string(),
                ))),
            })
            .collect(),
        _ => Err(Error::Query(QuerySyntaxError::InvalidFormatWhereClause(
            "order_by clause must be an array".to_string(),
        ))),
    }
}

impl Drive {
    /// Single entry point for the unified `GetDocumentsCount` request.
    ///
    /// Owns the whole pipeline:
    /// 1. [`DriveDocumentCountQuery::detect_mode`] classifies the
    ///    query shape from the where clauses + flags.
    /// 2. The matching `Drive::execute_document_count_*` per-mode
    ///    method picks an index and runs the executor.
    /// 3. The result is wrapped in [`DocumentCountResponse`] —
    ///    `Counts(...)` for no-proof modes, `Proof(...)` for proof
    ///    modes.
    ///
    /// Errors:
    /// - Mode-detection failures (multiple range clauses, range +
    ///   `In`, distinct on prove path, …) come back as
    ///   `Error::Query(QuerySyntaxError::InvalidWhereClauseComponents)`.
    /// - "No covering index" failures come back as
    ///   `Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty)`.
    /// - All other failures (grovedb, cost calculation, …) surface
    ///   as their native `Error` variants.
    ///
    /// The handler maps both `Error::Query(...)` cases to its own
    /// `QueryError::Query(...)` variant uniformly.
    pub fn execute_document_count_request(
        &self,
        request: DocumentCountRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentCountResponse, Error> {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;

        // Validate + canonicalize the structured `where_clauses` —
        // same rejections the regular document-query path runs,
        // applied here so the count endpoint's shape contract is
        // independent of whether the caller arrived via the CBOR-
        // shaped legacy path or the v1 typed-proto path. See
        // [`validate_and_canonicalize_where_clauses`]'s docstring
        // for the catalog of rejections / canonicalization rules.
        let where_clauses =
            validate_and_canonicalize_where_clauses(request.where_clauses, platform_version)?;
        let resolved_time_range_fields = request.resolved_time_range_fields;
        crate::query::validate_resolved_time_range_clause_shapes(
            &where_clauses,
            &resolved_time_range_fields,
        )?;
        let order_clauses = request.order_clauses;

        // Split-mode entry direction is whatever the first orderBy
        // clause specifies. Empty orderBy → ascending default. Used
        // by per-`In`-value, distinct-range no-proof, and
        // distinct-range prove paths; the `PointLookupProof` and
        // flat `Total` paths don't read it.
        let order_by_ascending = order_clauses.first().map(|c| c.ascending).unwrap_or(true);

        let mode = DriveDocumentCountQuery::detect_mode_versioned(
            &where_clauses,
            request.mode,
            request.prove,
            platform_version,
        )?;

        let contract_id = request.contract.id_ref().to_buffer();
        let document_type_name = request.document_type.name().to_string();

        match mode {
            DocumentCountMode::Total => {
                // Total mode → single aggregate. The executor returns
                // at most one entry (with empty key); collapse to
                // `Aggregate(count)` here so the response is a u64
                // with no per-key wrapping. Empty result (indexed
                // path doesn't exist yet) → `Aggregate(0)`.
                let entries = self.execute_document_count_total_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    &resolved_time_range_fields,
                    transaction,
                    platform_version,
                )?;
                let total = entries.first().and_then(|e| e.count).unwrap_or(0);
                Ok(DocumentCountResponse::Aggregate(total))
            }
            DocumentCountMode::PerInValue => {
                // |In| ≤ 100 is the structural bound; failsafe cap
                // keeps behavior independent of `default_query_limit`.
                // See [`super::MAX_LIMIT_AS_FAILSAFE`].
                let options = RangeCountOptions {
                    distinct: false, // ignored by PerInValue executor
                    limit: Some(super::MAX_LIMIT_AS_FAILSAFE),
                    order_by_ascending,
                };
                Ok(DocumentCountResponse::Entries(
                    self.execute_document_count_per_in_value_no_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        where_clauses,
                        &resolved_time_range_fields,
                        options,
                        transaction,
                        platform_version,
                    )?,
                ))
            }
            DocumentCountMode::RangeNoProof => {
                // Aggregate → failsafe cap (per-In fan-out bounded by
                // |In| ≤ 100); distinct walk → caller's limit with
                // `default_query_limit` fallback since range is
                // genuinely unbounded.
                let effective_limit = if request.mode.is_aggregate() {
                    super::MAX_LIMIT_AS_FAILSAFE
                } else {
                    let effective_limit = request
                        .limit
                        .unwrap_or(request.drive_config.default_query_limit as u32)
                        .min(request.drive_config.max_query_limit as u32);
                    // Fail closed instead of walking storage with a
                    // zero bound (grovedb treats `limit: 0` as "return
                    // nothing", which would masquerade as an empty
                    // result set). Mirrors the sum-side
                    // `effective_no_proof_distinct_limit` policy.
                    if effective_limit == 0 {
                        return Err(Error::Query(QuerySyntaxError::InvalidLimit(
                            "effective distinct COUNT limit must be greater than zero".to_string(),
                        )));
                    }
                    effective_limit
                };
                let options = RangeCountOptions {
                    distinct: request.mode.requires_distinct_walk(),
                    limit: Some(effective_limit),
                    order_by_ascending,
                };
                let entries = self.execute_document_count_range_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    &resolved_time_range_fields,
                    options,
                    transaction,
                    platform_version,
                )?;
                if request.mode.is_aggregate() {
                    // Aggregate mode: executor returns a single
                    // empty-key entry containing the sum (or empty
                    // vec if the path doesn't exist). Collapse to
                    // `Aggregate`.
                    let total = entries.first().and_then(|e| e.count).unwrap_or(0);
                    Ok(DocumentCountResponse::Aggregate(total))
                } else {
                    Ok(DocumentCountResponse::Entries(entries))
                }
            }
            DocumentCountMode::RangeProof => Ok(DocumentCountResponse::Proof(
                self.execute_document_count_range_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    &resolved_time_range_fields,
                    transaction,
                    platform_version,
                )?,
            )),
            DocumentCountMode::RangeDistinctProof => {
                // Validate-don't-clamp limit policy on the prove
                // path: client-side proof reconstruction needs the
                // exact same limit value the server applied to the
                // path query (so the merk-root recomputation
                // matches). Silent clamping would invisibly break
                // verification on any request with `limit >
                // max_query_limit`.
                //
                // **Limit fallback uses `crate::config::DEFAULT_QUERY_LIMIT`
                // (the compile-time constant), NOT
                // `drive_config.default_query_limit` (the
                // operator-tunable runtime value).** The SDK verifier
                // can't know an operator's tuned config, so any
                // operator who tuned `default_query_limit` away from
                // `DEFAULT_QUERY_LIMIT` would produce proofs whose
                // `SizedQuery::limit` byte-differs from the
                // verifier's reconstruction — silent verify failure
                // on a consensus-adjacent path. Anchoring the
                // fallback to the shared compile-time constant
                // removes that operator-tunable degree of freedom
                // from proof bytes entirely; the runtime
                // `default_query_limit` continues to govern no-proof
                // dispatch paths where there's no verifier to match.
                // `max_query_limit` still gates the request as a
                // DoS-protection knob (proofs never cross the
                // operator-set ceiling, but the ceiling itself doesn't
                // affect proof bytes — it only decides whether the
                // request gets served).
                let effective_limit = request
                    .limit
                    .unwrap_or(crate::config::DEFAULT_QUERY_LIMIT as u32);
                if effective_limit > request.drive_config.max_query_limit as u32 {
                    return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                        "limit {} exceeds max_query_limit {} on the prove + \
                         distinct-walk path (GROUP BY a range field); reduce the \
                         requested limit or use prove = false",
                        effective_limit, request.drive_config.max_query_limit
                    ))));
                }
                let limit_u16 = effective_limit as u16;
                // Default to ascending if the request didn't specify
                // — matches the no-proof default. The verifier reads
                // the same field to reconstruct the matching path
                // query (see SDK's `FromProof<DocumentQuery>` impl
                // for `DocumentSplitCounts`); both sides MUST land
                // on the same `left_to_right` value or the merk-root
                // recomputation fails.
                let left_to_right = order_by_ascending;
                Ok(DocumentCountResponse::Proof(
                    self.execute_document_count_range_distinct_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        where_clauses,
                        &resolved_time_range_fields,
                        limit_u16,
                        left_to_right,
                        transaction,
                        platform_version,
                    )?,
                ))
            }
            DocumentCountMode::PointLookupProof => Ok(DocumentCountResponse::Proof(
                self.execute_document_count_point_lookup_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    where_clauses,
                    &resolved_time_range_fields,
                    transaction,
                    platform_version,
                )?,
            )),
            DocumentCountMode::RangeAggregateCarrierProof => {
                // Validate-don't-clamp limit policy on the prove path
                // (same rationale as `RangeDistinctProof` above): the
                // verifier reconstructs the SizedQuery's `limit` byte-
                // identically, so silent clamping would invisibly
                // break verification.
                //
                // Two shape-dependent rules apply here:
                //
                // - **In-outer carrier (G7):** the caller's `|In|`
                //   already bounds the result. `SizedQuery::limit`
                //   stays `None`; if the caller passed a non-`None`
                //   `limit`, reject — there's no use case for a sub-
                //   `|In|` limit on this path, and accepting it would
                //   silently change which In-branches appear in the
                //   proof.
                //
                // - **Range-outer carrier (G8):** the platform
                //   enforces a max outer-walk cap of
                //   [`super::MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT`]
                //   on how many outer-range matches the carrier walks.
                //   Caller may pass a smaller `limit` to truncate the
                //   walk further; passing a larger one is rejected.
                //   If the caller passes `None`, the platform default
                //   (the cap itself) is used.
                let has_outer_range = where_clauses
                    .iter()
                    .filter(|wc| DriveDocumentCountQuery::is_range_operator(wc.operator))
                    .count()
                    == 2;
                let effective_limit = if has_outer_range {
                    match request.limit {
                        None => Some(super::MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT),
                        Some(n) => {
                            if n > super::MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT as u32 {
                                return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                                    "carrier-aggregate range-outer queries (e.g. \
                                         `outer_range_field > X AND inner_acor_field > \
                                         Y` with `group_by = [outer_range_field]`) cap \
                                         the outer walk at {} entries (compile-time \
                                         constant `MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT`); \
                                         got limit = {}. Pass a value ≤ {} or omit \
                                         `limit` to use the default.",
                                    super::MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT,
                                    n,
                                    super::MAX_CARRIER_AGGREGATE_OUTER_RANGE_LIMIT,
                                ))));
                            }
                            if n == 0 {
                                return Err(Error::Query(QuerySyntaxError::InvalidLimit(
                                    "carrier-aggregate range-outer queries require limit \
                                     ≥ 1; got limit = 0"
                                        .to_string(),
                                )));
                            }
                            Some(n as u16)
                        }
                    }
                } else {
                    if let Some(n) = request.limit {
                        return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                            "carrier-aggregate In-outer queries (e.g. `outer_in_field IN \
                             [...] AND inner_acor_field > Y` with `group_by = \
                             [outer_in_field]`) don't accept `limit` — the In array's \
                             length already bounds the result. Got limit = {n}.",
                        ))));
                    }
                    None
                };
                // Outer-walk direction: ascending by default (the
                // grovedb invariant for serialized-key carriers), or
                // descending when the caller's `order_by` first
                // clause is `desc`. Carried byte-identically through
                // `Query::left_to_right` so the verifier rebuilds the
                // exact same `PathQuery` — same load-bearing pattern
                // as the `RangeDistinctProof` arm above.
                let left_to_right = order_by_ascending;
                Ok(DocumentCountResponse::Proof(
                    self.execute_document_count_range_aggregate_carrier_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        where_clauses,
                        &resolved_time_range_fields,
                        effective_limit,
                        left_to_right,
                        transaction,
                        platform_version,
                    )?,
                ))
            }
        }
    }
}
