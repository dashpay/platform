//! Shared SUM-proof dispatch used by [`DocumentSum`] and
//! [`DocumentSplitSums`].
//!
//! Mirror of [`super::count_proof_helpers::verify_count_query`] for
//! the sum surface. Both consumers reduce to "give me verified
//! `Vec<SumEntry>` for this `DocumentQuery`" — [`DocumentSum`] sums
//! the entries into a single `i64`, [`DocumentSplitSums`] passes
//! them through.
//!
//! Routing is driven by drive's [`SumProofPlan`] (via
//! [`DriveDocumentSumQuery::proof_plan`], the plan the server proves
//! by) rather than ad-hoc clause-shape heuristics — same approach as
//! count. Without that, `group_by =
//! [in_field]` with a co-present range clause could be silently
//! answered with an aggregate-shape proof while the server emitted
//! a carrier-shape proof, producing verification failures (or worse,
//! silently-accepted mismatches if the byte shapes happened to
//! overlap).
//!
//! [`DocumentSum`]: drive_proof_verifier::DocumentSum
//! [`DocumentSplitSums`]: drive_proof_verifier::DocumentSplitSums

use crate::documents::aggregate_routing::grouped_mode;
use crate::documents::array_element_clauses::{
    aggregate_request_where_clauses, aggregate_where_clauses, document_type_named,
};
use crate::documents::document_query::normalize_time_range_clauses_with_metadata_time;
use crate::documents::document_query::DocumentQuery;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use drive::query::drive_document_sum_query::mode_detection::{
    check_sum_property_against_document_type, detect_sum_mode_from_inputs,
};
use drive::query::drive_document_sum_query::{DriveDocumentSumQuery, SumMode, SumProofPlan};
use drive::query::{ResolvedTimeRange, SelectFunction, WhereClause};
use drive_proof_verifier::{
    verify_aggregate_sum_proof, verify_carrier_aggregate_sum_proof, verify_distinct_sum_proof,
    verify_point_lookup_sum_proof, verify_primary_key_sum_tree_proof, SumEntry,
};

/// Validate that the caller-built [`DocumentQuery`] targets the
/// sum surface. SUM always needs a non-empty `field` naming the
/// integer property to aggregate.
pub(super) fn assert_select_is_sum(
    request: &DocumentQuery,
) -> Result<(), drive_proof_verifier::Error> {
    if request.select.function != SelectFunction::Sum || request.select.field.is_empty() {
        return Err(drive_proof_verifier::Error::RequestError {
            error: format!(
                "DocumentSum / DocumentSplitSums require \
                 `SelectProjection::sum(\"<field>\")`; got {:?}. \
                 The named field must match the doctype-level \
                 `documentsSummable` OR a `summable: \"<field>\"` \
                 index covering the where-clause shape.",
                request.select
            ),
        });
    }
    Ok(())
}

/// The refusals [`verify_sum_query`] makes from the request alone, made
/// before it is sent, for the reasons
/// [`check_count_request`](super::count_proof_helpers::check_count_request)
/// gives.
///
/// It also refuses a sum of another property than the one the document type
/// sums its documents by. Every node refuses that request with an internal
/// error, which the SDK would retry on every other node and ban each one for.
/// The last check is the verifier's proof plan ([`sum_proof_plan`]), which the
/// node plans its proof by too, so a sum no index serves is refused here in the
/// node's words.
pub(super) fn check_sum_request(
    request: &DocumentQuery,
    folds_entries: bool,
    platform_version: &PlatformVersion,
) -> Result<(), drive_proof_verifier::Error> {
    request.ensure_no_sub_queries()?;
    assert_select_is_sum(request)?;
    let document_type = document_type_named(&request.data_contract, &request.document_type_name)?;
    check_sum_property_against_document_type(document_type, &request.select.field).map_err(
        |_| drive_proof_verifier::Error::RequestError {
            error: format!(
                "{} documents are summed by {}, so no node sums {} over them",
                request.document_type_name,
                document_type
                    .documents_summable()
                    .unwrap_or("another property"),
                request.select.field
            ),
        },
    )?;
    let (resolved, resolved_time_ranges) = request.resolved_for_checking()?;
    let request = &resolved;
    request.ensure_no_having()?;
    let mode = grouped_mode(request, "SUM", platform_version)?;
    let where_clauses =
        aggregate_request_where_clauses(request, mode, folds_entries, platform_version)?;
    sum_proof_plan(
        request,
        &document_type,
        &where_clauses,
        &resolved_time_ranges,
        SumMode::from(mode),
        platform_version,
    )
    .map(drop)
}

/// The proof plan a node makes for a proved sum of `request`'s select field,
/// made from the request alone and lowered to the path query its proof
/// answers. Drive resolves the SQL-shape `sum_mode` into its proof mode
/// ([`detect_sum_mode_from_inputs`]), picks the primary-key route or the
/// covering index and the limit and walk direction
/// ([`DriveDocumentSumQuery::proof_plan`]), and builds the path query
/// ([`SumProofPlan::path_query`]), which the check before sending discards
/// and the verifier's Drive call rebuilds from the plan. A client cannot know
/// a node's `max_query_limit`, so it leaves that check to the node.
///
/// `where_clauses` are the request's as the server reads them
/// ([`aggregate_where_clauses`]).
fn sum_proof_plan<'a>(
    request: &DocumentQuery,
    document_type: &'a DocumentTypeRef<'_>,
    where_clauses: &[WhereClause],
    resolved_time_ranges: &[ResolvedTimeRange],
    sum_mode: SumMode,
    platform_version: &PlatformVersion,
) -> Result<SumProofPlan<'a>, drive_proof_verifier::Error> {
    let mode = detect_sum_mode_from_inputs(where_clauses, sum_mode, true, platform_version)
        .map_err(|e| drive_proof_verifier::Error::RequestError {
            error: format!("sum-mode detection failed: {e}"),
        })?;
    let unverifiable = |e| drive_proof_verifier::Error::RequestError {
        error: format!("no answer to this sum can be verified: {e}"),
    };
    let plan = DriveDocumentSumQuery::proof_plan(
        document_type,
        where_clauses,
        resolved_time_ranges,
        &request.select.field,
        mode,
        &request.order_by_clauses,
        (request.limit != 0).then_some(request.limit),
        None,
    )
    .map_err(unverifiable)?;
    plan.path_query(
        request.data_contract.id().to_buffer(),
        *document_type,
        request.document_type_name.clone(),
        where_clauses.to_vec(),
        request.select.field.clone(),
        false,
        platform_version,
    )
    .map_err(unverifiable)?;
    Ok(plan)
}

/// Verify a SUM-shape proof and return per-branch `SumEntry`s.
///
/// Picks the verifier primitive by **Drive's [`SumProofPlan`]**, the
/// plan the server proved by, rather than a clause-shape heuristic,
/// so the SDK's routing matches the server's exactly. Mirrors count's
/// [`super::count_proof_helpers::verify_count_query`] approach.
///
/// **Routing**: route the request to a [`SumMode`] with the node's
/// own routing ([`grouped_mode`]), then take Drive's plan
/// ([`sum_proof_plan`]). Branch by the plan:
///
/// - [`SumProofPlan::PrimaryKeySumTree`] (empty where on a doctype
///   whose `documentsSummable` is the summed property) →
///   [`verify_primary_key_sum_tree_proof`].
/// - [`SumProofPlan::PointLookup`] (no range, with or without `In`)
///   → [`verify_point_lookup_sum_proof`].
/// - [`SumProofPlan::Range`] (range, no In, no distinct) →
///   [`verify_aggregate_sum_proof`] → single empty-key entry.
/// - [`SumProofPlan::RangeDistinct`] (range + distinct walk via
///   `GroupByRange` / `GroupByCompound`) →
///   [`verify_distinct_sum_proof`].
/// - [`SumProofPlan::RangeAggregateCarrier`] (`In + range +
///   group_by = [in_field]` on the prove path) →
///   [`verify_carrier_aggregate_sum_proof`].
///
/// `folds_entries` is whether the caller adds the verified entries up into one
/// total ([`aggregate_where_clauses`]).
pub(super) fn verify_sum_query(
    mut request: DocumentQuery,
    response: GetDocumentsResponse,
    folds_entries: bool,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(Option<Vec<SumEntry>>, ResponseMetadata, Proof), drive_proof_verifier::Error> {
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
    // The grouped mode the node's routing answers the request in, read over
    // the clauses as the routing reads them: windows resolved, elements not
    // yet keyed.
    let mode = grouped_mode(&request, "SUM", platform_version)?;
    request.where_clauses = aggregate_where_clauses(
        document_type,
        mode,
        std::mem::take(&mut request.where_clauses),
        folds_entries,
        platform_version,
    )?;

    // Drive's plan, the one the prover made from the same request: the
    // proof mode, the primary-key route or covering index, and the limit
    // the path query carries. On the prove path the server refuses an
    // over-max limit with a typed `InvalidLimit` before producing proof
    // bytes, so the SDK never sees a clamped value to un-clamp.
    let plan = sum_proof_plan(
        &request,
        &document_type,
        &request.where_clauses,
        &resolved_time_ranges,
        SumMode::from(mode),
        platform_version,
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
            single_empty_key_entry(verify_primary_key_sum_tree_proof(
                contract_id,
                &request.document_type_name,
                proof,
                mtd,
                platform_version,
                provider,
            )?)
        }
        SumProofPlan::PointLookup(index) => verify_point_lookup_sum_proof(
            &sum_query(index),
            proof,
            mtd,
            platform_version,
            provider,
        )?,
        SumProofPlan::Range(index) => single_empty_key_entry(verify_aggregate_sum_proof(
            &sum_query(index),
            proof,
            mtd,
            platform_version,
            provider,
        )?),
        SumProofPlan::RangeDistinct {
            index,
            limit,
            left_to_right,
        } => verify_distinct_sum_proof(
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
        } => verify_carrier_aggregate_sum_proof(
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

/// Wrap a single `i64` from an aggregate primitive (range-aggregate
/// or primary-key direct read) as a one-element `Vec<SumEntry>` so
/// call sites see a uniform shape.
fn single_empty_key_entry(sum: i64) -> Vec<SumEntry> {
    vec![SumEntry {
        in_key: None,
        key: Vec::new(),
        sum: Some(sum),
    }]
}
