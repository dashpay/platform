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
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use drive::query::{
    CountMode, CountProofPlan, DriveDocumentCountQuery, ResolvedTimeRange, SelectFunction,
    WhereClause,
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

/// The refusals [`verify_count_query`] makes from the request alone, made
/// before it is sent ([`drive_proof_verifier::FromProof::check_request`]).
///
/// Pending window selections are resolved first, as the verifier resolves them
/// ([`DocumentQuery::resolved_for_checking`]), so the checks read the clauses the verifier reads.
/// The request is then routed as every node routes it ([`grouped_mode`]): a grouping no `In` or
/// range constrains, or a `limit` its mode cannot honor, is refused in the node's words, and a
/// request the node answers as a ranked or having-range page is refused in favour of the view
/// that verifies it. The last check is the verifier's proof plan, lowered to its path query
/// ([`count_proof_plan`]), which the node proves by too, so a count no index serves, or whose
/// path query cannot be built, is refused here in the node's words.
pub(super) fn check_count_request(
    request: &DocumentQuery,
    folds_entries: bool,
    platform_version: &PlatformVersion,
) -> Result<(), drive_proof_verifier::Error> {
    request.ensure_no_sub_queries()?;
    assert_select_is_count(request)?;
    let (resolved, resolved_time_ranges) = request.resolved_for_checking()?;
    let request = &resolved;
    request.ensure_no_having()?;
    let count_mode = grouped_mode(request, "COUNT", platform_version)?;
    let where_clauses =
        aggregate_request_where_clauses(request, count_mode, folds_entries, platform_version)?;
    let document_type = document_type_named(&request.data_contract, &request.document_type_name)?;
    count_proof_plan(
        request,
        &document_type,
        &where_clauses,
        &resolved_time_ranges,
        count_mode,
        platform_version,
    )
    .map(drop)
}

/// The proof plan a node makes for a proved count of `request`, made from the
/// request alone and lowered to the path query its proof answers. Drive
/// resolves the SQL-shape `count_mode` into its proof mode
/// ([`DriveDocumentCountQuery::detect_mode_versioned`]), picks the
/// primary-key route or the covering index and the limit and walk direction
/// ([`DriveDocumentCountQuery::proof_plan`]), and builds the path query
/// ([`CountProofPlan::path_query`]), which the check before sending discards
/// and the verifier's Drive call rebuilds from the plan. A client cannot know
/// a node's `max_query_limit`, so it leaves that check to the node.
///
/// `where_clauses` are the request's as the server reads them
/// ([`aggregate_where_clauses`]).
pub(super) fn count_proof_plan<'a>(
    request: &DocumentQuery,
    document_type: &'a DocumentTypeRef<'_>,
    where_clauses: &[WhereClause],
    resolved_time_ranges: &[ResolvedTimeRange],
    count_mode: CountMode,
    platform_version: &PlatformVersion,
) -> Result<CountProofPlan<'a>, drive_proof_verifier::Error> {
    let mode = DriveDocumentCountQuery::detect_mode_versioned(
        where_clauses,
        count_mode,
        true,
        platform_version,
    )
    .map_err(|e| drive_proof_verifier::Error::RequestError {
        error: format!("count-mode detection failed: {e}"),
    })?;
    let unverifiable = |e| drive_proof_verifier::Error::RequestError {
        error: format!("no answer to this count can be verified: {e}"),
    };
    let plan = DriveDocumentCountQuery::proof_plan(
        document_type,
        where_clauses,
        resolved_time_ranges,
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
        platform_version,
    )
    .map_err(unverifiable)?;
    Ok(plan)
}

/// Verify a count-shape proof and return per-branch entries.
///
/// Single source of truth for the count-proof dispatch. Picks
/// the verifier primitive by **Drive's [`CountProofPlan`]**, the
/// plan the server proved by, rather than a clause-shape heuristic,
/// so the SDK's routing matches the server's exactly. Mismatch here
/// was previously possible — e.g. `group_by = [in_field]` with a
/// co-present range clause on the prove path produces a
/// carrier-aggregate proof server-side but the old
/// `has_range && !group_by.is_empty()` heuristic routed it to
/// `verify_distinct_count_proof` (different primitive ⇒
/// verification fails).
///
/// **Routing**: route the request to a [`CountMode`] with the
/// node's own routing ([`grouped_mode`]), then take Drive's plan
/// ([`count_proof_plan`]). Branch by the plan:
///
/// - [`CountProofPlan::PrimaryKeyCountTree`] (empty where on a
///   `documents_countable: true` doctype) →
///   `verify_primary_key_count_tree_proof`.
/// - [`CountProofPlan::PointLookup`] (no range, with or without
///   `In`) → `verify_point_lookup_count_proof`.
/// - [`CountProofPlan::Range`] (range, no In, no distinct) →
///   `verify_aggregate_count_proof` → single empty-key entry.
/// - [`CountProofPlan::RangeDistinct`] (range + distinct walk via
///   `GroupByRange` / `GroupByCompound`) →
///   `verify_distinct_count_proof`.
/// - [`CountProofPlan::RangeAggregateCarrier`] (`In + range +
///   group_by=[in_field]` on the prove path; grovedb #663 carrier
///   primitive) → `verify_carrier_aggregate_count_proof`.
///
/// Wrapping aggregate primitives (`Range`, primary-key
/// CountTree) as single empty-key entries is the only shape
/// massage this helper does — the underlying primitives
/// genuinely emit `u64`s, and consumers ([`DocumentCount`] sums,
/// [`DocumentSplitCounts`] passes through) want a uniform
/// per-entry vec regardless.
///
/// [`DocumentCount`]: drive_proof_verifier::DocumentCount
/// [`DocumentSplitCounts`]: drive_proof_verifier::DocumentSplitCounts
///
/// `folds_entries` is whether the caller adds the verified entries up into one
/// total ([`aggregate_where_clauses`]).
pub(super) fn verify_count_query(
    mut request: DocumentQuery,
    response: GetDocumentsResponse,
    folds_entries: bool,
    platform_version: &PlatformVersion,
    provider: &dyn ContextProvider,
) -> Result<(Option<Vec<SplitCountEntry>>, ResponseMetadata, Proof), drive_proof_verifier::Error> {
    request.ensure_no_sub_queries()?;
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
    let count_mode = grouped_mode(&request, "COUNT", platform_version)?;
    request.where_clauses = aggregate_where_clauses(
        document_type,
        count_mode,
        std::mem::take(&mut request.where_clauses),
        folds_entries,
        platform_version,
    )?;

    // Drive's plan, the one the prover made from the same request: the
    // proof mode, the primary-key route or covering index, and the limit
    // the path query carries. A plan the verifier made apart from the
    // prover's would build a different path query.
    let plan = count_proof_plan(
        &request,
        &document_type,
        &request.where_clauses,
        &resolved_time_ranges,
        count_mode,
        platform_version,
    )?;
    let contract_id = request.data_contract.id().to_buffer();
    let count_query = |index| DriveDocumentCountQuery {
        document_type,
        contract_id,
        document_type_name: request.document_type_name.clone(),
        index,
        where_clauses: request.where_clauses.clone(),
    };

    let entries = match plan {
        CountProofPlan::PrimaryKeyCountTree => {
            single_empty_key_entry(verify_primary_key_count_tree_proof(
                contract_id,
                &request.document_type_name,
                proof,
                mtd,
                platform_version,
                provider,
            )?)
        }
        CountProofPlan::PointLookup(index) => verify_point_lookup_count_proof(
            &count_query(index),
            proof,
            mtd,
            platform_version,
            provider,
        )?,
        CountProofPlan::Range(index) => single_empty_key_entry(verify_aggregate_count_proof(
            &count_query(index),
            proof,
            mtd,
            platform_version,
            provider,
        )?),
        CountProofPlan::RangeDistinct {
            index,
            limit,
            left_to_right,
        } => verify_distinct_count_proof(
            &count_query(index),
            proof,
            mtd,
            limit,
            left_to_right,
            platform_version,
            provider,
        )?,
        // Carrier-ACOR (grovedb #663) — one verified `u64` per present In
        // branch.
        CountProofPlan::RangeAggregateCarrier {
            index,
            limit,
            left_to_right,
        } => verify_carrier_aggregate_count_proof(
            &count_query(index),
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
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::{DataContractFactory, TokenConfiguration};
    use dpp::platform_value::platform_value;
    use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
    use drive::query::{SelectProjection, TimeRangeSelector, WhereClause, WhereOperator};
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
        assert_eq!(resolutions[0].field(), CREATED_AT);
        let grid = resolutions[0]
            .transform
            .time_range()
            .expect("an IN_TIME_RANGE resolution carries a time grid");
        assert_eq!(
            grid.range_seconds, RANGE_SECONDS,
            "the provenance must carry the exact grid the resolution used"
        );
        assert_eq!(grid.step_seconds, STEP_SECONDS);
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

        let error = verify_count_query(
            request,
            response,
            false,
            platform_version(),
            &UnusedProvider,
        )
        .expect_err("an unproven response must not be accepted");
        assert!(
            matches!(error, drive_proof_verifier::Error::NoProofInResult),
            "expected NoProofInResult, got: {error:?}"
        );
    }
}
