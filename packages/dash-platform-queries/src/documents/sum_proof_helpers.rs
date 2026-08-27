//! Shared SUM-proof dispatch used by [`DocumentSum`] and
//! [`DocumentSplitSums`].
//!
//! Mirror of [`super::count_proof_helpers::verify_count_query`] for
//! the sum surface. Both consumers reduce to "give me verified
//! `Vec<SumEntry>` for this `DocumentQuery`" — [`DocumentSum`] sums
//! the entries into a single `i64`, [`DocumentSplitSums`] passes
//! them through.
//!
//! Routing is driven by drive's resolved [`DocumentSumMode`] (via
//! [`detect_sum_mode_from_inputs`]) rather than ad-hoc clause-shape
//! heuristics — same approach as count. Without that, `group_by =
//! [in_field]` with a co-present range clause could be silently
//! answered with an aggregate-shape proof while the server emitted
//! a carrier-shape proof, producing verification failures (or worse,
//! silently-accepted mismatches if the byte shapes happened to
//! overlap).
//!
//! [`DocumentSum`]: drive_proof_verifier::DocumentSum
//! [`DocumentSplitSums`]: drive_proof_verifier::DocumentSplitSums

use crate::documents::document_query::normalize_time_range_clauses_with_metadata_time;
use crate::documents::document_query::DocumentQuery;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters,
    data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters},
};
use drive::query::drive_document_sum_query::index_picker::{
    find_range_summable_index_for_where_clauses, find_summable_index_for_where_clauses,
};
use drive::query::drive_document_sum_query::mode_detection::detect_sum_mode_from_inputs;
use drive::query::drive_document_sum_query::{DocumentSumMode, DriveDocumentSumQuery, SumMode};
use drive::query::validate_and_canonicalize_where_clauses;
use drive::query::{SelectFunction, WhereOperator};
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

/// Verify a SUM-shape proof and return per-branch `SumEntry`s.
///
/// Picks the verifier primitive by **drive's resolved
/// [`DocumentSumMode`]** rather than a clause-shape heuristic, so
/// the SDK's routing matches the server's exactly. Mirrors count's
/// [`super::count_proof_helpers::verify_count_query`] approach.
///
/// **Routing**: build a [`SumMode`] from `(group_by,
/// where_clauses)` matching the abci handler's `validate_and_route`
/// logic, then call [`detect_sum_mode_from_inputs`] with
/// `prove = true` to get the resolved [`DocumentSumMode`]. Branch
/// by the resolved mode:
///
/// - [`DocumentSumMode::PointLookupProof`] (no range, with or
///   without `In`) → [`verify_point_lookup_sum_proof`].
///   Special-case: doctype-level `documentsSummable` + empty where
///   → [`verify_primary_key_sum_tree_proof`].
/// - [`DocumentSumMode::RangeProof`] (range, no In, no distinct) →
///   [`verify_aggregate_sum_proof`] → single empty-key entry.
/// - [`DocumentSumMode::RangeDistinctProof`] (range + distinct walk
///   via `GroupByRange` / `GroupByCompound`) →
///   [`verify_distinct_sum_proof`].
/// - [`DocumentSumMode::RangeAggregateCarrierProof`] (`In + range +
///   group_by = [in_field]` on the prove path) →
///   [`verify_carrier_aggregate_sum_proof`].
/// - `Total` / `PerInValue` / `RangeNoProof` are no-proof modes
///   that should be unreachable here (`prove = true`); reject as
///   `RequestError` if they bubble through.
pub(super) fn verify_sum_query(
    mut request: DocumentQuery,
    response: GetDocumentsResponse,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(Option<Vec<SumEntry>>, ResponseMetadata, Proof), drive_proof_verifier::Error> {
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

    // Canonicalize exactly as the server dispatcher does (the shared step
    // in `drive::query::canonicalize`): the prover merged `[f > A, f < B]`
    // pairs into one `between*` clause before its mode detection, so mode
    // detection here must run over the same canonical shape or a valid
    // proof is rejected.
    request.where_clauses = validate_and_canonicalize_where_clauses(
        std::mem::take(&mut request.where_clauses),
        platform_version,
    )
    .map_err(|e| drive_proof_verifier::Error::RequestError {
        error: format!("where-clause canonicalization failed: {e}"),
    })?;

    let document_type = request
        .data_contract
        .document_type_for_name(&request.document_type_name)
        .map_err(|e| drive_proof_verifier::Error::RequestError {
            error: format!(
                "document type {} not found in contract: {}",
                request.document_type_name, e
            ),
        })?;

    // Resolve the SQL-shape `SumMode` the request implies. Same
    // decision tree as `validate_and_route` in the abci handler —
    // single source of truth would be nicer but the SDK can't
    // depend on rs-drive-abci.
    let sum_mode = resolve_sum_mode(&request.group_by, &request.where_clauses)?;

    // Translate the SQL-shape mode + where-clause shape into the
    // resolved `DocumentSumMode` the prover dispatched on. Driver-
    // side detect_sum_mode_from_inputs is the single source of
    // truth — the SDK calling it directly keeps the verifier in
    // sync with whatever new prove-mode lands next.
    let resolved_mode =
        detect_sum_mode_from_inputs(&request.where_clauses, sum_mode, true, platform_version)
            .map_err(|e| drive_proof_verifier::Error::RequestError {
                error: format!("sum-mode detection failed: {e}"),
            })?;

    // Empty-where SUM fast path: primary-key SumTree element direct
    // read. Lives outside `detect_sum_mode`'s output because the
    // contract-level `documents_summable` flag isn't part of mode
    // detection; pre-empt before falling through to PointLookupProof.
    if matches!(resolved_mode, DocumentSumMode::PointLookupProof)
        && request.where_clauses.is_empty()
        && document_type
            .documents_summable()
            .map(|p| p == sum_property)
            .unwrap_or(false)
    {
        let sum = verify_primary_key_sum_tree_proof(
            contract_id,
            &request.document_type_name,
            proof,
            mtd,
            platform_version,
            provider,
        )?;
        return Ok((
            Some(single_empty_key_entry(sum)),
            mtd.clone(),
            proof.clone(),
        ));
    }

    // Pick the index the prover would have picked. Range modes
    // need a `range_summable: true` index; everything else uses
    // the regular `summable: "<prop>"` resolver. Mismatch here
    // would produce a path-query different from the prover's, so
    // the index lookup matches drive's dispatch.
    let needs_range_index = matches!(
        resolved_mode,
        DocumentSumMode::RangeProof
            | DocumentSumMode::RangeDistinctProof
            | DocumentSumMode::RangeAggregateCarrierProof
    );
    let index = if needs_range_index {
        find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            &request.where_clauses,
            &sum_property,
            &resolved_time_ranges,
        )
        .ok_or_else(|| drive_proof_verifier::Error::RequestError {
            error: "prove range SUM requires a `rangeSummable: true` index whose last \
                    property matches the range field and whose summable property \
                    matches the request's select `field`"
                .to_string(),
        })?
    } else {
        find_summable_index_for_where_clauses(
            document_type.indexes(),
            &request.where_clauses,
            &sum_property,
            &resolved_time_ranges,
        )
        .ok_or_else(|| drive_proof_verifier::Error::RequestError {
            error: "prove SUM requires a `summable: \"<prop>\"` index whose properties \
                    exactly match the where clause fields and whose summed property \
                    matches the request's select `field`, or `documentsSummable: \
                    \"<prop>\"` on the document type for unfiltered total sums"
                .to_string(),
        })?
    };
    let sum_query = DriveDocumentSumQuery {
        document_type,
        contract_id,
        document_type_name: request.document_type_name.clone(),
        index,
        where_clauses: request.where_clauses.clone(),
        sum_property,
    };

    match resolved_mode {
        DocumentSumMode::PointLookupProof => {
            let entries =
                verify_point_lookup_sum_proof(&sum_query, proof, mtd, platform_version, provider)?;
            Ok((Some(entries), mtd.clone(), proof.clone()))
        }
        DocumentSumMode::RangeProof => {
            let sum =
                verify_aggregate_sum_proof(&sum_query, proof, mtd, platform_version, provider)?;
            Ok((
                Some(single_empty_key_entry(sum)),
                mtd.clone(),
                proof.clone(),
            ))
        }
        DocumentSumMode::RangeDistinctProof => {
            // Limit handling on the prove path:
            // - Fallback uses [`drive::config::DEFAULT_QUERY_LIMIT`]
            //   (compile-time constant), matching the server's
            //   `DocumentSumMode::RangeDistinctProof` arm in
            //   `drive_document_sum_query/drive_dispatcher.rs`.
            //   Both sides MUST anchor to the same compile-time
            //   value — operator-tunable
            //   `drive_config.default_query_limit` is intentionally
            //   NOT used here so a tuned operator default can't
            //   byte-differ the reconstructed `SizedQuery::limit`
            //   from the prover's.
            // - Raw caller limit propagates unchanged on the prove
            //   path — the server rejects over-max
            //   (`max_query_limit`) requests with a typed
            //   `InvalidLimit` error before producing proof bytes,
            //   so the SDK never sees a clamped value to
            //   un-clamp.
            let limit_u16 = if request.limit == 0 {
                drive::config::DEFAULT_QUERY_LIMIT
            } else {
                u16::try_from(request.limit).map_err(|_| {
                    drive_proof_verifier::Error::RequestError {
                        error: format!(
                            "limit {} exceeds u16::MAX for distinct SUM proof",
                            request.limit
                        ),
                    }
                })?
            };
            let left_to_right = request
                .order_by_clauses
                .first()
                .map(|c| c.ascending)
                .unwrap_or(true);
            let entries = verify_distinct_sum_proof(
                &sum_query,
                proof,
                mtd,
                limit_u16,
                left_to_right,
                platform_version,
                provider,
            )?;
            Ok((Some(entries), mtd.clone(), proof.clone()))
        }
        DocumentSumMode::RangeAggregateCarrierProof => {
            let limit_u16 = if request.limit == 0 {
                None
            } else {
                Some(u16::try_from(request.limit).map_err(|_| {
                    drive_proof_verifier::Error::RequestError {
                        error: format!(
                            "limit {} exceeds u16::MAX for carrier-aggregate SUM proof",
                            request.limit
                        ),
                    }
                })?)
            };
            let left_to_right = request
                .order_by_clauses
                .first()
                .map(|c| c.ascending)
                .unwrap_or(true);
            let entries = verify_carrier_aggregate_sum_proof(
                &sum_query,
                proof,
                mtd,
                limit_u16,
                left_to_right,
                platform_version,
                provider,
            )?;
            Ok((Some(entries), mtd.clone(), proof.clone()))
        }
        // `Total` / `PerInValue` / `RangeNoProof` are no-proof modes
        // that detect_sum_mode_from_inputs returns only for
        // `prove = false`. We pass `prove = true`, so reaching here
        // would indicate a drive routing-table bug rather than a
        // user error — surface it clearly.
        DocumentSumMode::Total | DocumentSumMode::PerInValue | DocumentSumMode::RangeNoProof => {
            Err(drive_proof_verifier::Error::RequestError {
                error: format!(
                "internal: detect_sum_mode_from_inputs returned no-proof mode {resolved_mode:?} \
                 for prove=true — the routing table is internally inconsistent. \
                 Please report this as a drive bug."
            ),
            })
        }
    }
}

/// Build the SQL-shape [`SumMode`] from `(group_by, where_clauses)`.
/// Mirrors [`super::count_proof_helpers::resolve_count_mode`] —
/// same SQL surface, same routing decision shape (`SumMode` is
/// structurally identical to `CountMode`). Keeping the two
/// resolvers in lock-step means a future SQL extension only has
/// to land once in count + once in sum.
fn resolve_sum_mode(
    group_by: &[String],
    where_clauses: &[drive::query::WhereClause],
) -> Result<SumMode, drive_proof_verifier::Error> {
    let is_in_field = |field: &str| {
        where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In && wc.field == field)
    };
    let is_range_field = |field: &str| {
        where_clauses.iter().any(|wc| {
            drive::query::drive_document_sum_query::is_range_operator(wc.operator)
                && wc.field == field
        })
    };
    let unsupported = |feature: String| drive_proof_verifier::Error::RequestError {
        error: format!("{feature} (see issue #3655 for the v1 wire surface follow-ups)"),
    };
    match group_by {
        [] => Ok(SumMode::Aggregate),
        [field] => {
            if is_in_field(field) {
                Ok(SumMode::GroupByIn)
            } else if is_range_field(field) {
                Ok(SumMode::GroupByRange)
            } else {
                Err(drive_proof_verifier::Error::RequestError {
                    error: format!(
                        "GROUP BY on field '{field}' which is not constrained by an `In` \
                         or range where clause is not yet implemented (see issue #3655)"
                    ),
                })
            }
        }
        [first, second] => {
            if is_in_field(first) && is_range_field(second) {
                Ok(SumMode::GroupByCompound)
            } else {
                Err(unsupported(
                    "two-field GROUP BY outside the `(In, range)` compound shape \
                     is not yet implemented"
                        .to_string(),
                ))
            }
        }
        _ => Err(unsupported(
            "GROUP BY with more than two fields is not yet implemented".to_string(),
        )),
    }
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
