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
/// handler from a `GetDocumentsCountRequestV0` after CBOR-decoding +
/// contract lookup; drive owns everything past this point including
/// mode detection, index picking, and per-mode dispatch.
///
/// `raw_where_value` and `raw_order_by_value` arrive as CBOR-decoded
/// `Value`s and the dispatcher parses them once into structured
/// `Vec<WhereClause>` / `Vec<OrderClause>` for mode detection +
/// per-mode executors. None of the count executors consume the raw
/// `Value` form — the structured parse is the single source of
/// truth past the dispatcher entry point.
pub struct DocumentCountRequest<'a> {
    /// Live contract (already loaded by the handler).
    pub contract: &'a dpp::data_contract::DataContract,
    /// Resolved document type within `contract`.
    pub document_type: DocumentTypeRef<'a>,
    /// Decoded `where` value as it came off the wire (after CBOR
    /// decode). The dispatcher parses this into `Vec<WhereClause>`
    /// once (`where_clauses_from_value`) for every downstream
    /// consumer — mode detection, index picking, and the per-mode
    /// executors all operate on the structured form.
    ///
    /// Mirrors how the regular `query_documents_v0` handler
    /// delegates where-clause decomposition to drive: the abci
    /// layer just CBOR-decodes and hands the raw value down.
    pub raw_where_value: dpp::platform_value::Value,
    /// Decoded `order_by` value as it came off the wire. Parsed
    /// once via `order_clauses_from_value` into
    /// `Vec<OrderClause>`. The first clause's direction governs
    /// split-mode entry ordering (per-`In`-value / per-distinct-
    /// value-in-range) and, on the `RangeDistinctProof` prove
    /// path, is part of the path-query bytes the SDK reconstructs
    /// to verify the proof. `PointLookupProof` and the no-proof
    /// `Total` / `PerInValue` paths don't read order_by.
    ///
    /// `Value::Null` (empty `order_by` field on the wire) → no
    /// clauses. The dispatcher synthesizes a default direction of
    /// "ascending" for split-mode response ordering when no clauses
    /// are present.
    pub raw_order_by_value: dpp::platform_value::Value,
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
/// - Multiple `In` clauses (`MultipleInClauses`).
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
fn where_clauses_from_value(value: &dpp::platform_value::Value) -> Result<Vec<WhereClause>, Error> {
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

    // Run the parsed clauses through the system-wide validator.
    // The returned triple is discarded; we only care about the
    // validation errors — see this function's docstring for the
    // catalog of rejections this enables on the count endpoint.
    //
    // Exception: `MultipleRangeClauses` is intentionally tolerated
    // here. The regular-query parser rejects two ranges on
    // different fields wholesale (its callers expect
    // `(equal_clauses, in_clause, range_clause)` triples), but the
    // count-query path accepts the carrier-aggregate shape
    // (`outer_range + inner_ACOR_range` on different fields, e.g.
    // G8). Structural validation for that shape lives in
    // [`DriveDocumentCountQuery::detect_mode`] (which knows about
    // `CountMode::GroupByRange`-with-two-ranges and routes to
    // `DocumentCountMode::RangeAggregateCarrierProof`); replicating
    // it here would be redundant.
    match WhereClause::group_clauses(&clauses) {
        Ok(_) => {}
        Err(Error::Query(QuerySyntaxError::MultipleRangeClauses(_))) => {}
        Err(e) => return Err(e),
    }
    Ok(clauses)
}

/// Parse the decoded `order_by` value into structured [`OrderClause`]s.
///
/// Same shape as [`where_clauses_from_value`] for `order_by`:
/// `Value::Null` (empty `order_by` field on the wire) → no clauses;
/// any other shape must be an outer array of `[field, direction]`
/// inner arrays. Direction is `"asc"` / `"desc"` per
/// `OrderClause::from_components`.
fn order_clauses_from_value(value: &dpp::platform_value::Value) -> Result<Vec<OrderClause>, Error> {
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

        // Parse where clauses out of the raw decoded `Value` once,
        // then thread them through the per-mode executors. Mirrors
        // how the regular `query_documents_v0` handler delegates this
        // to `DriveDocumentQuery::from_decomposed_values` —
        // where-clause decomposition is a drive concern, not abci's.
        let where_clauses = where_clauses_from_value(&request.raw_where_value)?;
        let order_clauses = order_clauses_from_value(&request.raw_order_by_value)?;

        // Split-mode entry direction is whatever the first orderBy
        // clause specifies. Empty orderBy → ascending default. Used
        // by per-`In`-value, distinct-range no-proof, and
        // distinct-range prove paths; the `PointLookupProof` and
        // flat `Total` paths don't read it.
        let order_by_ascending = order_clauses.first().map(|c| c.ascending).unwrap_or(true);

        let mode =
            DriveDocumentCountQuery::detect_mode(&where_clauses, request.mode, request.prove)?;

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
                    request
                        .limit
                        .unwrap_or(request.drive_config.default_query_limit as u32)
                        .min(request.drive_config.max_query_limit as u32)
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
                    transaction,
                    platform_version,
                )?,
            )),
            DocumentCountMode::RangeAggregateCarrierProof => {
                // Validate-don't-clamp limit policy on the prove path
                // (same rationale as `RangeDistinctProof` above): the
                // verifier reconstructs the SizedQuery's `limit` byte-
                // identically, so silent clamping would invisibly
                // break verification. `limit` is meaningful only for
                // the outer-Range carrier shape (G8); for the
                // outer-In shape (G7) the caller's |In| already
                // bounds the result and `limit` is typically unset.
                let effective_limit = match request.limit {
                    Some(n) => {
                        if n > request.drive_config.max_query_limit as u32 {
                            return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                                "limit {} exceeds max_query_limit {} on the prove + carrier-\
                                 aggregate path; reduce the requested limit or omit it for the \
                                 In-outer shape",
                                n, request.drive_config.max_query_limit
                            ))));
                        }
                        Some(n as u16)
                    }
                    None => None,
                };
                Ok(DocumentCountResponse::Proof(
                    self.execute_document_count_range_aggregate_carrier_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        where_clauses,
                        effective_limit,
                        transaction,
                        platform_version,
                    )?,
                ))
            }
        }
    }
}
