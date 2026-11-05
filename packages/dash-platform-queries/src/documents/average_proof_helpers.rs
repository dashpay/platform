//! Shared AVG-proof dispatch used by [`DocumentAverage`] and
//! [`DocumentSplitAverages`].
//!
//! Mirror of [`super::count_proof_helpers::verify_count_query`] for
//! the average surface. Both consumers reduce to "give me verified
//! `Vec<AverageEntry>` for this `DocumentQuery`" —
//! [`DocumentAverage`] sums into a single `(count, sum)` pair,
//! [`DocumentSplitAverages`] passes the entries through.
//!
//! Routing is driven by drive's [`SumProofPlan`] for an average (via
//! [`DriveDocumentSumQuery::average_proof_plan`], the plan the
//! server's average dispatcher proves by). AVG and SUM share the plan
//! type because their grovedb primitives differ only in which element
//! type is extracted (`KVCountSum` for AVG vs `KVSum` for SUM), but an
//! average is planned by its own [`AverageMode`] and whether a range
//! constrains it, exactly as the server plans it.
//!
//! [`DocumentAverage`]: drive_proof_verifier::DocumentAverage
//! [`DocumentSplitAverages`]: drive_proof_verifier::DocumentSplitAverages

use crate::documents::array_element_clauses::{
    aggregate_request_where_clauses, aggregate_where_clauses, document_type_named,
};
use crate::documents::document_query::normalize_time_range_clauses_with_metadata_time;
use crate::documents::document_query::DocumentQuery;
use crate::documents::sum_proof_helpers::resolve_sum_mode;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use drive::query::drive_document_average_query::AverageMode;
use drive::query::drive_document_sum_query::{DriveDocumentSumQuery, SumMode, SumProofPlan};
use drive::query::{OrderClause, ResolvedTimeRange, SelectFunction, WhereClause};
use drive_proof_verifier::{
    verify_aggregate_count_and_sum_proof, verify_carrier_aggregate_count_and_sum_proof,
    verify_distinct_count_and_sum_proof, verify_point_lookup_count_and_sum_proof,
    verify_primary_key_count_sum_tree_proof, AverageEntry,
};

/// Validate that the caller-built [`DocumentQuery`] targets the
/// average surface. AVG always needs a non-empty `field` naming
/// the integer property to average — `AVG()` with no field is a
/// wire-shape error (the server-side `not_yet_implemented` gate
/// rejects it too, so this is the SDK-side mirror).
pub(super) fn assert_select_is_avg(
    request: &DocumentQuery,
) -> Result<(), drive_proof_verifier::Error> {
    if request.select.function != SelectFunction::Avg || request.select.field.is_empty() {
        return Err(drive_proof_verifier::Error::RequestError {
            error: format!(
                "DocumentAverage / DocumentSplitAverages require \
                 `SelectProjection::avg(\"<field>\")`; got {:?}. \
                 The named field must match the doctype-level \
                 `documentsSummable` (or `documentsAverageable`) \
                 OR a `summable: \"<field>\"` index covering the \
                 where-clause shape — averages reuse sum-tree \
                 indexes, no separate `averageable` flag is needed.",
                request.select
            ),
        });
    }
    Ok(())
}

/// The refusals [`verify_average_query`] makes from the request alone, made
/// before it is sent, for the reasons
/// [`check_count_request`](super::count_proof_helpers::check_count_request)
/// gives. The last check is the verifier's proof plan
/// ([`average_proof_plan`]), which the node plans its proof by too, so an
/// average no index serves is refused here in the node's words.
pub(super) fn check_average_request(
    request: &DocumentQuery,
    folds_entries: bool,
    platform_version: &PlatformVersion,
) -> Result<(), drive_proof_verifier::Error> {
    request.ensure_no_sub_queries()?;
    assert_select_is_avg(request)?;
    let (resolved, resolved_time_ranges) = request.resolved_for_checking()?;
    let request = &resolved;
    let where_clauses = aggregate_request_where_clauses(request, folds_entries, platform_version)?;
    let average_mode = resolve_average_mode(&request.group_by, &where_clauses)?;
    request.ensure_no_having()?;
    let document_type = document_type_named(&request.data_contract, &request.document_type_name)?;
    average_proof_plan(
        &document_type,
        &where_clauses,
        &resolved_time_ranges,
        &request.select.field,
        average_mode,
        &request.order_by_clauses,
        request.limit,
    )
    .map(drop)
}

/// The SQL-shape [`AverageMode`] of an average request: the [`SumMode`] a sum
/// over the same `(group_by, where_clauses)` resolves to
/// ([`resolve_sum_mode`]), mapped 1:1 as drive-abci maps it.
fn resolve_average_mode(
    group_by: &[String],
    where_clauses: &[WhereClause],
) -> Result<AverageMode, drive_proof_verifier::Error> {
    Ok(match resolve_sum_mode(group_by, where_clauses)? {
        SumMode::Aggregate => AverageMode::Aggregate,
        SumMode::GroupByIn => AverageMode::GroupByIn,
        SumMode::GroupByRange => AverageMode::GroupByRange,
        SumMode::GroupByCompound => AverageMode::GroupByCompound,
    })
}

/// The proof plan a node makes for a proved average of `sum_property`, made
/// from the request alone ([`DriveDocumentSumQuery::average_proof_plan`]): the
/// primary-key route or the covering index, and the limit the path query
/// carries. A client cannot know a node's `max_query_limit`, so it leaves that
/// check to the node.
///
/// `order_clauses` and `limit` are the request's, `limit` `0` when unset.
fn average_proof_plan<'a>(
    document_type: &'a DocumentTypeRef<'_>,
    where_clauses: &[WhereClause],
    resolved_time_ranges: &[ResolvedTimeRange],
    sum_property: &str,
    average_mode: AverageMode,
    order_clauses: &[OrderClause],
    limit: u32,
) -> Result<SumProofPlan<'a>, drive_proof_verifier::Error> {
    DriveDocumentSumQuery::average_proof_plan(
        document_type,
        where_clauses,
        resolved_time_ranges,
        sum_property,
        average_mode,
        order_clauses,
        (limit != 0).then_some(limit),
        None,
    )
    .map_err(|e| drive_proof_verifier::Error::RequestError {
        error: format!("no answer to this average can be verified: {e}"),
    })
}

/// Verify an AVG-shape proof and return per-branch `AverageEntry`s.
///
/// Picks the verifier primitive by **Drive's [`SumProofPlan`] for an
/// average**, the plan the server's average dispatcher proved by,
/// rather than a clause-shape heuristic, so the SDK's routing matches
/// the server's exactly.
///
/// **Routing**: build an [`AverageMode`] from `(group_by,
/// where_clauses)` matching the abci handler's `validate_and_route`
/// logic, then take Drive's plan ([`average_proof_plan`]). Branch by
/// the plan:
///
/// - [`SumProofPlan::PrimaryKeySumTree`] (empty where on a doctype
///   that declares `documentsCountable` and sums by the averaged
///   property) → [`verify_primary_key_count_sum_tree_proof`].
/// - [`SumProofPlan::PointLookup`] (no range, with or without `In`)
///   → [`verify_point_lookup_count_and_sum_proof`].
/// - [`SumProofPlan::Range`] (range, ungrouped) →
///   [`verify_aggregate_count_and_sum_proof`] → single empty-key
///   entry.
/// - [`SumProofPlan::RangeDistinct`] (range + distinct walk via
///   `GroupByRange` / `GroupByCompound`) →
///   [`verify_distinct_count_and_sum_proof`].
/// - [`SumProofPlan::RangeAggregateCarrier`] (`In + range +
///   group_by = [in_field]` on the prove path) →
///   [`verify_carrier_aggregate_count_and_sum_proof`].
///
/// `folds_entries` is whether the caller adds the verified entries up into one
/// total ([`aggregate_where_clauses`]).
pub(super) fn verify_average_query(
    mut request: DocumentQuery,
    response: GetDocumentsResponse,
    folds_entries: bool,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(Option<Vec<AverageEntry>>, ResponseMetadata, Proof), drive_proof_verifier::Error> {
    request.ensure_no_sub_queries()?;
    let proof = response
        .proof()
        .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
    let mtd = response
        .metadata()
        .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;
    let contract_id = request.data_contract.id().to_buffer();
    let sum_property = request.select.field.clone();

    // Resolve any pending time-range (`IN_TIME_RANGE`) selections into
    // concrete bucket-equality clauses using the quorum-signed metadata
    // block time — BEFORE mode detection and covering-index selection
    // below, which read `request.where_clauses`; the prover routed on
    // the resolved shape.
    // ...and enforce the same provenance-vs-shape contract the server
    // dispatchers do, through the one shared normalization helper.
    let resolved_time_ranges =
        normalize_time_range_clauses_with_metadata_time(&mut request, mtd.time_ms)?;

    // The server's dispatcher reads the clauses with a typed array's elements
    // keyed, then canonicalized, before its mode detection; the verifier reads
    // them the same way, or a valid proof is refused (or a refused total
    // accepted).
    let document_type = request
        .data_contract
        .document_type_for_name(&request.document_type_name)
        .map_err(|e| drive_proof_verifier::Error::RequestError {
            error: format!(
                "document type {} not found in contract: {}",
                request.document_type_name, e
            ),
        })?;
    request.where_clauses = aggregate_where_clauses(
        document_type,
        &request.group_by,
        std::mem::take(&mut request.where_clauses),
        folds_entries,
        platform_version,
    )?;

    // Resolve the SQL-shape `AverageMode` the request implies — the
    // same decision tree as `validate_and_route` in the abci handler,
    // which maps it 1:1 from the count / sum mode.
    let average_mode = resolve_average_mode(&request.group_by, &request.where_clauses)?;

    // Drive's plan, the one the prover made from the same request: the
    // primary-key route or covering index, and the limit the path query
    // carries.
    let plan = average_proof_plan(
        &document_type,
        &request.where_clauses,
        &resolved_time_ranges,
        &sum_property,
        average_mode,
        &request.order_by_clauses,
        request.limit,
    )?;
    let sum_query = |index| DriveDocumentSumQuery {
        document_type,
        contract_id,
        document_type_name: request.document_type_name.clone(),
        index,
        where_clauses: request.where_clauses.clone(),
        sum_property: sum_property.clone(),
    };

    let entries = match plan {
        SumProofPlan::PrimaryKeySumTree => {
            let (count, sum) = verify_primary_key_count_sum_tree_proof(
                contract_id,
                &request.document_type_name,
                proof,
                mtd,
                platform_version,
                provider,
            )?;
            single_empty_key_entry(count, sum)
        }
        SumProofPlan::PointLookup(index) => verify_point_lookup_count_and_sum_proof(
            &sum_query(index),
            proof,
            mtd,
            platform_version,
            provider,
        )?,
        SumProofPlan::Range(index) => {
            let (count, sum) = verify_aggregate_count_and_sum_proof(
                &sum_query(index),
                proof,
                mtd,
                platform_version,
                provider,
            )?;
            single_empty_key_entry(count, sum)
        }
        SumProofPlan::RangeDistinct {
            index,
            limit,
            left_to_right,
        } => verify_distinct_count_and_sum_proof(
            &sum_query(index),
            proof,
            mtd,
            limit,
            left_to_right,
            platform_version,
            provider,
        )?,
        SumProofPlan::RangeAggregateCarrier {
            index,
            limit,
            left_to_right,
        } => verify_carrier_aggregate_count_and_sum_proof(
            &sum_query(index),
            proof,
            mtd,
            limit,
            left_to_right,
            platform_version,
            provider,
        )?,
    };
    Ok((Some(entries), mtd.clone(), proof.clone()))
}

/// Wrap a single `(count, sum)` from a per-key-less aggregate
/// primitive (primary-key fast path / PCPS aggregate range) as a
/// one-element `Vec<AverageEntry>` so call sites see a uniform
/// shape across aggregate and carrier variants.
fn single_empty_key_entry(count: u64, sum: i64) -> Vec<AverageEntry> {
    vec![AverageEntry {
        in_key: None,
        key: Vec::new(),
        count: Some(count),
        sum: Some(sum),
    }]
}
