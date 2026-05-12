//! High-level SDK query for [`GetDocumentsCountRequest`].
//!
//! [`DocumentCountQuery`] mirrors [`super::document_query::DocumentQuery`] for
//! the new count endpoint introduced by PR #3435: it wraps the data contract,
//! document type, and where clauses, converts to the gRPC request for
//! transport, and converts to a [`DriveDocumentQuery`] for proof verification.

use std::sync::Arc;

use crate::error::Error;
use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::Fetch;
use ciborium::Value as CborValue;
use dapi_grpc::platform::v0::get_documents_count_request::{
    GetDocumentsCountRequestV0, Version as GetDocumentsCountRequestVersion,
};
use dapi_grpc::platform::v0::{
    GetDocumentsCountRequest, GetDocumentsCountResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters,
    data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters},
    platform_value::Value,
    prelude::DataContract,
    ProtocolError,
};
use drive::query::{
    DriveDocumentCountQuery, DriveDocumentQuery, OrderClause, WhereClause, WhereOperator,
};
use drive_proof_verifier::{
    verify_aggregate_count_proof, verify_distinct_count_proof, verify_point_lookup_count_proof,
    verify_primary_key_count_tree_proof, DocumentCount, DocumentSplitCounts, FromProof,
};
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportError, TransportRequest,
};

/// SDK-side query for the `GetDocumentsCount` endpoint.
///
/// Wraps a [`DocumentQuery`] (so we can reuse its [`DriveDocumentQuery`]
/// conversion machinery) and is consumed by [`DocumentCount::fetch`].
///
/// Field defaults match the gRPC defaults: total-count summed result,
/// ascending order, no limit, proof-verifying transport. Setters
/// override individual fields without disturbing the rest.
#[derive(Debug, Clone, dash_platform_macros::Mockable)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct DocumentCountQuery {
    /// Underlying document query — the count endpoint takes the same
    /// data-contract / document-type / where-clauses inputs as the
    /// regular document query.
    pub document_query: DocumentQuery,
    /// `return_distinct_counts_in_range` request flag. Meaningful
    /// when the where clauses contain a range operator: routes the
    /// request to the per-distinct-value execution path on both
    /// no-proof (`RangeNoProof`) AND prove (`RangeDistinctProof`)
    /// transports. The prove path returns a regular range proof
    /// against the property-name `ProvableCountTree` whose `KVCount`
    /// ops carry per-distinct-value counts; the SDK's
    /// `FromProof<DocumentCountQuery>` for `DocumentSplitCounts`
    /// extracts them via `verify_distinct_count_proof`. Default:
    /// `false`.
    pub return_distinct_counts_in_range: bool,
    /// `limit` cap for distinct-mode entries.
    /// - **No-proof paths**: server clamps to its `max_query_limit`
    ///   config; passing a larger value just gets clamped, not
    ///   rejected.
    /// - **Prove path** (`RangeDistinctProof`): validate-don't-clamp.
    ///   `limit > max_query_limit` is rejected by the server with
    ///   `Error::Query(QuerySyntaxError::InvalidLimit(...))` because
    ///   silent clamping would invisibly break proof verification.
    ///   Unset falls back to `drive::config::DEFAULT_QUERY_LIMIT`
    ///   (the same compile-time constant the SDK verifier reads),
    ///   so proof bytes are deterministic across operators
    ///   regardless of their runtime `default_query_limit` tuning.
    ///
    /// No cursor field: pagination is expressed by narrowing the
    /// underlying range itself (`color > <last-key-from-previous-
    /// page>`), which is equivalent in expressivity and avoids the
    /// ambiguity a single-`bytes` cursor would have for compound
    /// (`In + range + distinct`) queries whose natural sort is
    /// `(in_key, key)`.
    pub limit: Option<u32>,
    // Order direction lives on the wrapped `document_query` —
    // `DocumentQuery::order_by_clauses` is serialized into the
    // request's `order_by` field. The first clause's direction
    // controls split-mode entry ordering server-side; clauses are
    // also load-bearing for `(In + prove)` walk determinism (see the
    // `FromProof` impl below).
}

impl DocumentCountQuery {
    /// Build a count query from a contract reference and document type name.
    pub fn new<C: Into<Arc<DataContract>>>(
        contract: C,
        document_type_name: &str,
    ) -> Result<Self, Error> {
        Ok(Self {
            document_query: DocumentQuery::new(contract, document_type_name)?,
            return_distinct_counts_in_range: false,
            limit: None,
        })
    }

    /// Add a where clause to the underlying query.
    pub fn with_where(mut self, clause: WhereClause) -> Self {
        self.document_query = self.document_query.with_where(clause);
        self
    }

    /// Add an order_by clause to the underlying query. The first
    /// clause's direction controls split-mode entry ordering
    /// server-side and is part of the path query bytes on the
    /// `RangeDistinctProof` prove path (so prover and verifier must
    /// agree; empty `order_by` defaults to ascending on both sides).
    /// Unused on the `PointLookupProof` path — the builder sorts In
    /// keys lex-ascending unconditionally for prove/no-proof parity.
    pub fn with_order_by(mut self, clause: OrderClause) -> Self {
        self.document_query = self.document_query.with_order_by(clause);
        self
    }

    /// Set `return_distinct_counts_in_range`. Meaningful with a
    /// range where-clause on both no-proof and prove transports
    /// (see field doc).
    pub fn with_distinct_counts_in_range(mut self, distinct: bool) -> Self {
        self.return_distinct_counts_in_range = distinct;
        self
    }

    /// Cap distinct-mode entry count.
    /// - No-proof paths: server clamps to its `max_query_limit`.
    /// - Prove path: server rejects `limit > max_query_limit` with
    ///   `InvalidLimit` rather than clamping silently (clamping
    ///   would invisibly break verification). Unset falls back to
    ///   `drive::config::DEFAULT_QUERY_LIMIT`, the same compile-time
    ///   constant the SDK verifier uses — see the field doc for
    ///   the deterministic-across-operators rationale.
    pub fn with_limit(mut self, limit: Option<u32>) -> Self {
        self.limit = limit;
        self
    }
}

impl<'a> From<&'a DriveDocumentQuery<'a>> for DocumentCountQuery {
    fn from(value: &'a DriveDocumentQuery<'a>) -> Self {
        Self {
            document_query: value.into(),
            return_distinct_counts_in_range: false,
            limit: None,
        }
    }
}

impl<'a> From<DriveDocumentQuery<'a>> for DocumentCountQuery {
    fn from(value: DriveDocumentQuery<'a>) -> Self {
        Self {
            document_query: value.into(),
            return_distinct_counts_in_range: false,
            limit: None,
        }
    }
}

impl TryFrom<DocumentCountQuery> for GetDocumentsCountRequest {
    type Error = Error;

    fn try_from(query: DocumentCountQuery) -> Result<Self, Self::Error> {
        let where_bytes = serialize_where_clauses_to_cbor(&query.document_query.where_clauses)?;
        let order_by_bytes =
            serialize_order_by_clauses_to_cbor(&query.document_query.order_by_clauses)?;
        Ok(GetDocumentsCountRequest {
            version: Some(GetDocumentsCountRequestVersion::V0(
                GetDocumentsCountRequestV0 {
                    data_contract_id: query.document_query.data_contract.id().to_vec(),
                    document_type: query.document_query.document_type_name.clone(),
                    r#where: where_bytes,
                    return_distinct_counts_in_range: query.return_distinct_counts_in_range,
                    order_by: order_by_bytes,
                    limit: query.limit,
                    // **Count Fetch always proves.** The SDK `Fetch`
                    // path is wired through `FromProof<DocumentCountQuery>`,
                    // which only knows how to decode the `Proof(...)`
                    // response variant — the no-proof `Counts(...)` /
                    // `Entries(...)` variants need a different decoder
                    // entry point that doesn't exist yet on the SDK
                    // side. Setting this to anything other than
                    // `true` would either silently fail at decode
                    // time or strip the verification guarantee the
                    // rest of the SDK assumes.
                    //
                    // `SdkBuilder::with_proofs(false)` is consequently
                    // a **no-op** for `DocumentCountQuery` — the
                    // blanket `Query<T> for T` impl in
                    // `packages/rs-sdk/src/platform/query.rs:119-124`
                    // emits a `tracing::warn!` at `Fetch::fetch`
                    // time when proofs are disabled, but the request
                    // still ships with `prove: true` and the
                    // response is decoded through
                    // `FromProof<DocumentCountQuery>`. The server's
                    // unified `GetDocumentsCount` endpoint supports
                    // no-proof modes (`Total` / `PerInValue` /
                    // `RangeNoProof`) but the SDK has no typed
                    // decoder for them yet — shadowing the blanket
                    // impl to intercept the flag is blocked by
                    // Rust's coherence rules (`Query<T> for T`
                    // covers all `T: TransportRequest`, and
                    // `DocumentCountQuery` IS its own
                    // `TransportRequest`). Wiring a no-proof
                    // decoder is tracked as
                    // dashpay/platform#3630.
                    prove: true,
                },
            )),
        })
    }
}

impl TransportRequest for DocumentCountQuery {
    type Client = <GetDocumentsCountRequest as TransportRequest>::Client;
    type Response = <GetDocumentsCountRequest as TransportRequest>::Response;
    const SETTINGS_OVERRIDES: rs_dapi_client::RequestSettings =
        <GetDocumentsCountRequest as TransportRequest>::SETTINGS_OVERRIDES;

    fn request_name(&self) -> &'static str {
        "GetDocumentsCountRequest"
    }

    fn method_name(&self) -> &'static str {
        "get_documents_count"
    }

    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        // CBOR-serializing the where clauses can fail on values that
        // aren't representable (the conversion goes through ciborium).
        // Surface that as a recoverable transport error rather than
        // panicking — callers expect `Fetch` failures to be matchable
        // on `Error::DapiClientError`, not aborts.
        let request: GetDocumentsCountRequest = match self.try_into() {
            Ok(r) => r,
            Err(e) => {
                let status = dapi_grpc::tonic::Status::internal(format!(
                    "DocumentCountQuery -> GetDocumentsCountRequest conversion failed: {}",
                    e
                ));
                return Box::pin(async move { Err(TransportError::Grpc(status)) });
            }
        };
        request.execute_transport(client, settings)
    }
}

impl FromProof<DocumentCountQuery> for DocumentCount {
    type Request = DocumentCountQuery;
    type Response = GetDocumentsCountResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();

        // Range queries arrive with a grovedb `AggregateCountOnRange`
        // proof (produced by `Drive::execute_document_count_range_proof`)
        // that the materialize-and-count path below can't decode. Pivot
        // to the merk-level aggregate verifier instead, building the
        // exact same `PathQuery` the prover used via the shared
        // `DriveDocumentCountQuery::aggregate_count_path_query` builder
        // (kept in rs-drive under `cfg(any(server, verify))` so prover
        // and verifier never drift).
        if request
            .document_query
            .where_clauses
            .iter()
            .any(|wc| DriveDocumentCountQuery::is_range_operator(wc.operator))
        {
            let response: Self::Response = response.into();

            let document_type = request
                .document_query
                .data_contract
                .document_type_for_name(&request.document_query.document_type_name)
                .map_err(|e| drive_proof_verifier::Error::RequestError {
                    error: format!(
                        "document type {} not found in contract: {}",
                        request.document_query.document_type_name, e
                    ),
                })?;
            let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
                document_type.indexes(),
                &request.document_query.where_clauses,
            )
            .ok_or_else(|| drive_proof_verifier::Error::RequestError {
                error: "range count requires a `range_countable: true` index whose last \
                        property matches the range field"
                    .to_string(),
            })?;

            let count_query = DriveDocumentCountQuery {
                document_type,
                contract_id: request.document_query.data_contract.id().to_buffer(),
                document_type_name: request.document_query.document_type_name.clone(),
                index,
                where_clauses: request.document_query.where_clauses.clone(),
            };
            let proof = response
                .proof()
                .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
            let mtd = response
                .metadata()
                .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;

            // Dispatch on `return_distinct_counts_in_range`. The
            // server's `detect_mode` routes
            // `(range, prove=true, distinct=true)` to
            // `RangeDistinctProof` (emits per-key `KVCount` ops) and
            // `(range, prove=true, distinct=false)` to `RangeProof`
            // (emits a single `AggregateCountOnRange` aggregate);
            // the two proof shapes are NOT interchangeable.
            // Decoding a distinct proof with the aggregate verifier
            // would fail merk-root recomputation because the path
            // queries differ structurally.
            if request.return_distinct_counts_in_range {
                // Mirror the SDK's prove-distinct dispatcher (see the
                // `FromProof<DocumentCountQuery> for DocumentSplitCounts`
                // impl below) to rebuild the same path query the
                // prover signed. The limit anchors to the compile-time
                // `DEFAULT_QUERY_LIMIT` constant (matching the
                // server's `drive_dispatcher.rs` `RangeDistinctProof`
                // arm) so proof bytes are deterministic across
                // operators. Direction comes from the first
                // `order_by` clause, defaulting to ascending.
                let limit_u16 = match request.limit {
                    Some(l) => {
                        u16::try_from(l).map_err(|_| drive_proof_verifier::Error::RequestError {
                            error: format!(
                                "limit {} exceeds u16::MAX; the prove-distinct path query \
                                 cannot represent it",
                                l
                            ),
                        })?
                    }
                    None => drive::config::DEFAULT_QUERY_LIMIT,
                };
                let left_to_right = request
                    .document_query
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
                // `DocumentCount` collapses to a single aggregate
                // u64. Sum the verified per-key counts. The proof's
                // `KVCount` ops are merk-root-bound via
                // `node_hash_with_count`, so the sum is
                // cryptographically committed — same forge-resistance
                // as `AggregateCountOnRange`, just expressed as a
                // post-verification reduction in Rust.
                let total: u64 = entries.iter().map(|e| e.count).sum();
                return Ok((Some(DocumentCount(total)), mtd.clone(), proof.clone()));
            }

            // Range + prove + !distinct: aggregate proof path. The
            // verifier helper rebuilds the prover's path query
            // internally via `count_query.aggregate_count_path_query`
            // — same builder both sides share, so the path query
            // bytes match byte-for-byte and the merk root
            // recomputation succeeds.
            let count =
                verify_aggregate_count_proof(&count_query, proof, mtd, platform_version, provider)?;
            return Ok((Some(DocumentCount(count)), mtd.clone(), proof.clone()));
        }

        // No range clause: route through the count-tree proof
        // primitives. Two sub-cases mirror the server-side dispatch:
        //
        // 1. **documents_countable + empty where**: the doctype's
        //    primary-key tree is itself a CountTree. The server
        //    proves that element directly; the SDK verifies and
        //    extracts `count_value`. O(log n) proof, no index.
        // 2. **Else**: must have a `countable: true` index whose
        //    properties exactly match the where clauses. Server
        //    proves the per-branch CountTree elements; SDK sums their
        //    `count_value`s. Rejection on missing covering index is
        //    symmetric with the no-proof side.
        let response: Self::Response = response.into();
        let document_type = request
            .document_query
            .data_contract
            .document_type_for_name(&request.document_query.document_type_name)
            .map_err(|e| drive_proof_verifier::Error::RequestError {
                error: format!(
                    "document type {} not found in contract: {}",
                    request.document_query.document_type_name, e
                ),
            })?;
        let proof = response
            .proof()
            .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
        let mtd = response
            .metadata()
            .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;

        // documents_countable fast path
        if request.document_query.where_clauses.is_empty() && document_type.documents_countable() {
            let contract_id = request.document_query.data_contract.id().to_buffer();
            let count = verify_primary_key_count_tree_proof(
                contract_id,
                &request.document_query.document_type_name,
                proof,
                mtd,
                platform_version,
                provider,
            )?;
            return Ok((Some(DocumentCount(count)), mtd.clone(), proof.clone()));
        }

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &request.document_query.where_clauses,
        )
        .ok_or_else(|| drive_proof_verifier::Error::RequestError {
            error: "prove count requires a `countable: true` index whose properties \
                    exactly match the where clause fields, or `documentsCountable: \
                    true` on the document type for unfiltered total counts"
                .to_string(),
        })?;
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id: request.document_query.data_contract.id().to_buffer(),
            document_type_name: request.document_query.document_type_name.clone(),
            index,
            where_clauses: request.document_query.where_clauses.clone(),
        };

        let entries =
            verify_point_lookup_count_proof(&count_query, proof, mtd, platform_version, provider)?;
        // `DocumentCount` is a single aggregate u64 — sum the per-
        // branch CountTree entries. For Equal-only fully-covered the
        // verifier returns a single entry (empty `key`) and the sum
        // is just that entry's count; for Equal-prefix + In-on-last
        // it sums the per-In-value counts. A branch with zero docs is
        // omitted by the verifier so missing entries contribute 0.
        let total: u64 = entries.iter().map(|e| e.count).sum();
        Ok((Some(DocumentCount(total)), mtd.clone(), proof.clone()))
    }
}

impl Fetch for DocumentCount {
    type Request = DocumentCountQuery;
}

/// Per-key counts view of the unified count endpoint.
///
/// Backed by the same [`DocumentCountQuery`] as [`DocumentCount`]; the only
/// difference is response shape — `DocumentSplitCounts` returns the full
/// `entries` map keyed by the splitting property's serialized value, while
/// `DocumentCount` returns the sum.
///
/// Splitting is signalled by an `In` where-clause on the request: the field
/// of that clause becomes the split property and each value in the array
/// becomes one entry in the result. Without an `In` clause the response is
/// a single entry with empty key (i.e., the total count).
impl FromProof<DocumentCountQuery> for DocumentSplitCounts {
    type Request = DocumentCountQuery;
    type Response = GetDocumentsCountResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();

        // `has_in` controls the single-empty-key-entry guarantee on
        // the no-range prove path: Equal-only fully-covered queries
        // promise one entry with empty key (the verified count, even
        // if zero); In-on-last queries promise one entry per emitted
        // In value (zero-count branches are simply absent).
        let has_in = request
            .document_query
            .where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In);

        let has_range = request
            .document_query
            .where_clauses
            .iter()
            .any(|wc| DriveDocumentCountQuery::is_range_operator(wc.operator));

        // Range + distinct (with or without In on prefix): per-
        // distinct-value counts via a regular merk range proof
        // (no `AggregateCountOnRange` wrapper). The proof's
        // `KVCount` ops carry per-`(in_key, key)` counts that the
        // merk root commits to via `node_hash_with_count`, so
        // `verify_distinct_count_proof` runs the standard hash
        // chain check and reads the counts back as a verified
        // `Vec<SplitCountEntry>`. For compound queries the In
        // value is preserved in each entry's `in_key` — callers can
        // reduce by `key` via `DocumentSplitCounts::into_flat_map`
        // if they want the merged-histogram shape. Only reachable
        // when the SDK builder set
        // `with_distinct_counts_in_range(true)`.
        if has_range && request.return_distinct_counts_in_range {
            let response: Self::Response = response.into();

            let document_type = request
                .document_query
                .data_contract
                .document_type_for_name(&request.document_query.document_type_name)
                .map_err(|e| drive_proof_verifier::Error::RequestError {
                    error: format!(
                        "document type {} not found in contract: {}",
                        request.document_query.document_type_name, e
                    ),
                })?;
            let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
                document_type.indexes(),
                &request.document_query.where_clauses,
            )
            .ok_or_else(|| drive_proof_verifier::Error::RequestError {
                error: "distinct range count requires a `range_countable: true` index whose \
                        last property matches the range field"
                    .to_string(),
            })?;

            let count_query = DriveDocumentCountQuery {
                document_type,
                contract_id: request.document_query.data_contract.id().to_buffer(),
                document_type_name: request.document_query.document_type_name.clone(),
                index,
                where_clauses: request.document_query.where_clauses.clone(),
            };
            // Match the prover's defaults for limit and order so
            // the verifier helper can rebuild the same path query
            // internally. The server's prove-distinct dispatcher
            // anchors its fallback to `crate::config::DEFAULT_QUERY_LIMIT`
            // (the same compile-time constant we read here) and
            // rejects any value above its `max_query_limit` —
            // explicitly NOT the operator-tunable
            // `drive_config.default_query_limit`, since the SDK
            // can't know an operator's tuned config. With both
            // sides anchored to the shared constant, the path
            // query bytes match regardless of operator configuration.
            // See `drive_dispatcher.rs`'s `RangeDistinctProof` arm
            // for the symmetric reasoning on the server side.
            //
            // Direction comes from the first `order_by` clause; empty
            // `order_by` defaults to ascending — the server's
            // prove-distinct dispatcher derives `left_to_right` from
            // the same source (see drive_dispatcher.rs), so both
            // sides must land on the same value or the merk-root
            // recomputation fails.
            // Use `try_from` so a caller passing
            // `limit > u16::MAX` fails loudly at the SDK boundary
            // rather than silently truncating to a wrong value the
            // verifier would then build a mismatched path query
            // against. The server-side guard in
            // `drive_dispatcher.rs`'s `RangeDistinctProof` arm
            // already rejects `effective_limit > max_query_limit`
            // (and `max_query_limit` is itself a `u16`), so today
            // the truncation path is only hypothetical — but
            // defense-in-depth keeps the failure mode explicit if
            // a future code path widens the wire limit type or
            // lifts the server cap.
            let limit_u16 = match request.limit {
                Some(l) => {
                    u16::try_from(l).map_err(|_| drive_proof_verifier::Error::RequestError {
                        error: format!(
                            "limit {} exceeds u16::MAX; the prove-distinct path query cannot \
                             represent it",
                            l
                        ),
                    })?
                }
                None => drive::config::DEFAULT_QUERY_LIMIT,
            };
            let left_to_right = request
                .document_query
                .order_by_clauses
                .first()
                .map(|c| c.ascending)
                .unwrap_or(true);

            let proof = response
                .proof()
                .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
            let mtd = response
                .metadata()
                .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;

            let entries = verify_distinct_count_proof(
                &count_query,
                proof,
                mtd,
                limit_u16,
                left_to_right,
                platform_version,
                provider,
            )?;
            return Ok((
                Some(DocumentSplitCounts::from_verified(entries)),
                mtd.clone(),
                proof.clone(),
            ));
        }

        // No range clause + `prove = true`: route through the count-
        // tree proof primitives, mirroring `DocumentCount`'s dispatch.
        // Two sub-cases:
        //
        // 1. **documents_countable + empty where**: prove the
        //    doctype's primary-key CountTree directly. Result is a
        //    single empty-key entry with the verified count.
        // 2. **Else**: require a covering countable index. Server
        //    proves the per-branch CountTree elements; SDK returns
        //    them as Vec<SplitCountEntry>. For Equal-only fully-
        //    covered the verifier returns one empty-key entry
        //    (re-emitted as zero-count if absent); for Equal-prefix
        //    + In-on-last it returns one entry per In value (zero-
        //    count In branches are simply absent).
        let response: Self::Response = response.into();
        let document_type = request
            .document_query
            .data_contract
            .document_type_for_name(&request.document_query.document_type_name)
            .map_err(|e| drive_proof_verifier::Error::RequestError {
                error: format!(
                    "document type {} not found in contract: {}",
                    request.document_query.document_type_name, e
                ),
            })?;
        let proof = response
            .proof()
            .or(Err(drive_proof_verifier::Error::NoProofInResult))?;
        let mtd = response
            .metadata()
            .or(Err(drive_proof_verifier::Error::EmptyResponseMetadata))?;

        // documents_countable fast path → single empty-key entry.
        if request.document_query.where_clauses.is_empty() && document_type.documents_countable() {
            let contract_id = request.document_query.data_contract.id().to_buffer();
            let count = verify_primary_key_count_tree_proof(
                contract_id,
                &request.document_query.document_type_name,
                proof,
                mtd,
                platform_version,
                provider,
            )?;
            let entries = vec![drive_proof_verifier::SplitCountEntry {
                in_key: None,
                key: Vec::new(),
                count,
            }];
            return Ok((
                Some(DocumentSplitCounts::from_verified(entries)),
                mtd.clone(),
                proof.clone(),
            ));
        }

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &request.document_query.where_clauses,
        )
        .ok_or_else(|| drive_proof_verifier::Error::RequestError {
            error: "prove count requires a `countable: true` index whose properties \
                    exactly match the where clause fields, or `documentsCountable: \
                    true` on the document type for unfiltered total counts"
                .to_string(),
        })?;
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id: request.document_query.data_contract.id().to_buffer(),
            document_type_name: request.document_query.document_type_name.clone(),
            index,
            where_clauses: request.document_query.where_clauses.clone(),
        };

        let mut entries =
            verify_point_lookup_count_proof(&count_query, proof, mtd, platform_version, provider)?;
        // Total-count case (Equal-only fully-covered) MUST surface as
        // a single empty-key entry — callers distinguish "verified
        // zero" from "no proof returned" purely by structure. If the
        // verifier dropped the entry because count was 0, re-emit it.
        if !has_in && entries.is_empty() {
            entries.push(drive_proof_verifier::SplitCountEntry {
                in_key: None,
                key: Vec::new(),
                count: 0,
            });
        }
        Ok((
            Some(DocumentSplitCounts::from_verified(entries)),
            mtd.clone(),
            proof.clone(),
        ))
    }
}

impl Fetch for DocumentSplitCounts {
    type Request = DocumentCountQuery;
}

fn serialize_where_clauses_to_cbor(clauses: &[WhereClause]) -> Result<Vec<u8>, Error> {
    if clauses.is_empty() {
        return Ok(Vec::new());
    }

    let value_array = Value::Array(clauses.iter().cloned().map(Value::from).collect());

    let cbor_value: CborValue = TryInto::<CborValue>::try_into(value_array)
        .map_err(|e| Error::Protocol(ProtocolError::EncodingError(e.to_string())))?;

    let mut serialized = Vec::new();
    ciborium::ser::into_writer(&cbor_value, &mut serialized)
        .map_err(|e| Error::Protocol(ProtocolError::EncodingError(e.to_string())))?;

    Ok(serialized)
}

/// CBOR-encode an order_by clause list for the
/// `GetDocumentsCountRequestV0.order_by` field. Mirrors
/// [`serialize_where_clauses_to_cbor`]; empty → empty bytes (the
/// server treats that as `Value::Null` = no clauses).
fn serialize_order_by_clauses_to_cbor(clauses: &[OrderClause]) -> Result<Vec<u8>, Error> {
    if clauses.is_empty() {
        return Ok(Vec::new());
    }

    let value_array = Value::Array(clauses.iter().cloned().map(Value::from).collect());

    let cbor_value: CborValue = TryInto::<CborValue>::try_into(value_array)
        .map_err(|e| Error::Protocol(ProtocolError::EncodingError(e.to_string())))?;

    let mut serialized = Vec::new();
    ciborium::ser::into_writer(&cbor_value, &mut serialized)
        .map_err(|e| Error::Protocol(ProtocolError::EncodingError(e.to_string())))?;

    Ok(serialized)
}
