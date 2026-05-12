//! v1 handler for `getDocuments` — SQL-shaped unified surface
//! covering `getDocuments` and `getDocumentsCount` under a single
//! request type with `select`, `group_by`, and `having` clauses.
//!
//! ## What this handler is
//!
//! **Pure rewiring**, not new capability. Every supported request
//! shape translates to an existing v0 (`query_documents_v0`) or
//! v0-count (`query_documents_count_v0`) handler invocation and
//! produces the same proof bytes / response data. The v1 surface
//! just makes the SQL semantics explicit on the wire so callers
//! don't have to reverse-engineer "this where clause shape happens
//! to produce per-value entries."
//!
//! ## What it rejects
//!
//! Every request shape outside the v0 / v0-count capability surface
//! returns [`QuerySyntaxError::Unsupported`] with `"… is not yet
//! implemented"` text. The error variant carries a `String` so the
//! exact rejected shape reaches the caller without prose-parsing,
//! and the message wording signals **future capability**, not
//! malformed request — clients can keep these requests around in
//! code and they'll start working once the capability lands without
//! a wire-format change. See the message-level docstring on
//! `GetDocumentsRequestV1` in `platform.proto` for the full Phase 1
//! supported/rejected shape table.
//!
//! ## Why the indirection
//!
//! Forwarding to the v0 handlers means: (a) zero risk of v1
//! drifting from v0's execution semantics, (b) the proof bytes
//! produced by v1 and the corresponding v0/v0-count call are
//! byte-identical for the same logical query — important once SDKs
//! migrate, since the proof verifier doesn't need to know which
//! wire path produced the bytes, and (c) the rejection table is the
//! only "new" code, which is exactly the surface that needs review.

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_documents_count_request::GetDocumentsCountRequestV0;
use dapi_grpc::platform::v0::get_documents_count_response::{
    get_documents_count_response_v0, GetDocumentsCountResponseV0,
};
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start as RequestV0Start;
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::{
    Select, Start as RequestV1Start,
};
use dapi_grpc::platform::v0::get_documents_request::{
    GetDocumentsRequestV0, GetDocumentsRequestV1,
};
use dapi_grpc::platform::v0::get_documents_response::{
    get_documents_response_v0, get_documents_response_v1, GetDocumentsResponseV1,
};
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use drive::error::query::QuerySyntaxError;
use drive::query::{WhereClause, WhereOperator};

/// Build a `QuerySyntaxError::Unsupported` carrying a stable
/// "<feature> is not yet implemented" message. The wording is
/// deliberate — Phase 1 of v1 publishes a SQL-shaped surface that
/// the server only partially implements; the rejected shapes signal
/// future capability, not malformed requests, and callers can keep
/// the request structure unchanged when the capability lands.
fn not_yet_implemented(feature: &str) -> QueryError {
    QueryError::Query(QuerySyntaxError::Unsupported(format!(
        "{} is not yet implemented",
        feature
    )))
}

/// Parse the raw CBOR-encoded `where` bytes into structured
/// [`WhereClause`]s. v1 needs the structured form to enforce
/// `group_by` ↔ where-field cross-checks before delegating; v0
/// re-parses them on its side, so the parse happens twice for v1
/// requests. The overhead is negligible (CBOR decode of ≤ a few
/// clauses) and lets the v0 handler stay verbatim.
fn decode_where_clauses(where_bytes: &[u8]) -> Result<Vec<WhereClause>, QueryError> {
    if where_bytes.is_empty() {
        return Ok(Vec::new());
    }
    let value: Value = ciborium::de::from_reader(where_bytes).map_err(|_| {
        QueryError::Query(QuerySyntaxError::DeserializationError(
            "unable to decode 'where' query from cbor".to_string(),
        ))
    })?;
    let array = match value {
        Value::Array(a) => a,
        Value::Null => return Ok(Vec::new()),
        _ => {
            return Err(QueryError::Query(
                QuerySyntaxError::InvalidFormatWhereClause("where clause must be an array"),
            ));
        }
    };
    let mut clauses = Vec::with_capacity(array.len());
    for entry in array {
        let components = match entry {
            Value::Array(c) => c,
            _ => {
                return Err(QueryError::Query(
                    QuerySyntaxError::InvalidFormatWhereClause("where clause must be an array"),
                ));
            }
        };
        let clause = WhereClause::from_components(&components).map_err(|e| {
            QueryError::Query(QuerySyntaxError::InvalidFormatWhereClause(Box::leak(
                format!("invalid where clause components: {e}").into_boxed_str(),
            )))
        })?;
        clauses.push(clause);
    }
    Ok(clauses)
}

/// Validate the `select` × `group_by` × `having` combination
/// against the Phase 1 supported-shape table. Returns:
/// - `Ok(true)` if the query should route to v0's documents path.
/// - `Ok(false)` if the query should route to v0-count's path.
/// - `Err(...)` for any rejected shape (HAVING, GROUP BY with
///   DOCUMENTS, group_by field not matching an `In`/range clause,
///   `group_by.len() > 2`, etc.).
///
/// Also extracts the value of `return_distinct_counts_in_range` to
/// pass down to v0-count: empty `group_by` → false (aggregate),
/// non-empty → true (entries).
fn validate_and_route(
    request_v1: &GetDocumentsRequestV1,
    where_clauses: &[WhereClause],
) -> Result<RoutingDecision, QueryError> {
    let select = Select::try_from(request_v1.select).map_err(|_| {
        not_yet_implemented(&format!(
            "select value {} (not in the Select enum)",
            request_v1.select
        ))
    })?;

    if !request_v1.having.is_empty() {
        return Err(not_yet_implemented("HAVING clause"));
    }

    match select {
        Select::Documents => {
            if !request_v1.group_by.is_empty() {
                return Err(not_yet_implemented(
                    "GROUP BY with SELECT DOCUMENTS (use SELECT COUNT with GROUP BY \
                     for per-group counts, or SELECT DOCUMENTS without GROUP BY for \
                     matched documents)",
                ));
            }
            Ok(RoutingDecision::Documents)
        }
        Select::Count => {
            // Identify the In and range fields on the where clauses
            // — used to validate group_by membership and to decide
            // whether v0-count will return aggregate or entries.
            let in_field: Option<&str> = where_clauses
                .iter()
                .find(|wc| wc.operator == WhereOperator::In)
                .map(|wc| wc.field.as_str());
            let range_field: Option<&str> = where_clauses
                .iter()
                .find(|wc| {
                    matches!(
                        wc.operator,
                        WhereOperator::GreaterThan
                            | WhereOperator::GreaterThanOrEquals
                            | WhereOperator::LessThan
                            | WhereOperator::LessThanOrEquals
                            | WhereOperator::Between
                            | WhereOperator::BetweenExcludeBounds
                            | WhereOperator::BetweenExcludeLeft
                            | WhereOperator::BetweenExcludeRight
                            | WhereOperator::StartsWith
                    )
                })
                .map(|wc| wc.field.as_str());

            match request_v1.group_by.as_slice() {
                // Empty GROUP BY → aggregate count.
                [] => Ok(RoutingDecision::CountAggregate),

                // Single-field GROUP BY: must match the In or range
                // field. Anything else is "not yet implemented" — a
                // bare `GROUP BY x` without a matching where clause
                // requires walking a property-name `ProvableCountTree`,
                // which is a new server-side primitive we haven't
                // wired here. See platform.proto's message-level
                // docstring for the full table.
                [field] => {
                    if Some(field.as_str()) == in_field {
                        Ok(RoutingDecision::CountEntriesViaInField)
                    } else if Some(field.as_str()) == range_field {
                        Ok(RoutingDecision::CountEntriesViaRangeField)
                    } else {
                        Err(not_yet_implemented(&format!(
                            "GROUP BY on field '{}' which is not constrained by an \
                             `In` or range where clause",
                            field
                        )))
                    }
                }

                // Two-field GROUP BY: only the existing compound
                // `(In + range)` shape is supported, with the In
                // field first and range field second (the order the
                // server emits entries in via the In-as-outer-key,
                // range-as-subquery merk walk).
                [first, second] => {
                    if Some(first.as_str()) == in_field && Some(second.as_str()) == range_field {
                        Ok(RoutingDecision::CountEntriesViaCompound)
                    } else {
                        Err(not_yet_implemented(
                            "two-field GROUP BY outside the `(In, range)` compound \
                             shape (the existing compound count path orders entries \
                             as `(in_key, key)`; other orderings would need a new \
                             merk walk)",
                        ))
                    }
                }

                // Three or more fields.
                _ => Err(not_yet_implemented("GROUP BY with more than two fields")),
            }
        }
    }
}

/// Outcome of `validate_and_route` — names the v0-side dispatch
/// path the v1 request should be translated into.
enum RoutingDecision {
    /// `select = DOCUMENTS, group_by = []` → forward to
    /// `query_documents_v0`. Response wraps the v0 `Documents` or
    /// `Proof` into v1's matching oneof.
    Documents,
    /// `select = COUNT, group_by = []` → forward to
    /// `query_documents_count_v0` with `return_distinct_counts_in_range
    /// = false`. For modes that naturally return entries (PerInValue
    /// on `In + no range`), the v1 handler sums them server-side
    /// into a single aggregate before wrapping the response.
    CountAggregate,
    /// `select = COUNT, group_by = [in_field]` → forward to
    /// v0-count; v0's PerInValue already returns entries. Response
    /// re-wraps the entries as v1's `CountResults.Entries`.
    CountEntriesViaInField,
    /// `select = COUNT, group_by = [range_field]` → forward to
    /// v0-count with `return_distinct_counts_in_range = true`.
    /// Response re-wraps the per-distinct-value entries.
    CountEntriesViaRangeField,
    /// `select = COUNT, group_by = [in_field, range_field]` →
    /// forward to v0-count with `return_distinct_counts_in_range =
    /// true`; v0's compound dispatch returns `(in_key, key)`
    /// entries. v1 re-wraps unchanged.
    CountEntriesViaCompound,
}

impl<C> Platform<C> {
    pub(super) fn query_documents_v1(
        &self,
        request_v1: GetDocumentsRequestV1,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        // Decode the where clauses once for shape validation. v0
        // decodes them again on its side — the duplication is
        // acceptable for the clarity of a "v1 = pure rewiring" PR;
        // a follow-up can share the parse if profiling shows it
        // matters.
        let where_clauses = match decode_where_clauses(&request_v1.r#where) {
            Ok(c) => c,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        let routing = match validate_and_route(&request_v1, &where_clauses) {
            Ok(r) => r,
            Err(e) => return Ok(QueryValidationResult::new_with_error(e)),
        };

        match routing {
            RoutingDecision::Documents => {
                self.dispatch_documents_v1_to_v0(request_v1, platform_state, platform_version)
            }
            RoutingDecision::CountAggregate => self.dispatch_count_v1_to_v0(
                request_v1,
                /* return_distinct_counts_in_range = */ false,
                /* expect_aggregate = */ true,
                platform_state,
                platform_version,
            ),
            RoutingDecision::CountEntriesViaInField => self.dispatch_count_v1_to_v0(
                request_v1,
                /* return_distinct_counts_in_range = */ false,
                /* expect_aggregate = */ false,
                platform_state,
                platform_version,
            ),
            RoutingDecision::CountEntriesViaRangeField
            | RoutingDecision::CountEntriesViaCompound => self.dispatch_count_v1_to_v0(
                request_v1,
                /* return_distinct_counts_in_range = */ true,
                /* expect_aggregate = */ false,
                platform_state,
                platform_version,
            ),
        }
    }

    /// Forward a `select = DOCUMENTS` request through the v0
    /// handler. The v1 → v0 request translation is straight 1:1 —
    /// v1's DOCUMENTS shape with empty `group_by`/`having` is a
    /// superset of v0 by only one field (the SQL-shaped knobs that
    /// are guaranteed empty here).
    fn dispatch_documents_v1_to_v0(
        &self,
        request_v1: GetDocumentsRequestV1,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        let start = request_v1.start.map(|s| match s {
            RequestV1Start::StartAfter(b) => RequestV0Start::StartAfter(b),
            RequestV1Start::StartAt(b) => RequestV0Start::StartAt(b),
        });
        // `limit` is `optional uint32` on v1 vs unwrapped `uint32`
        // (default 0) on v0. Unset on v1 → 0 on v0 (v0 reads `0` as
        // "use the server's `default_query_limit`"). Mirroring the
        // existing v0 semantics keeps the proof bytes identical.
        let request_v0 = GetDocumentsRequestV0 {
            data_contract_id: request_v1.data_contract_id,
            document_type: request_v1.document_type,
            r#where: request_v1.r#where,
            order_by: request_v1.order_by,
            limit: request_v1.limit.unwrap_or(0),
            prove: request_v1.prove,
            start,
        };
        let result = self.query_documents_v0(request_v0, platform_state, platform_version)?;
        Ok(result.map(translate_documents_v0_to_v1))
    }

    /// Forward a `select = COUNT` request through the v0 count
    /// handler. The v1 → v0-count translation differs from the
    /// documents one in two ways:
    /// - `start` is rejected at the v1 layer (no concept of "start
    ///   after this aggregate" for a single u64 or a per-key
    ///   entries map paginated by serialized key) — see below.
    /// - When `expect_aggregate = true` and the v0-count handler
    ///   returns `Entries` (PerInValue mode for `In + no range`),
    ///   the v1 handler sums them server-side before emitting a
    ///   single aggregate on the wire. The wasted entry
    ///   construction is acceptable for PR 1; a future
    ///   optimization can push the aggregation into drive.
    fn dispatch_count_v1_to_v0(
        &self,
        request_v1: GetDocumentsRequestV1,
        return_distinct_counts_in_range: bool,
        expect_aggregate: bool,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetDocumentsResponseV1>, Error> {
        // `start` on a COUNT request — v0-count has no `start`
        // field and the underlying count executors don't read one.
        // For aggregate this is meaningless; for per-key entries
        // pagination happens by narrowing the range (the documented
        // contract on `RangeCountOptions::limit`). Reject explicitly
        // so callers see the divergence at request time.
        if request_v1.start.is_some() {
            return Ok(QueryValidationResult::new_with_error(not_yet_implemented(
                "start_after / start_at with SELECT COUNT (paginate by narrowing the \
                 range clause itself)",
            )));
        }

        let request_v0_count = GetDocumentsCountRequestV0 {
            data_contract_id: request_v1.data_contract_id,
            document_type: request_v1.document_type,
            r#where: request_v1.r#where,
            return_distinct_counts_in_range,
            order_by: request_v1.order_by,
            limit: request_v1.limit,
            prove: request_v1.prove,
        };
        let result =
            self.query_documents_count_v0(request_v0_count, platform_state, platform_version)?;

        // Translate the v0-count response into v1 shape. For
        // `expect_aggregate = true` we additionally sum any
        // `Entries` payload into a single `AggregateCount` —
        // covers the `select = COUNT, group_by = [], In, no range,
        // no prove` case where v0-count's PerInValue emits one
        // entry per In value.
        Ok(result.map(|response_v0| translate_count_v0_to_v1(response_v0, expect_aggregate)))
    }
}

/// Translate a v0 `GetDocumentsResponseV0` into v1's response
/// envelope. v1's `Documents` and `Proof` variants point at the v0
/// types directly (Protobuf nested-type reference), so the
/// translation is a oneof rewrap; no field copying needed.
fn translate_documents_v0_to_v1(
    response_v0: dapi_grpc::platform::v0::get_documents_response::GetDocumentsResponseV0,
) -> GetDocumentsResponseV1 {
    let metadata = response_v0.metadata;
    let result = match response_v0.result {
        Some(get_documents_response_v0::Result::Documents(docs)) => {
            Some(get_documents_response_v1::Result::Documents(docs))
        }
        Some(get_documents_response_v0::Result::Proof(proof)) => {
            Some(get_documents_response_v1::Result::Proof(proof))
        }
        None => None,
    };
    GetDocumentsResponseV1 { result, metadata }
}

/// Test-only: expose the routing decision for unit tests without
/// needing a full `Platform` setup. The same function is called
/// from the production handler — tests here pin the rejection
/// table; end-to-end tests below pin the full handler wiring.
#[cfg(test)]
pub(super) fn validate_and_route_for_tests(
    request_v1: &GetDocumentsRequestV1,
    where_clauses: &[WhereClause],
) -> Result<&'static str, QueryError> {
    validate_and_route(request_v1, where_clauses).map(|d| match d {
        RoutingDecision::Documents => "documents",
        RoutingDecision::CountAggregate => "count_aggregate",
        RoutingDecision::CountEntriesViaInField => "count_entries_via_in_field",
        RoutingDecision::CountEntriesViaRangeField => "count_entries_via_range_field",
        RoutingDecision::CountEntriesViaCompound => "count_entries_via_compound",
    })
}

/// Translate a v0-count `GetDocumentsCountResponseV0` into v1's
/// `GetDocumentsResponseV1` envelope. Three cases:
/// - `Proof` → forward as-is into v1's `Proof` variant.
/// - `Counts(AggregateCount(n))` → forward.
/// - `Counts(Entries(es))`:
///   - If `expect_aggregate` (v1 caller asked for aggregate but v0
///     PerInValue returned per-In entries), sum the entry counts
///     into a single `AggregateCount`.
///   - Otherwise forward as-is.
fn translate_count_v0_to_v1(
    response_v0: GetDocumentsCountResponseV0,
    expect_aggregate: bool,
) -> GetDocumentsResponseV1 {
    let metadata = response_v0.metadata;
    let result = match response_v0.result {
        Some(get_documents_count_response_v0::Result::Counts(counts)) => {
            let variant = match counts.variant {
                Some(get_documents_count_response_v0::count_results::Variant::AggregateCount(
                    n,
                )) => {
                    Some(get_documents_count_response_v0::count_results::Variant::AggregateCount(n))
                }
                Some(get_documents_count_response_v0::count_results::Variant::Entries(entries)) => {
                    if expect_aggregate {
                        // Sum entries into a single aggregate — the
                        // v1-handler-side fan-in for the
                        // `(select=COUNT, group_by=[], In, no range,
                        // no prove)` shape. `saturating_add` on the
                        // off-chance an operator-misconfigured count
                        // tree exceeds u64; the realistic ceiling
                        // is `|In| × max_per-branch-count`, well
                        // under u64.
                        let total: u64 = entries
                            .entries
                            .iter()
                            .map(|e| e.count)
                            .fold(0u64, |a, b| a.saturating_add(b));
                        Some(
                            get_documents_count_response_v0::count_results::Variant::AggregateCount(
                                total,
                            ),
                        )
                    } else {
                        Some(
                            get_documents_count_response_v0::count_results::Variant::Entries(
                                entries,
                            ),
                        )
                    }
                }
                None => None,
            };
            Some(get_documents_response_v1::Result::Counts(
                get_documents_count_response_v0::CountResults { variant },
            ))
        }
        Some(get_documents_count_response_v0::Result::Proof(proof)) => {
            Some(get_documents_response_v1::Result::Proof(proof))
        }
        None => None,
    };
    GetDocumentsResponseV1 { result, metadata }
}

#[cfg(test)]
mod tests {
    //! Tests for the v1 `getDocuments` handler — pure rewiring of
    //! v0 documents + v0 count under a SQL-shaped surface. Two test
    //! kinds:
    //!
    //! - **Rejection-table unit tests** (`reject_*`): drive
    //!   `validate_and_route` directly with hand-built v1 requests
    //!   and assert the right `Unsupported("… is not yet
    //!   implemented")` error fires. No `Platform` setup — fast,
    //!   focused on the rejection contract.
    //!
    //! - **End-to-end parity tests** (`e2e_*`): build a real
    //!   contract + documents, issue equivalent v0 and v1 requests,
    //!   assert the responses are functionally identical. Pins that
    //!   the v1 → v0 forwarding doesn't drift.
    //!
    //! The rejection arms are the only "new" logic in v1 — the
    //! happy paths all delegate to existing v0 handlers — so the
    //! rejection tests carry the bulk of the test surface here.
    use super::*;
    use crate::query::tests::{setup_platform, store_data_contract, store_document};
    use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::{
        Select as V1Select, Start as V1Start,
    };
    use dapi_grpc::platform::v0::get_documents_request::GetDocumentsRequestV1;
    use dpp::dashcore::Network;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::platform_value::platform_value;
    use drive::error::query::QuerySyntaxError;

    /// Helper: minimal v1 request with empty `where`, `order_by`,
    /// `group_by`, `having`. Test-specific fields can be set via
    /// struct-update syntax.
    fn empty_v1_request() -> GetDocumentsRequestV1 {
        GetDocumentsRequestV1 {
            data_contract_id: vec![0u8; 32],
            document_type: "widget".to_string(),
            r#where: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            start: None,
            prove: false,
            select: V1Select::Documents as i32,
            group_by: Vec::new(),
            having: Vec::new(),
        }
    }

    fn assert_not_yet_implemented(
        result: Result<&'static str, QueryError>,
        expected_feature: &str,
    ) {
        match result {
            Err(QueryError::Query(QuerySyntaxError::Unsupported(msg))) => {
                assert!(
                    msg.contains(expected_feature) && msg.contains("not yet implemented"),
                    "expected message containing '{}' and 'not yet implemented', got: {}",
                    expected_feature,
                    msg
                );
            }
            other => panic!(
                "expected QueryError::Query(Unsupported) for '{}', got {:?}",
                expected_feature, other
            ),
        }
    }

    /// `HAVING` is wire-reserved but always rejected in Phase 1.
    /// Pins that any non-empty `having` blob fires before any other
    /// validation (so callers see the HAVING-specific message
    /// regardless of what else is in the request).
    #[test]
    fn reject_having_non_empty() {
        let request = GetDocumentsRequestV1 {
            having: vec![0x01, 0x02], // any non-empty payload
            ..empty_v1_request()
        };
        assert_not_yet_implemented(validate_and_route_for_tests(&request, &[]), "HAVING clause");
    }

    /// `SELECT DOCUMENTS` doesn't take `GROUP BY` — SQL has no
    /// meaningful `SELECT *, … GROUP BY field` without an aggregate
    /// or a `DISTINCT ON`, neither of which v1 ships.
    #[test]
    fn reject_group_by_with_documents() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Documents as i32,
            group_by: vec!["color".to_string()],
            ..empty_v1_request()
        };
        assert_not_yet_implemented(
            validate_and_route_for_tests(&request, &[]),
            "GROUP BY with SELECT DOCUMENTS",
        );
    }

    /// `GROUP BY field` where `field` isn't constrained by an `In`
    /// or range where clause requires a new server-side primitive
    /// (walking the property-name `ProvableCountTree`'s children
    /// without a covering prefix). Phase 1 doesn't ship that.
    #[test]
    fn reject_group_by_field_not_in_where_clauses() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["color".to_string()],
            ..empty_v1_request()
        };
        // `where_clauses = []` → group_by field 'color' matches
        // neither in_field nor range_field.
        assert_not_yet_implemented(
            validate_and_route_for_tests(&request, &[]),
            "GROUP BY on field 'color' which is not constrained",
        );
    }

    /// More than two `group_by` fields requires multi-level
    /// CountEntry serialization that's a wire format change. Phase 1
    /// caps at two.
    #[test]
    fn reject_group_by_more_than_two_fields() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            ..empty_v1_request()
        };
        assert_not_yet_implemented(
            validate_and_route_for_tests(&request, &[]),
            "GROUP BY with more than two fields",
        );
    }

    /// Two-field `group_by` only matches the existing compound
    /// `(In, range)` shape (where the In is on a prefix and the
    /// range is on the terminator). Reordering or any other pair
    /// hits this rejection.
    #[test]
    fn reject_two_field_group_by_outside_compound_shape() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            // Order reversed: range field first, In field second —
            // not the (In, range) compound shape the server emits.
            group_by: vec!["color".to_string(), "brand".to_string()],
            ..empty_v1_request()
        };
        let where_clauses = vec![
            WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::In,
                value: platform_value!(["acme", "contoso"]),
            },
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: platform_value!("blue"),
            },
        ];
        assert_not_yet_implemented(
            validate_and_route_for_tests(&request, &where_clauses),
            "two-field GROUP BY outside the `(In, range)` compound shape",
        );
    }

    /// Empty `group_by` + `SELECT COUNT` routes to the aggregate
    /// path regardless of where-clause shape. The routing decision
    /// here doesn't peek at the where clauses — they're handled
    /// downstream by v0-count's dispatcher.
    #[test]
    fn accept_count_with_empty_group_by_routes_to_aggregate() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            ..empty_v1_request()
        };
        assert_eq!(
            validate_and_route_for_tests(&request, &[]).unwrap(),
            "count_aggregate"
        );
    }

    /// `group_by=[in_field]` with an `In` clause on the same field
    /// routes to v0-count's PerInValue entries path.
    #[test]
    fn accept_count_group_by_in_field_routes_to_in_entries() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["brand".to_string()],
            ..empty_v1_request()
        };
        let where_clauses = vec![WhereClause {
            field: "brand".to_string(),
            operator: WhereOperator::In,
            value: platform_value!(["acme", "contoso"]),
        }];
        assert_eq!(
            validate_and_route_for_tests(&request, &where_clauses).unwrap(),
            "count_entries_via_in_field"
        );
    }

    /// `group_by=[range_field]` with a range clause on the same
    /// field routes to v0-count's RangeDistinct entries path
    /// (equivalent to v0's `return_distinct_counts_in_range = true`).
    #[test]
    fn accept_count_group_by_range_field_routes_to_range_entries() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["color".to_string()],
            ..empty_v1_request()
        };
        let where_clauses = vec![WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: platform_value!("blue"),
        }];
        assert_eq!(
            validate_and_route_for_tests(&request, &where_clauses).unwrap(),
            "count_entries_via_range_field"
        );
    }

    /// Compound `(In, range)` `group_by` routes to v0-count's
    /// compound distinct path (the existing
    /// `return_distinct_counts_in_range = true` + In-on-prefix
    /// shape that emits `(in_key, key)` entries).
    #[test]
    fn accept_count_group_by_compound_routes_to_compound_entries() {
        let request = GetDocumentsRequestV1 {
            select: V1Select::Count as i32,
            group_by: vec!["brand".to_string(), "color".to_string()],
            ..empty_v1_request()
        };
        let where_clauses = vec![
            WhereClause {
                field: "brand".to_string(),
                operator: WhereOperator::In,
                value: platform_value!(["acme", "contoso"]),
            },
            WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: platform_value!("blue"),
            },
        ];
        assert_eq!(
            validate_and_route_for_tests(&request, &where_clauses).unwrap(),
            "count_entries_via_compound"
        );
    }

    /// End-to-end: `select=DOCUMENTS` parity with v0 — same query
    /// shape against both endpoints should return the same matched
    /// documents.
    #[test]
    fn e2e_documents_select_matches_v0() {
        use dpp::data_contract::DataContractFactory;

        const PROTOCOL_VERSION_V12: u32 = 12;

        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let platform_version = PlatformVersion::latest();

        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color": {"type": "string", "position": 0, "maxLength": 32},
            },
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        let contract = factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create contract")
            .data_contract_owned();
        store_data_contract(&platform, &contract, version);

        let document_type = contract.document_type_for_name("widget").expect("widget");
        for i in 1..=3u8 {
            let doc = document_type
                .random_document(Some(i as u64), platform_version)
                .expect("random doc");
            store_document(&platform, &contract, document_type, &doc, platform_version);
        }

        // v0 baseline.
        let request_v0 = GetDocumentsRequestV0 {
            data_contract_id: contract.id().to_vec(),
            document_type: "widget".to_string(),
            r#where: Vec::new(),
            order_by: Vec::new(),
            limit: 0,
            prove: false,
            start: None,
        };
        let v0_result = platform
            .query_documents_v0(request_v0, &state, version)
            .expect("v0 query");
        let v0_docs = match v0_result.data {
            Some(r) => match r.result {
                Some(get_documents_response_v0::Result::Documents(d)) => d.documents,
                other => panic!("v0: expected Documents, got {:?}", other),
            },
            None => panic!("v0: empty data"),
        };
        assert_eq!(v0_docs.len(), 3);

        // v1 equivalent.
        let request_v1 = GetDocumentsRequestV1 {
            data_contract_id: contract.id().to_vec(),
            document_type: "widget".to_string(),
            r#where: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            start: None,
            prove: false,
            select: V1Select::Documents as i32,
            group_by: Vec::new(),
            having: Vec::new(),
        };
        let v1_result = platform
            .query_documents_v1(request_v1, &state, version)
            .expect("v1 query");
        let v1_docs = match v1_result.data {
            Some(r) => match r.result {
                Some(get_documents_response_v1::Result::Documents(d)) => d.documents,
                other => panic!("v1: expected Documents, got {:?}", other),
            },
            None => panic!("v1: empty data"),
        };
        assert_eq!(v1_docs, v0_docs, "v0 and v1 returned the same documents");
    }

    /// End-to-end: HAVING rejection reaches the response cleanly
    /// (not as a panic or generic error). The full handler is
    /// exercised so we know the `validate_and_route` rejection
    /// surfaces correctly through the `QueryValidationResult`
    /// machinery.
    #[test]
    fn e2e_having_rejection_surfaces_in_response() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);
        let request = GetDocumentsRequestV1 {
            data_contract_id: vec![0u8; 32],
            document_type: "anything".to_string(),
            r#where: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            start: None,
            prove: false,
            select: V1Select::Count as i32,
            group_by: Vec::new(),
            having: vec![0xFF, 0xFE],
        };
        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("query call should not error at the transport layer");
        assert!(
            !result.errors.is_empty(),
            "expected validation error for HAVING request"
        );
        match &result.errors[0] {
            QueryError::Query(QuerySyntaxError::Unsupported(msg)) => {
                assert!(
                    msg.contains("HAVING") && msg.contains("not yet implemented"),
                    "expected HAVING-specific message, got: {}",
                    msg
                );
            }
            other => panic!("expected Unsupported error, got {:?}", other),
        }
    }

    /// `start_after` / `start_at` doesn't make sense on `SELECT
    /// COUNT` — no concept of "skip past this aggregate." Reject
    /// explicitly with a hint pointing at range-narrowing as the
    /// pagination strategy.
    #[test]
    fn reject_start_with_select_count() {
        let (platform, state, version) = setup_platform(None, Network::Testnet, None);

        // Build a contract and document_type so the v0-count
        // delegation reaches a real codepath before our `start`
        // check in `dispatch_count_v1_to_v0` short-circuits.
        // Actually — the start check fires before contract lookup;
        // we can use a dummy contract_id and document_type and
        // still trigger the rejection cleanly.
        let request = GetDocumentsRequestV1 {
            data_contract_id: vec![0u8; 32],
            document_type: "widget".to_string(),
            r#where: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            start: Some(V1Start::StartAfter(vec![1u8; 32])),
            prove: false,
            select: V1Select::Count as i32,
            group_by: Vec::new(),
            having: Vec::new(),
        };
        let result = platform
            .query_documents_v1(request, &state, version)
            .expect("query call should not error at the transport layer");
        assert!(!result.errors.is_empty(), "expected validation error");
        match &result.errors[0] {
            QueryError::Query(QuerySyntaxError::Unsupported(msg)) => {
                assert!(
                    msg.contains("start_after") && msg.contains("not yet implemented"),
                    "expected start_after-specific message, got: {}",
                    msg
                );
            }
            other => panic!("expected Unsupported error, got {:?}", other),
        }
    }
}
