//! Document search operations

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dash_sdk::dpp::document::serialization_traits::DocumentPlatformValueMethodsV0;
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::DataContract;
use dash_sdk::drive::query::{OrderClause, WhereClause, WhereOperator};
use dash_sdk::platform::{DocumentQuery, FetchMany};
use serde::{Deserialize, Serialize};
use serde_json;

use crate::sdk::SDKWrapper;
use crate::types::{DataContractHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Document search parameters
#[repr(C)]
pub struct DashSDKDocumentSearchParams {
    /// Data contract handle
    pub data_contract_handle: *const DataContractHandle,
    /// Document type name
    pub document_type: *const c_char,
    /// JSON string of where clauses (optional)
    pub where_json: *const c_char,
    /// JSON string of order by clauses (optional)
    pub order_by_json: *const c_char,
    /// Limit number of results (0 = default)
    pub limit: u32,
    /// Start from index (for pagination)
    pub start_at: u32,
}

/// JSON representation of a where clause
#[derive(Debug, Deserialize)]
struct WhereClauseJson {
    field: String,
    operator: String,
    value: serde_json::Value,
}

/// JSON representation of an order by clause
#[derive(Debug, Deserialize)]
struct OrderByClauseJson {
    field: String,
    ascending: bool,
}

/// Result containing serialized documents
#[derive(Debug, Serialize)]
struct DocumentSearchResult {
    documents: Vec<serde_json::Value>,
    total_count: usize,
}

/// Parse where operator from string
fn parse_where_operator(op: &str) -> Result<WhereOperator, FFIError> {
    match op {
        "=" | "==" | "equal" => Ok(WhereOperator::Equal),
        ">" | "gt" => Ok(WhereOperator::GreaterThan),
        ">=" | "gte" => Ok(WhereOperator::GreaterThanOrEquals),
        "<" | "lt" => Ok(WhereOperator::LessThan),
        "<=" | "lte" => Ok(WhereOperator::LessThanOrEquals),
        "in" => Ok(WhereOperator::In),
        "startsWith" => Ok(WhereOperator::StartsWith),
        // "contains" and "elementMatch" are not supported in the current version
        "contains" | "elementMatch" => Err(FFIError::InternalError(format!(
            "Operator '{}' is not supported",
            op
        ))),
        _ => Err(FFIError::InternalError(format!(
            "Unknown where operator: {}",
            op
        ))),
    }
}

/// Convert JSON value to platform value
fn json_to_platform_value(json: serde_json::Value) -> Result<Value, FFIError> {
    match json {
        serde_json::Value::Null => Ok(Value::Null),
        serde_json::Value::Bool(b) => Ok(Value::Bool(b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::I64(i))
            } else if let Some(u) = n.as_u64() {
                Ok(Value::U64(u))
            } else if let Some(f) = n.as_f64() {
                // Platform value doesn't support float, convert to string
                Ok(Value::Float(f))
            } else {
                Err(FFIError::InternalError("Invalid number value".to_string()))
            }
        }
        serde_json::Value::String(s) => Ok(Value::Text(s)),
        serde_json::Value::Array(arr) => {
            let values: Result<Vec<Value>, _> =
                arr.into_iter().map(json_to_platform_value).collect();
            Ok(Value::Array(values?))
        }
        serde_json::Value::Object(map) => {
            let mut pairs = Vec::new();
            for (k, v) in map {
                pairs.push((Value::Text(k), json_to_platform_value(v)?));
            }
            Ok(Value::Map(pairs))
        }
    }
}

/// Search for documents
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_search(
    sdk_handle: *const SDKHandle,
    params: *const DashSDKDocumentSearchParams,
) -> DashSDKResult {
    if sdk_handle.is_null() || params.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or params is null".to_string(),
        ));
    }

    let params = &*params;

    if params.data_contract_handle.is_null() || params.document_type.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Data contract handle or document type is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let data_contract = &*(params.data_contract_handle as *const DataContract);

    // Parse document type
    let document_type_str = match CStr::from_ptr(params.document_type).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        // Create the base query
        let mut query = DocumentQuery::new(data_contract.clone(), document_type_str)
            .map_err(|e| FFIError::InternalError(format!("Failed to create query: {}", e)))?;

        // Parse and add where clauses if provided
        if !params.where_json.is_null() {
            let where_json_str = CStr::from_ptr(params.where_json)
                .to_str()
                .map_err(FFIError::from)?;

            if !where_json_str.is_empty() {
                let where_clauses: Vec<WhereClauseJson> = serde_json::from_str(where_json_str)
                    .map_err(|e| FFIError::InternalError(format!("Invalid where JSON: {}", e)))?;

                for clause in where_clauses {
                    let operator = parse_where_operator(&clause.operator)?;
                    let value = json_to_platform_value(clause.value)?;

                    query = query.with_where(WhereClause {
                        field: clause.field,
                        operator,
                        value,
                    });
                }
            }
        }

        // Parse and add order by clauses if provided
        if !params.order_by_json.is_null() {
            let order_json_str = CStr::from_ptr(params.order_by_json)
                .to_str()
                .map_err(FFIError::from)?;

            if !order_json_str.is_empty() {
                let order_clauses: Vec<OrderByClauseJson> = serde_json::from_str(order_json_str)
                    .map_err(|e| {
                        FFIError::InternalError(format!("Invalid order by JSON: {}", e))
                    })?;

                for clause in order_clauses {
                    query = query.with_order_by(OrderClause {
                        field: clause.field,
                        ascending: clause.ascending,
                    });
                }
            }
        }

        // Set limit if provided
        if params.limit > 0 {
            query.limit = params.limit;
        }

        // Note: start_at is currently not supported as it requires a document ID
        // TODO: Implement proper pagination with document IDs
        if params.start_at > 0 {
            return Err(FFIError::InternalError(
                "start_at pagination is not yet implemented. Use limit instead.".to_string(),
            ));
        }

        // Execute the query
        let documents = Document::fetch_many(&wrapper.sdk, query)
            .await
            .map_err(|e| FFIError::InternalError(format!("Failed to fetch documents: {}", e)))?;

        // Convert documents to JSON
        let mut json_documents = Vec::new();
        for (_, doc) in documents.iter() {
            if let Some(document) = doc {
                // Convert document to JSON using its to_object method
                let doc_value = document.to_object().map_err(|e| {
                    FFIError::InternalError(format!("Failed to convert document to JSON: {}", e))
                })?;
                // Convert platform value to serde_json::Value
                let json_value = serde_json::to_value(&doc_value).map_err(|e| {
                    FFIError::InternalError(format!("Failed to serialize document: {}", e))
                })?;
                json_documents.push(json_value);
            }
        }

        // Create result
        let result = DocumentSearchResult {
            documents: json_documents,
            total_count: documents.len(),
        };

        // Serialize result to JSON string
        serde_json::to_string(&result)
            .map_err(|e| FFIError::InternalError(format!("Failed to serialize result: {}", e)))
    });

    match result {
        Ok(json) => {
            let c_str = match CString::new(json) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InternalError,
                        format!("Failed to create CString: {}", e),
                    ))
                }
            };
            DashSDKResult::success(c_str.into_raw() as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}
