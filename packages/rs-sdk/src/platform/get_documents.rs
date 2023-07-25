//! `GetDocuments*` requests.

use dapi_grpc::platform::v0::{self as platform_proto};
use drive::query::{OrderClause, StartClause, WhereClause};

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

/// Request documents.
// TODO: is it needed or we use drivequery?
#[derive(Debug)]
pub struct GetDocumentsRequest {
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

impl From<GetDocumentsRequest> for platform_proto::GetDocumentsRequest {
    fn from(dapi_request: GetDocumentsRequest) -> Self {
        platform_proto::GetDocumentsRequest {
            data_contract_id: dapi_request.data_contract_id,
            document_type: dapi_request.document_type,
            r#where: dapi_request
                .where_clauses
                .into_iter()
                .map(to_proto_where_clause)
                .collect(),
            order_by: dapi_request
                .order_by_clauses
                .into_iter()
                .map(to_proto_order_clause)
                .collect(),
            limit: dapi_request.limit,
            prove: true,
            start: dapi_request.start.map(to_proto_start),
        }
    }
}
