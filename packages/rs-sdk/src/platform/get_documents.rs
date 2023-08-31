//! Get documents request

// TODO: Move to rs-sdk

use dapi_grpc::platform::v0::{self as platform_proto};
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters, prelude::DataContract, ProtocolError,
};
use drive::{
    error::{drive::DriveError, query::QuerySyntaxError},
    query::{DriveQuery, InternalClauses, OrderClause, WhereClause},
};

use crate::error::Error;

/// Request documents.
// TODO: is it needed or we use drivequery?
#[derive(Debug)]
pub struct GetDocumentsRequest {
    /// Data contract ID
    pub data_contract: DataContract,
    /// Document type for the data contract
    pub document_type_name: String,
    /// `where` clauses for the query
    // TODO: implement
    pub where_clauses: Vec<WhereClause>,
    /// `order_by` clauses for the query
    // TODO: implement
    pub order_by_clauses: Vec<OrderClause>,
    /// queryset limit
    pub limit: u32,
}

impl From<GetDocumentsRequest> for platform_proto::GetDocumentsRequest {
    fn from(dapi_request: GetDocumentsRequest) -> Self {
        // TODO implement where and order_by clauses
        let empty: Vec<u8> = Vec::new();
        let empty = dpp::util::cbor_serializer::serializable_value_to_cbor(&empty, None)
            .expect("serialize empty vec");

        platform_proto::GetDocumentsRequest {
            data_contract_id: dapi_request.data_contract.id().to_vec(),
            document_type: dapi_request.document_type_name,
            r#where: empty.clone(),  // TODO: Implement
            order_by: empty.clone(), // TODO: Implement
            limit: dapi_request.limit,
            prove: true,
            start: None, // TODO: Implement
        }
    }
}
impl<'a> TryFrom<&'a GetDocumentsRequest> for DriveQuery<'a> {
    type Error = crate::error::Error;

    fn try_from(request: &'a GetDocumentsRequest) -> Result<Self, Self::Error> {
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
