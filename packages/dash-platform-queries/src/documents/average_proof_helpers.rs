//! Shared AVG-proof dispatch used by [`DocumentAverage`] and
//! [`DocumentSplitAverages`].
//!
//! Mirror of [`super::count_proof_helpers::verify_count_query`] for
//! the average surface. Both consumers reduce to "give me verified
//! `Vec<AverageEntry>` for this `DocumentQuery`" —
//! [`DocumentAverage`] sums into a single `(count, sum)` pair,
//! [`DocumentSplitAverages`] passes the entries through.
//!
//! Routing is driven by drive's resolved [`DocumentSumMode`] (via
//! [`detect_sum_mode_from_inputs`]) — same as the SUM helper.
//! AVG and SUM share the same routing table because their grovedb
//! primitives differ only in which element type is extracted
//! (`KVCountSum` for AVG vs `KVSum` for SUM); the dispatch
//! decisions are otherwise identical, which is why drive's
//! `drive_dispatcher` translates `AverageMode` → `(CountMode,
//! SumMode)` 1:1 before invoking the per-mode executor.
//!
//! [`DocumentAverage`]: drive_proof_verifier::DocumentAverage
//! [`DocumentSplitAverages`]: drive_proof_verifier::DocumentSplitAverages

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
use drive::query::{SelectFunction, WhereOperator};
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

/// Verify an AVG-shape proof and return per-branch `AverageEntry`s.
///
/// Picks the verifier primitive by **drive's resolved
/// [`DocumentSumMode`]** (AVG reuses SUM's resolved-mode space —
/// see module docstring) rather than a clause-shape heuristic, so
/// the SDK's routing matches the server's exactly.
///
/// **Routing**: build a [`SumMode`] from `(group_by,
/// where_clauses)` matching the abci handler's `validate_and_route`
/// logic, then call [`detect_sum_mode_from_inputs`] with
/// `prove = true` to get the resolved [`DocumentSumMode`]. Branch
/// by the resolved mode:
///
/// - [`DocumentSumMode::PointLookupProof`] (no range, with or
///   without `In`) → [`verify_point_lookup_count_and_sum_proof`].
///   Special-case: doctype-level `documentsCountable +
///   documentsSummable` + empty where →
///   [`verify_primary_key_count_sum_tree_proof`].
/// - [`DocumentSumMode::RangeProof`] (range, no In, no distinct) →
///   [`verify_aggregate_count_and_sum_proof`] → single empty-key
///   entry.
/// - [`DocumentSumMode::RangeDistinctProof`] (range + distinct walk
///   via `GroupByRange` / `GroupByCompound`) →
///   [`verify_distinct_count_and_sum_proof`].
/// - [`DocumentSumMode::RangeAggregateCarrierProof`] (`In + range +
///   group_by = [in_field]` on the prove path) →
///   [`verify_carrier_aggregate_count_and_sum_proof`].
/// - `Total` / `PerInValue` / `RangeNoProof` are no-proof modes
///   that should be unreachable here (`prove = true`); reject as
///   `RequestError` if they bubble through.
pub(super) fn verify_average_query(
    mut request: DocumentQuery,
    response: GetDocumentsResponse,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(Option<Vec<AverageEntry>>, ResponseMetadata, Proof), drive_proof_verifier::Error> {
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
        super::document_query::normalize_time_range_clauses_with_metadata_time(
            &mut request,
            mtd.time_ms,
        )?;

    let document_type = request
        .data_contract
        .document_type_for_name(&request.document_type_name)
        .map_err(|e| drive_proof_verifier::Error::RequestError {
            error: format!(
                "document type {} not found in contract: {}",
                request.document_type_name, e
            ),
        })?;

    // Resolve the SQL-shape `SumMode` the request implies — AVG
    // shares the routing table with SUM (see module docstring), so
    // we use the same SumMode resolver. The shape is mechanically
    // identical to `AverageMode` (Aggregate / GroupByIn /
    // GroupByRange / GroupByCompound).
    let sum_mode = resolve_sum_mode(&request.group_by, &request.where_clauses)?;

    let resolved_mode =
        detect_sum_mode_from_inputs(&request.where_clauses, sum_mode, true, platform_version)
            .map_err(|e| drive_proof_verifier::Error::RequestError {
                error: format!("avg-mode detection failed (via sum-mode router): {e}"),
            })?;

    // Empty-where AVG fast path: primary-key count-sum-bearing
    // element direct read. Doctype must declare BOTH
    // `documentsCountable` AND a matching `documentsSummable`.
    if matches!(resolved_mode, DocumentSumMode::PointLookupProof)
        && request.where_clauses.is_empty()
        && document_type.documents_countable()
        && document_type
            .documents_summable()
            .map(|p| p == sum_property)
            .unwrap_or(false)
    {
        let (count, sum) = verify_primary_key_count_sum_tree_proof(
            contract_id,
            &request.document_type_name,
            proof,
            mtd,
            platform_version,
            provider,
        )?;
        return Ok((
            Some(single_empty_key_entry(count, sum)),
            mtd.clone(),
            proof.clone(),
        ));
    }

    // Pick the index the prover would have picked. Range modes need
    // an index that's BOTH `range_summable: true` AND
    // `range_countable: true` (i.e. PCPS) — that's the surface a
    // `rangeAverageable: true` index resolves to. Everything else
    // uses a summable + countable terminator.
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
        .filter(|idx| idx.range_countable)
        .ok_or_else(|| drive_proof_verifier::Error::RequestError {
            error: "prove range AVG requires an index that declares BOTH `rangeCountable: \
                    true` AND `rangeSummable: true` (a `rangeAverageable: true` \
                    index is the shorthand) whose last property matches the range \
                    field and whose summable property matches the request's \
                    select `field`"
                .to_string(),
        })?
    } else {
        find_summable_index_for_where_clauses(
            document_type.indexes(),
            &request.where_clauses,
            &sum_property,
            &resolved_time_ranges,
        )
        .filter(|idx| idx.countable.is_countable())
        .ok_or_else(|| drive_proof_verifier::Error::RequestError {
            error: "prove AVG requires an index that declares BOTH `summable: \
                    \"<prop>\"` AND a countable terminator (`countable: \
                    \"countable\"` or `\"countableAllowingOffset\"`) whose properties \
                    exactly match the where clause fields"
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
            let entries = verify_point_lookup_count_and_sum_proof(
                &sum_query,
                proof,
                mtd,
                platform_version,
                provider,
            )?;
            Ok((Some(entries), mtd.clone(), proof.clone()))
        }
        DocumentSumMode::RangeProof => {
            let (count, sum) = verify_aggregate_count_and_sum_proof(
                &sum_query,
                proof,
                mtd,
                platform_version,
                provider,
            )?;
            Ok((
                Some(single_empty_key_entry(count, sum)),
                mtd.clone(),
                proof.clone(),
            ))
        }
        DocumentSumMode::RangeDistinctProof => {
            let limit_u16 = if request.limit == 0 {
                drive::config::DEFAULT_QUERY_LIMIT
            } else {
                u16::try_from(request.limit).map_err(|_| {
                    drive_proof_verifier::Error::RequestError {
                        error: format!(
                            "limit {} exceeds u16::MAX for distinct AVG proof",
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
            let entries = verify_distinct_count_and_sum_proof(
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
                            "limit {} exceeds u16::MAX for carrier-aggregate AVG proof",
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
            let entries = verify_carrier_aggregate_count_and_sum_proof(
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
        DocumentSumMode::Total | DocumentSumMode::PerInValue | DocumentSumMode::RangeNoProof => {
            Err(drive_proof_verifier::Error::RequestError {
                error: format!(
                "internal: detect_sum_mode_from_inputs returned no-proof mode {resolved_mode:?} \
                 for prove=true (AVG path) — the routing table is internally inconsistent. \
                 Please report this as a drive bug."
            ),
            })
        }
    }
}

/// Build the SQL-shape [`SumMode`] from `(group_by, where_clauses)`.
/// Identical resolver to [`super::sum_proof_helpers::resolve_sum_mode`]
/// — AVG and SUM share the same SQL surface (their `AverageMode` /
/// `SumMode` enums have the same shape and the same routing
/// decisions). Duplicated here to keep the SDK helper self-contained
/// (so users can disable the sum surface without breaking AVG).
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
