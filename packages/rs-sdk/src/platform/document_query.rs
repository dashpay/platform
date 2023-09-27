//! Get documents request

// TODO: Move to rs-sdk

use crate::{error::Error, sdk::Sdk};
use ciborium::Value as CborValue;
use dapi_grpc::platform::v0::{
    self as platform_proto, get_documents_request::Start, GetDocumentsRequest,
};
use dpp::{
    data_contract::{
        accessors::v0::DataContractV0Getters, document_type::accessors::DocumentTypeV0Getters,
    },
    document::Document,
    platform_value::{platform_value, Value},
    prelude::{DataContract, Identifier},
};
use drive::query::{DriveQuery, InternalClauses, OrderClause, WhereClause, WhereOperator};
use drive_proof_verifier::FromProof;
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportClient, TransportRequest,
};

use super::fetch::Fetch;

/// Request documents.
// TODO: is it needed or we use drivequery?
#[derive(Debug, Clone)]
pub struct DocumentQuery {
    /// Data contract ID
    pub data_contract: DataContract,
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
    /// Create new document query based on a [DriveQuery].
    pub fn new_with_drive_query<API: Sdk>(d: &DriveQuery) -> Self {
        Self::from(d)
    }

    /// Fetch one document with provided document ID
    pub async fn new_with_document_id<API: Sdk>(
        api: &API,
        data_contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
    ) -> Result<Self, Error> {
        let data_contract =
            DataContract::fetch(api, data_contract_id)
                .await?
                .ok_or(Error::NotFound(format!(
                    "data contract {} for document {} of type {} not found",
                    data_contract_id, document_id, document_type_name
                )))?;

        data_contract.document_type_for_name(&document_type_name)?;

        let where_clauses = vec![WhereClause {
            field: "id".to_string(),
            operator: WhereOperator::Equal,
            value: platform_value!(document_id),
        }];

        // Order clause
        let order_by_clauses = vec![OrderClause {
            ascending: true,
            field: "id".to_string(),
        }];

        Ok(DocumentQuery {
            data_contract: data_contract.into(),
            document_type_name: document_type_name.to_string(),
            where_clauses,
            order_by_clauses,
            start: Some(Start::StartAt(document_id.to_vec())),
            limit: 1,
        })
    }
}

impl TransportRequest for DocumentQuery {
    type Client = <GetDocumentsRequest as TransportRequest>::Client;
    type Response = <GetDocumentsRequest as TransportRequest>::Response;
    const SETTINGS_OVERRIDES: rs_dapi_client::RequestSettings =
        <GetDocumentsRequest as TransportRequest>::SETTINGS_OVERRIDES;

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
    fn maybe_from_proof<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        provider: &'a dyn drive_proof_verifier::QuorumInfoProvider,
    ) -> Result<Option<Self>, drive_proof_verifier::Error>
    where
        Self: Sized + 'a,
    {
        let request: Self::Request = request.into();

        let documents: Option<Vec<Document>> =
            <Vec<Self> as FromProof<Self::Request>>::maybe_from_proof(request, response, provider)?;

        match documents {
            None => Ok(None),
            Some(mut docs) => match docs.len() {
                0 => Ok(None),
                1 => Ok(Some(docs.remove(0))),
                n => Err(drive_proof_verifier::Error::ResponseDecodeError {
                    error: format!("expected 1 element, got {}", n),
                }),
            },
        }
    }
}

impl FromProof<DocumentQuery> for Vec<Document> {
    type Request = DocumentQuery;
    type Response = platform_proto::GetDocumentsResponse;
    fn maybe_from_proof<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        provider: &'a dyn drive_proof_verifier::QuorumInfoProvider,
    ) -> Result<Option<Self>, drive_proof_verifier::Error>
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

        <drive_proof_verifier::proof::from_proof::Documents as FromProof<DriveQuery>>::maybe_from_proof(
            drive_query,
            response,
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

        Ok(platform_proto::GetDocumentsRequest {
            data_contract_id: dapi_request.data_contract.id().to_vec(),
            document_type: dapi_request.document_type_name.clone(),
            r#where: where_clauses,
            order_by,
            limit: dapi_request.limit,
            prove: true,
            start: dapi_request.start.clone(),
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
            data_contract,
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
            .document_type_for_name(&request.document_type_name)?
            .clone();

        let internal_clauses = InternalClauses::extract_from_clauses(request.where_clauses.clone())
            .map_err(|e: drive::error::Error| Error::Drive(e))?;

        let limit = if request.limit != 0 {
            Some(request.limit as u16)
        } else {
            None
        };
        let query = Self {
            contract: &request.data_contract,
            document_type: document_type.clone(),
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
