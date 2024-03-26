//! Method to query documents from the Drive.

use std::sync::Arc;

use crate::{error::Error, sdk::Sdk};
use ciborium::Value as CborValue;
use dapi_grpc::platform::v0::get_documents_request::Version::V0;
use dapi_grpc::platform::v0::{
    self as platform_proto,
    get_documents_request::{get_documents_request_v0::Start, GetDocumentsRequestV0},
    GetDocumentsRequest, ResponseMetadata,
};
use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters, document_type::accessors::DocumentTypeV0Getters,
    },
    document::Document,
    platform_value::{platform_value, Value},
    prelude::{DataContract, Identifier},
    ProtocolError,
};
use drive::query::{DriveQuery, InternalClauses, OrderClause, WhereClause, WhereOperator};
use drive_proof_verifier::{types::Documents, FromProof};
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportClient, TransportRequest,
};

use super::fetch::Fetch;

// TODO: remove DocumentQuery once ContextProvider that provides data contracts is merged.

/// Request that is used to query documents from the Dash Platform.
///
/// This is an abstraction layer built on top of [GetDocumentsRequest] to address issues with missing details
/// required to correctly verify proofs returned by the Dash Platform.
///
/// Conversions are implemented between this type, [GetDocumentsRequest] and [DriveQuery] using [TryFrom] trait.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, dapi_grpc_macros::Mockable)]
pub struct DocumentQuery {
    /// Data contract ID
    pub data_contract: Arc<DataContract>,
    /// Document type for the data contract
    pub document_type_name: String,
    /// `where` clauses for the query
    pub where_clauses: Vec<WhereClause>,
    /// `order_by` clauses for the query
    pub order_by_clauses: Vec<OrderClause>,
    /// queryset limit
    pub limit: u32,
    /// first object to start with
    pub start: Option<Start>,
}

impl DocumentQuery {
    /// Create new DocumentQuery for provided contract and document type name.
    pub fn new<C: Into<Arc<DataContract>>>(
        contract: C,
        document_type_name: &str,
    ) -> Result<Self, Error> {
        let contract = contract.into();
        // ensure document type name is correct
        contract
            .document_type_for_name(document_type_name)
            .map_err(ProtocolError::DataContractError)?;

        Ok(Self {
            data_contract: Arc::clone(&contract),
            document_type_name: document_type_name.to_string(),
            where_clauses: vec![],
            order_by_clauses: vec![],
            limit: 0,
            start: None,
        })
    }

    /// Create new document query based on a [DriveQuery].
    pub fn new_with_drive_query(d: &DriveQuery) -> Self {
        Self::from(d)
    }

    /// Create new document query for provided document type name and data contract ID.
    ///
    /// Note that this method will fetch data contract first.
    pub async fn new_with_data_contract_id(
        api: &Sdk,
        data_contract_id: Identifier,
        document_type_name: &str,
    ) -> Result<Self, Error> {
        let data_contract =
            DataContract::fetch(api, data_contract_id)
                .await?
                .ok_or(Error::MissingDependency(
                    "DataContract".to_string(),
                    format!("data contract {} not found", data_contract_id),
                ))?;

        Self::new(data_contract, document_type_name)
    }

    /// Point to a specific document ID.
    pub fn with_document_id(self, document_id: &Identifier) -> Self {
        let clause = WhereClause {
            field: "$id".to_string(),
            operator: WhereOperator::Equal,
            value: platform_value!(document_id),
        };

        self.with_where(clause)
    }

    /// Add new where clause to the query.
    ///
    /// Existing where clauses will be preserved.
    pub fn with_where(mut self, clause: WhereClause) -> Self {
        self.where_clauses.push(clause);

        self
    }

    /// Add order by clause to the query.
    ///
    /// Existing order by clauses will be preserved.
    pub fn with_order_by(mut self, clause: OrderClause) -> Self {
        self.order_by_clauses.push(clause);

        self
    }
}

impl TransportRequest for DocumentQuery {
    type Client = <GetDocumentsRequest as TransportRequest>::Client;
    type Response = <GetDocumentsRequest as TransportRequest>::Response;
    const SETTINGS_OVERRIDES: rs_dapi_client::RequestSettings =
        <GetDocumentsRequest as TransportRequest>::SETTINGS_OVERRIDES;

    fn request_name(&self) -> &'static str {
        "GetDocumentsRequest"
    }

    fn method_name(&self) -> &'static str {
        "get_documents"
    }

    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, <Self::Client as TransportClient>::Error>> {
        let request: GetDocumentsRequest = self
            .try_into()
            .expect("DocumentQuery should always be valid");
        request.execute_transport(client, settings)
    }
}

impl FromProof<DocumentQuery> for Document {
    type Request = DocumentQuery;
    type Response = platform_proto::GetDocumentsResponse;
    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        version: &dpp::version::PlatformVersion,
        provider: &'a dyn drive_proof_verifier::ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata), drive_proof_verifier::Error>
    where
        Self: Sized + 'a,
    {
        let request: Self::Request = request.into();

        let (documents, metadata): (Option<Documents>, ResponseMetadata) =
            <Documents as FromProof<Self::Request>>::maybe_from_proof_with_metadata(
                request, response, version, provider,
            )?;

        match documents {
            None => Ok((None, metadata)),
            Some(docs) => match docs.len() {
                0 | 1 => Ok((docs.into_iter().next().and_then(|(_, v)| v), metadata)),
                n => Err(drive_proof_verifier::Error::ResponseDecodeError {
                    error: format!("expected 1 element, got {}", n),
                }),
            },
        }
    }
}

impl FromProof<DocumentQuery> for drive_proof_verifier::types::Documents {
    type Request = DocumentQuery;
    type Response = platform_proto::GetDocumentsResponse;
    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        version: &dpp::version::PlatformVersion,
        provider: &'a dyn drive_proof_verifier::ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata), drive_proof_verifier::Error>
    where
        Self: Sized + 'a,
    {
        let request: Self::Request = request.into();
        let drive_query: DriveQuery =
            (&request)
                .try_into()
                .map_err(|e| drive_proof_verifier::Error::RequestDecodeError {
                    error: format!("Failed to convert DocumentQuery to DriveQuery: {}", e),
                })?;

        <drive_proof_verifier::types::Documents as FromProof<DriveQuery>>::maybe_from_proof_with_metadata(
            drive_query,
            response,
            version,
            provider,
        )
    }
}

impl TryFrom<DocumentQuery> for platform_proto::GetDocumentsRequest {
    type Error = Error;
    fn try_from(dapi_request: DocumentQuery) -> Result<Self, Self::Error> {
        // TODO implement where and order_by clause

        let where_clauses = serialize_vec_to_cbor(dapi_request.where_clauses.clone())
            .expect("where clauses serialization should never fail");
        let order_by = serialize_vec_to_cbor(dapi_request.order_by_clauses.clone())?;
        // Order clause

        //todo: transform this into PlatformVersionedTryFrom
        Ok(GetDocumentsRequest {
            version: Some(V0(GetDocumentsRequestV0 {
                data_contract_id: dapi_request.data_contract.id().to_vec(),
                document_type: dapi_request.document_type_name.clone(),
                r#where: where_clauses,
                order_by,
                limit: dapi_request.limit,
                prove: true,
                start: dapi_request.start.clone(),
            })),
        })
    }
}

impl<'a> From<&'a DriveQuery<'a>> for DocumentQuery {
    fn from(value: &'a DriveQuery<'a>) -> Self {
        let data_contract = value.contract.clone();
        let document_type_name = value.document_type.name();
        let where_clauses = value.internal_clauses.clone().into();
        let order_by_clauses = value.order_by.iter().map(|(_, v)| v.clone()).collect();
        let limit = value.limit.unwrap_or(0) as u32;

        let start = if let Some(start_at) = value.start_at {
            match value.start_at_included {
                true => Some(Start::StartAt(start_at.to_vec())),
                false => Some(Start::StartAfter(start_at.to_vec())),
            }
        } else {
            None
        };

        Self {
            data_contract: Arc::new(data_contract),
            document_type_name: document_type_name.to_string(),
            where_clauses,
            order_by_clauses,
            limit,
            start,
        }
    }
}

impl<'a> From<DriveQuery<'a>> for DocumentQuery {
    fn from(value: DriveQuery<'a>) -> Self {
        let data_contract = value.contract.clone();
        let document_type_name = value.document_type.name();
        let where_clauses = value.internal_clauses.clone().into();
        let order_by_clauses = value.order_by.iter().map(|(_, v)| v.clone()).collect();
        let limit = value.limit.unwrap_or(0) as u32;

        let start = if let Some(start_at) = value.start_at {
            match value.start_at_included {
                true => Some(Start::StartAt(start_at.to_vec())),
                false => Some(Start::StartAfter(start_at.to_vec())),
            }
        } else {
            None
        };

        Self {
            data_contract: Arc::new(data_contract),
            document_type_name: document_type_name.to_string(),
            where_clauses,
            order_by_clauses,
            limit,
            start,
        }
    }
}

impl<'a> TryFrom<&'a DocumentQuery> for DriveQuery<'a> {
    type Error = crate::error::Error;

    fn try_from(request: &'a DocumentQuery) -> Result<Self, Self::Error> {
        // let data_contract = request.data_contract.clone();
        let document_type = request
            .data_contract
            .document_type_for_name(&request.document_type_name)
            .map_err(ProtocolError::DataContractError)?;

        let internal_clauses = InternalClauses::extract_from_clauses(request.where_clauses.clone())
            .map_err(Error::Drive)?;

        let limit = if request.limit != 0 {
            Some(request.limit as u16)
        } else {
            None
        };
        let query = Self {
            contract: &request.data_contract,
            document_type,
            internal_clauses,
            offset: None,
            limit,
            order_by: request
                .order_by_clauses
                .clone()
                .into_iter()
                .map(|v| (v.field.clone(), v))
                .collect(),
            start_at: None,
            start_at_included: false,
            block_time_ms: None,
        };

        Ok(query)
    }
}

fn serialize_vec_to_cbor<T: Into<Value>>(input: Vec<T>) -> Result<Vec<u8>, Error> {
    let values = Value::Array(
        input
            .into_iter()
            .map(|v| v.into() as Value)
            .collect::<Vec<Value>>(),
    );

    let cbor_values: CborValue = TryInto::<CborValue>::try_into(values)
        .map_err(|e| Error::Protocol(dpp::ProtocolError::EncodingError(e.to_string())))?;

    let mut serialized = Vec::new();
    ciborium::ser::into_writer(&cbor_values, &mut serialized)
        .map_err(|e| Error::Protocol(dpp::ProtocolError::EncodingError(e.to_string())))?;

    Ok(serialized)
}
