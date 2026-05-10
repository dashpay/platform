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
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters, platform_value::Value,
    prelude::DataContract, ProtocolError,
};
use drive::query::{DriveDocumentCountQuery, DriveDocumentQuery, WhereClause, WhereOperator};
use drive_proof_verifier::{DocumentCount, DocumentSplitCounts, FromProof};
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportError, TransportRequest,
};

/// SDK-side query for the `GetDocumentsCount` endpoint.
///
/// Wraps a [`DocumentQuery`] (so we can reuse its [`DriveDocumentQuery`]
/// conversion machinery) and is consumed by [`DocumentCount::fetch`].
///
/// Optional fields below correspond to the unified count endpoint's
/// pagination / distinct-mode knobs added in PR #3623. Defaults match
/// the gRPC defaults: total-count summed result, ascending order,
/// no limit, no cursor, proof-verifying transport. Setters override
/// individual fields without disturbing the rest.
#[derive(Debug, Clone, dash_platform_macros::Mockable)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct DocumentCountQuery {
    /// Underlying document query — the count endpoint takes the same
    /// data-contract / document-type / where-clauses inputs as the
    /// regular document query.
    pub document_query: DocumentQuery,
    /// `return_distinct_counts_in_range` request flag. Only meaningful
    /// when the where clauses contain a range operator AND the
    /// request goes through a no-proof transport — the proof
    /// endpoint rejects this combination because the merk-level
    /// `AggregateCountOnRange` proof returns a single aggregate.
    /// Default: `false`.
    pub return_distinct_counts_in_range: bool,
    /// `order_by_ascending` request flag. `None` (default) means the
    /// server uses the natural BTreeMap order (ascending) for
    /// distinct-mode entries; `Some(false)` reverses.
    pub order_by_ascending: Option<bool>,
    /// `limit` cap for distinct-mode entries. The server clamps this
    /// to its `max_query_limit` config; passing a larger value here
    /// just gets clamped, not rejected.
    pub limit: Option<u32>,
    /// `start_after_split_key` pagination cursor for distinct-mode
    /// entries. Skips up to AND including this serialized key, in
    /// the requested order.
    pub start_after_split_key: Option<Vec<u8>>,
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
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
        })
    }

    /// Add a where clause to the underlying query.
    pub fn with_where(mut self, clause: WhereClause) -> Self {
        self.document_query = self.document_query.with_where(clause);
        self
    }

    /// Set `return_distinct_counts_in_range`. Only meaningful with a
    /// range where-clause AND a no-proof transport (see field doc).
    pub fn with_distinct_counts_in_range(mut self, distinct: bool) -> Self {
        self.return_distinct_counts_in_range = distinct;
        self
    }

    /// Set the sort order for distinct-mode entries. `None` (default)
    /// means ascending; `Some(false)` reverses.
    pub fn with_order_by_ascending(mut self, ascending: Option<bool>) -> Self {
        self.order_by_ascending = ascending;
        self
    }

    /// Cap distinct-mode entry count. Server clamps to its
    /// `max_query_limit` config — larger values are silently reduced.
    pub fn with_limit(mut self, limit: Option<u32>) -> Self {
        self.limit = limit;
        self
    }

    /// Pagination cursor: skip distinct-mode entries up to and
    /// including this serialized key, in the requested order.
    pub fn with_start_after_split_key(mut self, cursor: Option<Vec<u8>>) -> Self {
        self.start_after_split_key = cursor;
        self
    }
}

impl<'a> From<&'a DriveDocumentQuery<'a>> for DocumentCountQuery {
    fn from(value: &'a DriveDocumentQuery<'a>) -> Self {
        Self {
            document_query: value.into(),
            return_distinct_counts_in_range: false,
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
        }
    }
}

impl<'a> From<DriveDocumentQuery<'a>> for DocumentCountQuery {
    fn from(value: DriveDocumentQuery<'a>) -> Self {
        Self {
            document_query: value.into(),
            return_distinct_counts_in_range: false,
            order_by_ascending: None,
            limit: None,
            start_after_split_key: None,
        }
    }
}

impl<'a> TryFrom<&'a DocumentCountQuery> for DriveDocumentQuery<'a> {
    type Error = Error;

    fn try_from(query: &'a DocumentCountQuery) -> Result<Self, Self::Error> {
        // Force the underlying DriveDocumentQuery to be unbounded.
        //
        // The proof verifier counts documents from the verified proof, so
        // any limit set on the wrapped DocumentQuery would silently cap the
        // returned count. The server-side count handler also runs with no
        // limit, so the client must match. Without this, callers (e.g. the
        // WASM SDK, which defaults DocumentQuery.limit to 100) would see
        // a count truncated at their pagination limit instead of the actual
        // total.
        let mut drive_query: DriveDocumentQuery = (&query.document_query).try_into()?;
        drive_query.limit = None;
        Ok(drive_query)
    }
}

impl TryFrom<DocumentCountQuery> for GetDocumentsCountRequest {
    type Error = Error;

    fn try_from(query: DocumentCountQuery) -> Result<Self, Self::Error> {
        let where_bytes = serialize_where_clauses_to_cbor(&query.document_query.where_clauses)?;
        Ok(GetDocumentsCountRequest {
            version: Some(GetDocumentsCountRequestVersion::V0(
                GetDocumentsCountRequestV0 {
                    data_contract_id: query.document_query.data_contract.id().to_vec(),
                    document_type: query.document_query.document_type_name.clone(),
                    r#where: where_bytes,
                    return_distinct_counts_in_range: query.return_distinct_counts_in_range,
                    order_by_ascending: query.order_by_ascending,
                    limit: query.limit,
                    start_after_split_key: query.start_after_split_key.clone(),
                    // SDK Fetch path always requests a proof; users
                    // wanting no-proof distinct-mode would need a
                    // separate transport entry point that doesn't
                    // try to verify the response as a proof.
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
        let request: GetDocumentsCountRequest = self
            .try_into()
            .expect("DocumentCountQuery should always be valid");
        request.execute_transport(client, settings)
    }
}

impl FromProof<DocumentCountQuery> for DocumentCount {
    type Request = DocumentCountQuery;
    type Response = GetDocumentsCountResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();

        // Range queries arrive with a grovedb `AggregateCountOnRange`
        // proof (produced by `Drive::execute_document_count_range_proof`),
        // which the materialize-and-count verifier below cannot decode.
        // The merk-level verifier `GroveDb::verify_aggregate_count_query`
        // is gated to grovedb's `feature = "minimal"`, not `"verify"`,
        // so it isn't reachable from rs-drive-proof-verifier today.
        // Wiring this up requires an upstream grovedb feature-gate
        // change; until then, surface a clear error directing callers
        // to either:
        // - Use `prove = false` for range counts (no SDK gap), or
        // - Build the path-query via
        //   `DriveDocumentCountQuery::aggregate_count_path_query` and
        //   call `GroveDb::verify_aggregate_count_query` directly with
        //   `grovedb` pulled in under `feature = "minimal"`.
        //
        // The path-builder is intentionally kept in rs-drive under
        // `cfg(any(server, verify))` so direct callers don't have to
        // duplicate it.
        if request
            .document_query
            .where_clauses
            .iter()
            .any(|wc| DriveDocumentCountQuery::is_range_operator(wc.operator))
        {
            return Err(drive_proof_verifier::Error::RequestError {
                error: "AggregateCountOnRange proof verification is not yet wired in the SDK \
                        (grovedb's verify_aggregate_count_query is gated to feature = \"minimal\", \
                        not \"verify\"). Use prove = false for range counts, or call \
                        GroveDb::verify_aggregate_count_query directly with the path query \
                        from DriveDocumentCountQuery::aggregate_count_path_query."
                    .to_string(),
            });
        }

        let drive_query: DriveDocumentQuery =
            (&request)
                .try_into()
                .map_err(|e| drive_proof_verifier::Error::RequestError {
                    error: format!(
                        "Failed to convert DocumentCountQuery to DriveDocumentQuery: {}",
                        e
                    ),
                })?;

        <DocumentCount as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
            drive_query,
            response,
            network,
            platform_version,
            provider,
        )
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
        network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        Self: 'a,
    {
        let request: Self::Request = request.into();

        // The split property comes from the In clause's field name (if any).
        // No In → no split; result is a single entry with empty key.
        let split_property = request
            .document_query
            .where_clauses
            .iter()
            .find(|wc| wc.operator == WhereOperator::In)
            .map(|wc| wc.field.clone());

        let drive_query: DriveDocumentQuery =
            (&request)
                .try_into()
                .map_err(|e| drive_proof_verifier::Error::RequestError {
                    error: format!(
                        "Failed to convert DocumentCountQuery to DriveDocumentQuery: {}",
                        e
                    ),
                })?;

        if let Some(split_property) = split_property {
            DocumentSplitCounts::maybe_from_proof_with_split_property::<DriveDocumentQuery, _, _>(
                drive_query,
                &split_property,
                response,
                network,
                platform_version,
                provider,
            )
        } else {
            // Total-count case: just count documents from the proof and
            // return a single entry with empty key.
            <DocumentCount as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
                drive_query,
                response,
                network,
                platform_version,
                provider,
            )
            .map(|(opt, mtd, proof)| {
                let map = opt
                    .map(|DocumentCount(count)| {
                        let mut m = std::collections::BTreeMap::new();
                        if count > 0 {
                            m.insert(Vec::new(), count);
                        }
                        m
                    })
                    .unwrap_or_default();
                (Some(DocumentSplitCounts(map)), mtd, proof)
            })
        }
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
