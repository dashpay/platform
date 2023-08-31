//! Get documents request

// TODO: Move to rs-sdk

use crate::error::Error;
use ciborium::Value as CborValue;
use dapi_grpc::platform::v0::{self as platform_proto, get_documents_request::Start};
use dpp::{
    data_contract::accessors::v0::DataContractV0Getters, platform_value::Value,
    prelude::DataContract,
};
use drive::query::{DriveQuery, InternalClauses, OrderClause, WhereClause};

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

    pub start: Option<Start>,
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

impl TryFrom<&DocumentQuery> for platform_proto::GetDocumentsRequest {
    type Error = Error;
    fn try_from(dapi_request: &DocumentQuery) -> Result<Self, Self::Error> {
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
