//! `GetDocuments*` requests.

use dapi_grpc::platform::v0::{self as platform_proto, Proof, ResponseMetadata};
use drive::query::{OrderClause, StartClause, WhereClause};

use crate::{platform::IncompleteMessage, transport::TransportRequest, DapiRequest, Settings};

fn to_proto_where_clause(
    where_clause: WhereClause,
) -> platform_proto::get_documents_request::WhereClause {
    let mut value_cbor_bytes = Vec::new();
    ciborium::into_writer(&where_clause.value, &mut value_cbor_bytes)
        .expect("byte slice is a safe writer");

    platform_proto::get_documents_request::WhereClause {
        field: where_clause.field,
        operator: where_clause.operator as i32,
        value: value_cbor_bytes,
    }
}

fn to_proto_order_clause(
    order_clause: OrderClause,
) -> platform_proto::get_documents_request::OrderClause {
    platform_proto::get_documents_request::OrderClause {
        field: order_clause.field,
        ascending: order_clause.ascending,
    }
}

fn to_proto_start(start_clause: StartClause) -> platform_proto::get_documents_request::Start {
    use platform_proto::get_documents_request::Start as ProtoStart;
    match start_clause {
        StartClause::StartAt(x) => ProtoStart::StartAt(x.to_vec()),
        StartClause::StartAfter(x) => ProtoStart::StartAfter(x.to_vec()),
    }
}

/// Request documents bytes.
#[derive(Debug)]
pub struct GetDocuments {
    /// Data contract ID
    pub data_contract_id: Vec<u8>,
    /// Document type for the data contract
    pub document_type: String,
    /// `where` clauses for the query
    pub where_clauses: Vec<WhereClause>,
    /// `order_by` clauses for the query
    pub order_by_clauses: Vec<OrderClause>,
    /// queryset limit
    pub limit: u32,
    /// TODO start
    pub start: Option<StartClause>,
}

impl DapiRequest for GetDocuments {
    type DapiResponse = GetDocumentsResponse;

    const SETTINGS_OVERRIDES: Settings = Settings::default();

    type Error = IncompleteMessage;

    type TransportRequest = platform_proto::GetDocumentsRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        platform_proto::GetDocumentsRequest {
            data_contract_id: self.data_contract_id.clone(),
            document_type: self.document_type.clone(),
            r#where: self
                .where_clauses
                .clone()
                .into_iter()
                .map(to_proto_where_clause)
                .collect(),
            order_by: self
                .order_by_clauses
                .clone()
                .into_iter()
                .map(to_proto_order_clause)
                .collect(),
            limit: self.limit,
            prove: false,
            start: self.start.clone().map(to_proto_start),
        }
    }

    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        use platform_proto::get_documents_response::{Documents, Result as GrpcResponseBody};
        use platform_proto::GetDocumentsResponse as GrpcResponse;

        match transport_response {
            GrpcResponse {
                metadata: Some(metadata),
                result: Some(GrpcResponseBody::Documents(Documents { documents })),
            } => Ok(GetDocumentsResponse {
                documents_bytes: documents,
                metadata,
            }),
            _ => Err(IncompleteMessage),
        }
    }
}

/// DAPI response for [GetDocumentsRequest].
#[derive(Debug)]
pub struct GetDocumentsResponse {
    /// Serialized Documents
    pub documents_bytes: Vec<Vec<u8>>,
    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Request documents with proof.
#[derive(Debug)]
pub struct GetDocumentsProof {
    /// Data contract ID
    pub data_contract_id: Vec<u8>,
    /// Document type for the data contract
    pub document_type: String,
    /// `where` clauses for the query
    pub where_clauses: Vec<WhereClause>,
    /// `order_by` clauses for the query
    pub order_by_clauses: Vec<OrderClause>,
    /// queryset limit
    pub limit: u32,
    /// TODO start
    pub start: Option<StartClause>,
}

impl DapiRequest for GetDocumentsProof {
    type DapiResponse = GetDocumentsProofResponse;

    const SETTINGS_OVERRIDES: Settings = Settings::default();

    type Error = IncompleteMessage;

    type TransportRequest = platform_proto::GetDocumentsRequest;

    fn to_transport_request(&self) -> Self::TransportRequest {
        platform_proto::GetDocumentsRequest {
            data_contract_id: self.data_contract_id.clone(),
            document_type: self.document_type.clone(),
            r#where: self
                .where_clauses
                .clone()
                .into_iter()
                .map(to_proto_where_clause)
                .collect(),
            order_by: self
                .order_by_clauses
                .clone()
                .into_iter()
                .map(to_proto_order_clause)
                .collect(),
            limit: self.limit,
            prove: true,
            start: self.start.clone().map(to_proto_start),
        }
    }

    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error> {
        use platform_proto::get_documents_response::Result as GrpcResponseBody;
        use platform_proto::GetDocumentsResponse as GrpcResponse;

        match transport_response {
            GrpcResponse {
                result: Some(GrpcResponseBody::Proof(proof)),
                metadata: Some(metadata),
            } => Ok(GetDocumentsProofResponse { proof, metadata }),
            _ => Err(IncompleteMessage),
        }
    }
}

/// DAPI response for [GetDocumentsProofRequest].
#[derive(Debug)]
pub struct GetDocumentsProofResponse {
    /// Proof data that wraps Documents
    pub proof: Proof,
    /// Response metadata
    pub metadata: ResponseMetadata,
}
