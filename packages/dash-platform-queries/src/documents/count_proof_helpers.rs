//! Shared count-proof dispatch used by [`DocumentCount`] and
//! [`DocumentSplitCounts`].
//!
//! Both consumers reduce to "give me a verified
//! `Vec<SplitCountEntry>` for this `DocumentQuery`" —
//! [`DocumentCount`] sums the entries into a single `u64`,
//! [`DocumentSplitCounts`] passes them through. Putting the
//! four-way proof dispatch behind one helper means the per-shape
//! routing (which proof primitive to use, which index to pick,
//! how to wrap the result) lives in exactly one place; the
//! consumers become thin wrappers.
//!
//! [`DocumentCount`]: drive_proof_verifier::DocumentCount
//! [`DocumentSplitCounts`]: drive_proof_verifier::DocumentSplitCounts

use crate::documents::document_query::DocumentQuery;
use dapi_grpc::platform::v0::{GetDocumentsResponse, Proof, ResponseMetadata};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters,
    data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters},
};
use drive::query::{
    CountMode, DocumentCountMode, DriveDocumentCountQuery, SelectFunction, WhereOperator,
};
use drive_proof_verifier::{
    verify_aggregate_count_proof, verify_carrier_aggregate_count_proof,
    verify_distinct_count_proof, verify_point_lookup_count_proof,
    verify_primary_key_count_tree_proof, SplitCountEntry,
};

/// Validate that the caller-built [`DocumentQuery`] actually
/// targets the count surface AND uses the `COUNT(*)` shape — the
/// only shape today's verifier can reproduce. The verifier in
/// `verify_count_query()` rebuilds a `DriveDocumentCountQuery`
/// without threading the selected `field`, so an accepted
/// `COUNT(field)` request would verify as `COUNT(*)` (different
/// result for nullable fields). Reject `COUNT(field)` upstream
/// until the verifier carries the counted field; the
/// not-yet-implemented gate already rejects it server-side, so
/// this check is the SDK-side mirror.
pub(super) fn assert_select_is_count(
    request: &DocumentQuery,
) -> Result<(), drive_proof_verifier::Error> {
    if request.select.function != SelectFunction::Count || !request.select.field.is_empty() {
        return Err(drive_proof_verifier::Error::RequestError {
            error: format!(
                "DocumentCount / DocumentSplitCounts currently require \
                 `SelectProjection::count_star()` (i.e. `COUNT(*)`); got {:?}. \
                 `COUNT(field)` is not verifiable today because the proof \
                 query doesn't carry the counted field — `COUNT(field)` \
                 against a nullable field would verify as `COUNT(*)` and \
                 return a different total. Call \
                 `.with_select(SelectProjection::count_star())` on the \
                 DocumentQuery before fetching.",
                request.select
            ),
        });
    }
    Ok(())
}

/// Translate the SDK's `u32`-with-`0`-sentinel limit into the
/// `u16` the proof verifier wants to rebuild the prover's path
/// query.
///
/// `0` falls back to [`drive::config::DEFAULT_QUERY_LIMIT`] — the
/// same compile-time constant the server's prove-distinct
/// dispatcher reads (NOT the operator-tunable
/// `drive_config.default_query_limit`, which the SDK can't see).
/// With both sides anchored to the shared constant the path-query
/// bytes match byte-for-byte across operators, so merk-root
/// recomputation succeeds regardless of any operator's tuning.
///
/// Non-zero values must fit in `u16` since the wire's
/// `optional uint32` is wider than the verifier's path-query
/// representation. We `try_from` rather than truncate so a caller
/// passing `limit > u16::MAX` fails loudly at the SDK boundary
/// rather than silently producing a mismatched path query.
fn limit_to_u16_or_default(limit: u32) -> Result<u16, drive_proof_verifier::Error> {
    if limit == 0 {
        return Ok(drive::config::DEFAULT_QUERY_LIMIT);
    }
    u16::try_from(limit).map_err(|_| drive_proof_verifier::Error::RequestError {
        error: format!(
            "limit {} exceeds u16::MAX; the prove-distinct path query cannot represent it",
            limit
        ),
    })
}

/// Verify a count-shape proof and return per-branch entries.
///
/// Single source of truth for the count-proof dispatch. Picks
/// the verifier primitive by **drive's resolved
/// [`DocumentCountMode`]** rather than a clause-shape heuristic,
/// so the SDK's routing decision matches the server's exactly.
/// Mismatch here was previously possible — e.g. `group_by =
/// [in_field]` with a co-present range clause on the prove path
/// produces a carrier-aggregate proof server-side but the old
/// `has_range && !group_by.is_empty()` heuristic routed it to
/// `verify_distinct_count_proof` (different primitive ⇒
/// verification fails).
///
/// **Routing**: build a [`CountMode`] from `(group_by,
/// where_clauses)` matching the abci handler's
/// `validate_and_route` logic, then call
/// [`DriveDocumentCountQuery::detect_mode`] with `prove = true`
/// to get the resolved [`DocumentCountMode`]. Branch by the
/// resolved mode:
///
/// - [`DocumentCountMode::PointLookupProof`] (no range, with or
///   without `In`) → `verify_point_lookup_count_proof`.
///   Special-case: `documents_countable: true` doctype + empty
///   where → `verify_primary_key_count_tree_proof`.
/// - [`DocumentCountMode::RangeProof`] (range, no In, no
///   distinct) → `verify_aggregate_count_proof` → single
///   empty-key entry.
/// - [`DocumentCountMode::RangeDistinctProof`] (range + distinct
///   walk via `GroupByRange` / `GroupByCompound`) →
///   `verify_distinct_count_proof`.
/// - [`DocumentCountMode::RangeAggregateCarrierProof`] (`In +
///   range + group_by=[in_field]` on the prove path; grovedb #663
///   carrier primitive) → `verify_carrier_aggregate_count_proof`.
/// - `Total` / `PerInValue` / `RangeNoProof` are no-proof modes
///   and would be unreachable here (`prove=true`); reject as
///   `Internal` if they ever bubble through.
///
/// Wrapping aggregate primitives (`RangeProof`, primary-key
/// CountTree) as single empty-key entries is the only shape
/// massage this helper does — the underlying primitives
/// genuinely emit `u64`s, and consumers ([`DocumentCount`] sums,
/// [`DocumentSplitCounts`] passes through) want a uniform
/// per-entry vec regardless.
///
/// [`DocumentCount`]: drive_proof_verifier::DocumentCount
/// [`DocumentSplitCounts`]: drive_proof_verifier::DocumentSplitCounts
pub(super) fn verify_count_query(
    request: DocumentQuery,
    response: GetDocumentsResponse,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(Option<Vec<SplitCountEntry>>, ResponseMetadata, Proof), drive_proof_verifier::Error> {
    let document_type = request
        .data_contract
        .document_type_for_name(&request.document_type_name)
        .map_err(|e| drive_proof_verifier::Error::RequestError {
            error: format!(
                "document type {} not found in contract: {}",
                request.document_type_name, e
            ),
        })?;
    let proof = response
        .proof()
        .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
    let mtd = response
        .metadata()
        .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;

    // Resolve the SQL-shape `CountMode` the request implies. Same
    // decision tree as `validate_and_route` in the abci handler —
    // single source of truth would be nicer but the SDK can't
    // depend on rs-drive-abci, and drive doesn't expose this
    // helper because `validate_and_route` also folds in the
    // unrelated `select` projection check.
    let count_mode = resolve_count_mode(&request.group_by, &request.where_clauses)?;

    // Translate the SQL-shape mode + where-clause shape into the
    // resolved `DocumentCountMode` the prover would dispatch on.
    // Driver-side detect_mode is the single source of truth — the
    // SDK calling it directly is what keeps the verifier in sync
    // with whatever new prove-mode lands next.
    let resolved_mode = DriveDocumentCountQuery::detect_mode_versioned(
        &request.where_clauses,
        count_mode,
        true,
        platform_version,
    )
    .map_err(|e| drive_proof_verifier::Error::RequestError {
        error: format!("count-mode detection failed: {e}"),
    })?;

    // Special-case: empty where-clauses on a `documents_countable`
    // doctype proves the primary-key CountTree element directly,
    // skipping the per-index covering walk. This lives outside
    // `detect_mode`'s output because the contract-level
    // `documents_countable` flag isn't part of mode detection;
    // pre-empt it here before falling through to PointLookupProof.
    if matches!(resolved_mode, DocumentCountMode::PointLookupProof)
        && request.where_clauses.is_empty()
        && document_type.documents_countable()
    {
        let contract_id = request.data_contract.id().to_buffer();
        let count = verify_primary_key_count_tree_proof(
            contract_id,
            &request.document_type_name,
            proof,
            mtd,
            platform_version,
            provider,
        )?;
        return Ok((
            Some(single_empty_key_entry(count)),
            mtd.clone(),
            proof.clone(),
        ));
    }

    // Pick the index the prover would have picked. Range modes
    // need a `range_countable: true` index; everything else uses
    // the regular `countable: true` resolver. Mismatch here would
    // produce a path-query different from the prover's, so the
    // index lookup matches drive's `range_count_path_query` /
    // `point_lookup_count_path_query` dispatch.
    let needs_range_index = matches!(
        resolved_mode,
        DocumentCountMode::RangeProof
            | DocumentCountMode::RangeDistinctProof
            | DocumentCountMode::RangeAggregateCarrierProof
    );
    let index = if needs_range_index {
        DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &request.where_clauses,
        )
        .ok_or_else(|| drive_proof_verifier::Error::RequestError {
            error: "range count requires a `range_countable: true` index whose last \
                    property matches the range field"
                .to_string(),
        })?
    } else {
        DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &request.where_clauses,
        )
        .ok_or_else(|| drive_proof_verifier::Error::RequestError {
            error: "prove count requires a `countable: true` index whose properties \
                    exactly match the where clause fields, or `documentsCountable: \
                    true` on the document type for unfiltered total counts"
                .to_string(),
        })?
    };
    let count_query = DriveDocumentCountQuery {
        document_type,
        contract_id: request.data_contract.id().to_buffer(),
        document_type_name: request.document_type_name.clone(),
        index,
        where_clauses: request.where_clauses.clone(),
    };

    match resolved_mode {
        DocumentCountMode::PointLookupProof => {
            let entries = verify_point_lookup_count_proof(
                &count_query,
                proof,
                mtd,
                platform_version,
                provider,
            )?;
            Ok((Some(entries), mtd.clone(), proof.clone()))
        }
        DocumentCountMode::RangeProof => {
            let count =
                verify_aggregate_count_proof(&count_query, proof, mtd, platform_version, provider)?;
            Ok((
                Some(single_empty_key_entry(count)),
                mtd.clone(),
                proof.clone(),
            ))
        }
        DocumentCountMode::RangeDistinctProof => {
            let limit_u16 = limit_to_u16_or_default(request.limit)?;
            let left_to_right = request
                .order_by_clauses
                .first()
                .map(|c| c.ascending)
                .unwrap_or(true);
            let entries = verify_distinct_count_proof(
                &count_query,
                proof,
                mtd,
                limit_u16,
                left_to_right,
                platform_version,
                provider,
            )?;
            Ok((Some(entries), mtd.clone(), proof.clone()))
        }
        DocumentCountMode::RangeAggregateCarrierProof => {
            // Carrier-ACOR (grovedb #663) — one verified `u64` per
            // present In branch. `limit` cap on the per-branch
            // walk follows the same `validate-don't-clamp`
            // contract the distinct path uses; pass through what
            // the caller asked for (with the `0` → default
            // sentinel) so the path-query bytes match the
            // server's exactly.
            let limit_u16 = if request.limit == 0 {
                None
            } else {
                Some(limit_to_u16_or_default(request.limit)?)
            };
            let left_to_right = request
                .order_by_clauses
                .first()
                .map(|c| c.ascending)
                .unwrap_or(true);
            let entries = verify_carrier_aggregate_count_proof(
                &count_query,
                proof,
                mtd,
                limit_u16,
                left_to_right,
                platform_version,
                provider,
            )?;
            Ok((Some(entries), mtd.clone(), proof.clone()))
        }
        // The three no-proof modes are unreachable under `prove =
        // true`. `detect_mode` would only return them when called
        // with `prove = false`. If we ever see one here it means
        // drive's detect_mode contract changed unexpectedly;
        // surface a clear internal error rather than crash.
        DocumentCountMode::Total
        | DocumentCountMode::PerInValue
        | DocumentCountMode::RangeNoProof => Err(drive_proof_verifier::Error::RequestError {
            error: format!(
                "unexpected no-proof DocumentCountMode {resolved_mode:?} returned for a \
                 prove=true request — drive's detect_mode contract may have changed"
            ),
        }),
    }
}

/// Build the SQL-shape [`CountMode`] from `(group_by,
/// where_clauses)`. Mirrors the abci handler's
/// `validate_and_route` logic so the SDK side picks the same
/// mode the server would have routed to, which keeps
/// [`DriveDocumentCountQuery::detect_mode`]'s output (and the
/// proof-verification primitive) in sync end-to-end.
///
/// Match-any semantics on the field lookups (`is_in_field` /
/// `is_range_field`) — clause ordering on the wire must not
/// affect routing, same fix as the abci handler's round-3
/// regression.
fn resolve_count_mode(
    group_by: &[String],
    where_clauses: &[drive::query::WhereClause],
) -> Result<CountMode, drive_proof_verifier::Error> {
    let is_in_field = |field: &str| {
        where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In && wc.field == field)
    };
    let is_range_field = |field: &str| {
        where_clauses
            .iter()
            .any(|wc| DriveDocumentCountQuery::is_range_operator(wc.operator) && wc.field == field)
    };
    let unsupported = |feature: String| drive_proof_verifier::Error::RequestError {
        error: format!("{feature} (see issue #3655 for the v1 wire surface follow-ups)"),
    };
    match group_by {
        [] => Ok(CountMode::Aggregate),
        [field] => {
            if is_in_field(field) {
                Ok(CountMode::GroupByIn)
            } else if is_range_field(field) {
                Ok(CountMode::GroupByRange)
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
                Ok(CountMode::GroupByCompound)
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

/// Wrap a single `u64` from an aggregate proof primitive
/// (`AggregateCountOnRange` or `verify_primary_key_count_tree_proof`)
/// as a one-element `Vec<SplitCountEntry>` so callers see a
/// uniform shape regardless of which primitive verified the
/// proof.
fn single_empty_key_entry(count: u64) -> Vec<SplitCountEntry> {
    vec![SplitCountEntry {
        in_key: None,
        key: Vec::new(),
        count: Some(count),
    }]
}
