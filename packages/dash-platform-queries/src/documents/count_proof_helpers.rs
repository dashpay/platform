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
    mut request: DocumentQuery,
    response: GetDocumentsResponse,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(Option<Vec<SplitCountEntry>>, ResponseMetadata, Proof), drive_proof_verifier::Error> {
    let proof = response
        .proof()
        .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
    let mtd = response
        .metadata()
        .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;

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
            &resolved_time_ranges,
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
            &resolved_time_ranges,
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

#[cfg(test)]
mod tests {
    //! Offline tests for the client half of a time-range
    //! (`IN_TIME_RANGE`) query: where the bucket comes from, what
    //! provenance the resolution hands back, and how the count
    //! surface behaves when a response cannot be verified. Nothing
    //! here touches a proof — this crate builds drive with `verify`
    //! only, so no Drive exists to prove against. The prove→verify
    //! round trips (overlapping-window counts, a tampered signed
    //! time, the documents route) live in rs-drive-abci's
    //! `time_range_proof_verification`, where a populated platform
    //! does.
    //!
    //! The property under test throughout is that the bucket is a
    //! function of the **quorum-signed** metadata time and of the
    //! contract's declared window, and of nothing else the client
    //! could pick: a client-local clock would let a node answer with
    //! whatever bucket it liked and still verify.

    use super::*;
    use crate::documents::document_query::resolve_time_range_clauses_with_metadata_time;
    use dapi_grpc::platform::v0::get_documents_response::get_documents_response_v1::{
        count_results, result_data, CountResults, ResultData,
    };
    use dapi_grpc::platform::v0::get_documents_response::{
        get_documents_response_v1, GetDocumentsResponseV1, Version as ResponseVersion,
    };
    use dash_context_provider::ContextProviderError;
    use dpp::data_contract::{DataContractFactory, TokenConfiguration};
    use dpp::platform_value::platform_value;
    use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use drive::query::{SelectProjection, TimeRangeSelector, WhereClause};
    use std::sync::Arc;

    const DOCUMENT_TYPE: &str = "post";
    const CREATED_AT: &str = "$createdAt";
    const BUCKETED_INDEX: &str = "trending";
    /// Six-hour windows sliding every two hours — overlap factor 3, the
    /// shape a trending leaderboard actually declares.
    const RANGE_SECONDS: u64 = 6 * 3_600;
    const STEP_SECONDS: u64 = 2 * 3_600;
    const STEP_MS: u64 = STEP_SECONDS * 1_000;
    /// An exact multiple of the two-hour step, so on the `phase: 0` grid it
    /// is itself a bucket start.
    const BUCKET_START_MS: u64 = 1_755_000_000_000;
    /// A one-hour phase — strictly less than the step, as validation
    /// requires — for the epoch-sliver refusal test.
    const PHASE_SECONDS: u64 = 3_600;
    const PHASE_MS: u64 = PHASE_SECONDS * 1_000;

    fn platform_version() -> &'static PlatformVersion {
        PlatformVersion::latest()
    }

    /// Never consulted by these tests — every one of them fails (or is
    /// asserted) before a proof reaches the tenderdash binding. It exists
    /// because [`verify_count_query`] takes a provider by reference.
    struct UnusedProvider;

    impl ContextProvider for UnusedProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            Ok(None)
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            Ok([0u8; 48])
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }
    }

    /// A contract whose `post` doctype carries a `countable` bucketed index
    /// over `(timeRange($createdAt), hashtag)`. `phase_seconds` is a
    /// parameter because the epoch-sliver refusal is one of the behaviours
    /// under test.
    fn trending_contract(phase_seconds: u64) -> Arc<DataContract> {
        let schemas = platform_value!({
            "post": {
                "type": "object",
                "properties": {
                    "hashtag": { "type": "string", "maxLength": 63, "position": 0 },
                },
                "indices": [
                    {
                        "name": "trending",
                        "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
                        "countable": true,
                        "timeRange": {
                            "on": "$createdAt",
                            "range": RANGE_SECONDS,
                            "step": STEP_SECONDS,
                            "phase": phase_seconds,
                        },
                    },
                ],
                "required": ["$createdAt", "hashtag"],
                "additionalProperties": false,
            }
        });
        let contract = DataContractFactory::new(platform_version().protocol_version)
            .expect("expected a factory")
            .create_with_value_config(Identifier::new([7u8; 32]), 0, schemas, None, None)
            .expect("the trending contract is well-formed")
            .data_contract_owned();
        Arc::new(contract)
    }

    /// The bucket start the contract's own transform puts `time_ms` in —
    /// the expectation is derived from the declared window rather than
    /// restated as a literal, so a fixture edit cannot leave it behind.
    fn expected_bucket(contract: &DataContract, time_ms: u64) -> u64 {
        contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("post doctype exists")
            .indexes()
            .get(BUCKETED_INDEX)
            .expect("the bucketed index survives contract creation")
            .time_range
            .as_ref()
            .expect("the bucketed index carries its transform")
            .newest_active_start(time_ms)
            .expect("the metadata time is inside an active range")
    }

    fn newest_bucket_query(contract: Arc<DataContract>) -> DocumentQuery {
        DocumentQuery::new(contract, DOCUMENT_TYPE)
            .expect("the fixture has this document type")
            .with_time_range(CREATED_AT, TimeRangeSelector::Newest)
    }

    fn resolved_equality(request: &DocumentQuery) -> &WhereClause {
        let matching: Vec<_> = request
            .where_clauses
            .iter()
            .filter(|clause| clause.field == CREATED_AT)
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "resolution must push exactly one clause on the bucketed field"
        );
        matching[0]
    }

    /// The whole contract of the resolution step: the pending selector is
    /// consumed, an ordinary equality on the bucketed field appears in its
    /// place, and the field name comes back as provenance — which is the
    /// only thing that will later keep index selection on the bucketed
    /// index, since the pushed clause is indistinguishable from a
    /// hand-written raw-timestamp lookup.
    #[test]
    fn the_newest_selector_resolves_to_the_bucket_containing_the_metadata_time() {
        let contract = trending_contract(0);
        let mut request = newest_bucket_query(Arc::clone(&contract));
        let metadata_time_ms = BUCKET_START_MS + 3_600_000;

        let resolutions =
            resolve_time_range_clauses_with_metadata_time(&mut request, metadata_time_ms)
                .expect("a metadata time inside an active range resolves");

        assert_eq!(resolutions.len(), 1);
        assert_eq!(resolutions[0].field, CREATED_AT);
        assert_eq!(
            resolutions[0].transform.range_seconds, RANGE_SECONDS,
            "the provenance must carry the exact grid the resolution used"
        );
        assert_eq!(resolutions[0].transform.step_seconds, STEP_SECONDS);
        assert!(
            request.time_range_clauses.is_empty(),
            "the pending selector must be drained, not left to be encoded twice"
        );
        let clause = resolved_equality(&request);
        assert_eq!(clause.operator, WhereOperator::Equal);
        assert_eq!(
            clause.value,
            dpp::platform_value::Value::U64(expected_bucket(&contract, metadata_time_ms))
        );
        assert_eq!(
            clause.value,
            dpp::platform_value::Value::U64(BUCKET_START_MS),
            "one hour past a bucket start is still inside that bucket"
        );
    }

    /// The same query against a metadata time one full step later resolves
    /// one bucket later — pinning that the bucket is derived from the signed
    /// time rather than from anything the client holds. If the resolution
    /// ever started reading a local clock this assertion is what breaks.
    #[test]
    fn a_metadata_time_one_step_later_resolves_to_the_next_bucket() {
        let contract = trending_contract(0);
        let earlier_time_ms = BUCKET_START_MS + 3_600_000;

        let mut earlier = newest_bucket_query(Arc::clone(&contract));
        resolve_time_range_clauses_with_metadata_time(&mut earlier, earlier_time_ms)
            .expect("a metadata time inside an active range resolves");

        let mut later = newest_bucket_query(Arc::clone(&contract));
        resolve_time_range_clauses_with_metadata_time(&mut later, earlier_time_ms + STEP_MS)
            .expect("a metadata time inside an active range resolves");

        let earlier_bucket = resolved_equality(&earlier)
            .value
            .to_integer::<u64>()
            .expect("a bucket start is a millisecond timestamp");
        let later_bucket = resolved_equality(&later)
            .value
            .to_integer::<u64>()
            .expect("a bucket start is a millisecond timestamp");
        assert_eq!(
            later_bucket,
            earlier_bucket + STEP_MS,
            "one step of signed time must move the resolution exactly one bucket"
        );
    }

    /// A metadata time inside the epoch sliver before the grid's phase
    /// anchor belongs to no range at all. No real block time reaches it, but
    /// the client must refuse rather than invent a bucket, mirroring the
    /// server, which refuses the same request at resolution time — so the
    /// two sides cannot disagree about whether the query was answerable.
    #[test]
    fn a_metadata_time_in_the_epoch_sliver_refuses_to_resolve() {
        let contract = trending_contract(PHASE_SECONDS);
        let mut request = newest_bucket_query(contract);

        let error = resolve_time_range_clauses_with_metadata_time(&mut request, PHASE_MS - 1)
            .expect_err("a time predating every range has no honest bucket");

        match error {
            drive_proof_verifier::Error::RequestError { error } => assert!(
                error.contains("phase"),
                "expected the epoch-sliver refusal, got: {error}"
            ),
            other => panic!("expected a request error, got: {other:?}"),
        }
    }

    /// A node that answers a `prove = true` time-range count with data
    /// instead of a proof must be rejected as an unproven response, not
    /// mistaken for a query the client failed to reconstruct: the proof
    /// check is the first gate in [`verify_count_query`], ahead of the
    /// time-range resolution and the provenance-shape guard. Callers
    /// therefore get `NoProofInResult` — "this node did not prove it" —
    /// rather than a resolution error that would send them looking at their
    /// own query.
    #[test]
    fn a_count_response_carrying_no_proof_is_rejected_as_unproven() {
        let contract = trending_contract(0);
        let request = newest_bucket_query(contract)
            .with_select(SelectProjection::count_star())
            .with_where(WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::Equal,
                value: dpp::platform_value::Value::Text("ibiza".to_string()),
            });

        let response = GetDocumentsResponse {
            version: Some(ResponseVersion::V1(GetDocumentsResponseV1 {
                result: Some(get_documents_response_v1::Result::Data(ResultData {
                    variant: Some(result_data::Variant::Counts(CountResults {
                        variant: Some(count_results::Variant::AggregateCount(2)),
                    })),
                })),
                metadata: Some(ResponseMetadata {
                    time_ms: BUCKET_START_MS + 3_600_000,
                    ..Default::default()
                }),
            })),
        };

        let error = verify_count_query(request, response, platform_version(), &UnusedProvider)
            .expect_err("an unproven response must not be accepted");
        assert!(
            matches!(error, drive_proof_verifier::Error::NoProofInResult),
            "expected NoProofInResult, got: {error:?}"
        );
    }
}
