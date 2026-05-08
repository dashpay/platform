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
use drive::query::{DriveDocumentQuery, WhereClause};
use drive_proof_verifier::{DocumentCount, FromProof};
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportError, TransportRequest,
};

/// SDK-side query for the `GetDocumentsCount` endpoint.
///
/// Wraps a [`DocumentQuery`] (so we can reuse its [`DriveDocumentQuery`]
/// conversion machinery) and is consumed by [`DocumentCount::fetch`].
#[derive(Debug, Clone, dash_platform_macros::Mockable)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub struct DocumentCountQuery {
    /// Underlying document query — the count endpoint takes the same
    /// data-contract / document-type / where-clauses inputs as the
    /// regular document query.
    pub document_query: DocumentQuery,
}

impl DocumentCountQuery {
    /// Build a count query from a contract reference and document type name.
    pub fn new<C: Into<Arc<DataContract>>>(
        contract: C,
        document_type_name: &str,
    ) -> Result<Self, Error> {
        Ok(Self {
            document_query: DocumentQuery::new(contract, document_type_name)?,
        })
    }

    /// Add a where clause to the underlying query.
    pub fn with_where(mut self, clause: WhereClause) -> Self {
        self.document_query = self.document_query.with_where(clause);
        self
    }
}

impl<'a> From<&'a DriveDocumentQuery<'a>> for DocumentCountQuery {
    fn from(value: &'a DriveDocumentQuery<'a>) -> Self {
        Self {
            document_query: value.into(),
        }
    }
}

impl<'a> From<DriveDocumentQuery<'a>> for DocumentCountQuery {
    fn from(value: DriveDocumentQuery<'a>) -> Self {
        Self {
            document_query: value.into(),
        }
    }
}

impl<'a> TryFrom<&'a DocumentCountQuery> for DriveDocumentQuery<'a> {
    type Error = Error;

    fn try_from(query: &'a DocumentCountQuery) -> Result<Self, Self::Error> {
        (&query.document_query).try_into()
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
