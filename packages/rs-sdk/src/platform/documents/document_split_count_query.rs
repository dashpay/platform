//! High-level SDK query for [`GetDocumentsSplitCountRequest`].
//!
//! Adds a `split_property` parameter on top of the inputs accepted by
//! [`super::document_count_query::DocumentCountQuery`]: the index property
//! whose values partition the count.

use std::sync::Arc;

use crate::error::Error;
use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::Fetch;
use ciborium::Value as CborValue;
use dapi_grpc::platform::v0::get_documents_split_count_request::{
    GetDocumentsSplitCountRequestV0, Version as GetDocumentsSplitCountRequestVersion,
};
use dapi_grpc::platform::v0::{
    GetDocumentsSplitCountRequest, GetDocumentsSplitCountResponse, Proof, ResponseMetadata,
};
use dash_context_provider::ContextProvider;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters, platform_value::Value,
    prelude::DataContract, ProtocolError,
};
use drive::query::{DriveDocumentQuery, WhereClause};
use drive_proof_verifier::{DocumentSplitCounts, FromProof};
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportError, TransportRequest,
};

/// SDK-side query for the `GetDocumentsSplitCount` endpoint.
///
/// Same shape as [`DocumentCountQuery`](super::document_count_query::DocumentCountQuery),
/// plus a `split_property` field naming the index property whose distinct
/// values partition the returned counts.
#[derive(Debug, Clone, dash_platform_macros::Mockable)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct DocumentSplitCountQuery {
    /// Underlying document query.
    pub document_query: DocumentQuery,
    /// Index property whose distinct values partition the counts.
    pub split_property: String,
}

impl DocumentSplitCountQuery {
    /// Build a split-count query.
    pub fn new<C: Into<Arc<DataContract>>>(
        contract: C,
        document_type_name: &str,
        split_property: impl Into<String>,
    ) -> Result<Self, Error> {
        Ok(Self {
            document_query: DocumentQuery::new(contract, document_type_name)?,
            split_property: split_property.into(),
        })
    }

    /// Add a where clause to the underlying query.
    pub fn with_where(mut self, clause: WhereClause) -> Self {
        self.document_query = self.document_query.with_where(clause);
        self
    }
}

impl<'a> TryFrom<&'a DocumentSplitCountQuery> for DriveDocumentQuery<'a> {
    type Error = Error;

    fn try_from(query: &'a DocumentSplitCountQuery) -> Result<Self, Self::Error> {
        (&query.document_query).try_into()
    }
}

impl TryFrom<DocumentSplitCountQuery> for GetDocumentsSplitCountRequest {
    type Error = Error;

    fn try_from(query: DocumentSplitCountQuery) -> Result<Self, Self::Error> {
        let where_bytes = serialize_where_clauses_to_cbor(&query.document_query.where_clauses)?;
        Ok(GetDocumentsSplitCountRequest {
            version: Some(GetDocumentsSplitCountRequestVersion::V0(
                GetDocumentsSplitCountRequestV0 {
                    data_contract_id: query.document_query.data_contract.id().to_vec(),
                    document_type: query.document_query.document_type_name.clone(),
                    r#where: where_bytes,
                    split_count_by_index_property: query.split_property.clone(),
                    prove: true,
                },
            )),
        })
    }
}

impl TransportRequest for DocumentSplitCountQuery {
    type Client = <GetDocumentsSplitCountRequest as TransportRequest>::Client;
    type Response = <GetDocumentsSplitCountRequest as TransportRequest>::Response;
    const SETTINGS_OVERRIDES: rs_dapi_client::RequestSettings =
        <GetDocumentsSplitCountRequest as TransportRequest>::SETTINGS_OVERRIDES;

    fn request_name(&self) -> &'static str {
        "GetDocumentsSplitCountRequest"
    }

    fn method_name(&self) -> &'static str {
        "get_documents_split_count"
    }

    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        let request: GetDocumentsSplitCountRequest = self
            .try_into()
            .expect("DocumentSplitCountQuery should always be valid");
        request.execute_transport(client, settings)
    }
}

impl FromProof<DocumentSplitCountQuery> for DocumentSplitCounts {
    type Request = DocumentSplitCountQuery;
    type Response = GetDocumentsSplitCountResponse;

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
        let drive_query: DriveDocumentQuery =
            (&request)
                .try_into()
                .map_err(|e| drive_proof_verifier::Error::RequestError {
                    error: format!(
                        "Failed to convert DocumentSplitCountQuery to DriveDocumentQuery: {}",
                        e
                    ),
                })?;

        <DocumentSplitCounts as FromProof<DriveDocumentQuery>>::maybe_from_proof_with_metadata(
            drive_query,
            response,
            network,
            platform_version,
            provider,
        )
    }
}

impl Fetch for DocumentSplitCounts {
    type Request = DocumentSplitCountQuery;
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
