//! Document count + split-count query operations.
//!
//! Wraps the rs-sdk `DocumentCount::fetch` and `DocumentSplitCounts::fetch`
//! flows so iOS / native callers can obtain document counts without having
//! to construct `GetDocumentsCountRequest` payloads themselves.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::DataContract;
use dash_sdk::drive::query::{WhereClause, WhereOperator};
use dash_sdk::platform::documents::document_count_query::DocumentCountQuery;
use dash_sdk::platform::documents::document_query::DocumentQuery;
use dash_sdk::platform::documents::document_split_count_query::DocumentSplitCountQuery;
use dash_sdk::platform::Fetch;
use drive_proof_verifier::{DocumentCount, DocumentSplitCounts};
use serde::{Deserialize, Serialize};
use serde_json;

use crate::sdk::SDKWrapper;
use crate::types::{DataContractHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

#[derive(Debug, Deserialize)]
struct WhereClauseJson {
    field: String,
    operator: String,
    value: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct DocumentCountResult {
    count: u64,
}

#[derive(Debug, Serialize)]
struct DocumentSplitCountResult {
    /// Per-key counts. Keys are hex-encoded so iOS callers can match them
    /// against the corresponding platform-value-encoded property bytes.
    counts: BTreeMap<String, u64>,
}

#[allow(clippy::result_large_err)]
fn parse_where_operator(op: &str) -> Result<WhereOperator, FFIError> {
    match op {
        "=" | "==" | "equal" => Ok(WhereOperator::Equal),
        ">" | "gt" => Ok(WhereOperator::GreaterThan),
        ">=" | "gte" => Ok(WhereOperator::GreaterThanOrEquals),
        "<" | "lt" => Ok(WhereOperator::LessThan),
        "<=" | "lte" => Ok(WhereOperator::LessThanOrEquals),
        "in" => Ok(WhereOperator::In),
        "startsWith" => Ok(WhereOperator::StartsWith),
        _ => Err(FFIError::InternalError(format!(
            "Unknown where operator: {}",
            op
        ))),
    }
}

#[allow(clippy::result_large_err)]
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

#[allow(clippy::result_large_err)]
unsafe fn build_base_query(
    data_contract: &DataContract,
    document_type: *const c_char,
    where_json: *const c_char,
) -> Result<DocumentQuery, FFIError> {
    let document_type_str = CStr::from_ptr(document_type)
        .to_str()
        .map_err(FFIError::from)?;

    let mut query = DocumentQuery::new(data_contract.clone(), document_type_str)
        .map_err(|e| FFIError::InternalError(format!("Failed to create query: {}", e)))?;

    if !where_json.is_null() {
        let where_str = CStr::from_ptr(where_json)
            .to_str()
            .map_err(FFIError::from)?;
        if !where_str.is_empty() {
            let clauses: Vec<WhereClauseJson> = serde_json::from_str(where_str)
                .map_err(|e| FFIError::InternalError(format!("Invalid where JSON: {}", e)))?;

            for clause in clauses {
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

    Ok(query)
}

/// Count documents matching a query.
///
/// Returns a JSON string of shape `{"count": <number>}`.
///
/// # Safety
/// - `sdk_handle` and `data_contract_handle` must be valid, non-null pointers.
/// - `document_type` must be a NUL-terminated C string valid for the duration of the call.
/// - `where_json` may be null; if non-null it must be a NUL-terminated JSON string of `[{field, operator, value}]`.
/// - On success, returns a heap-allocated C string pointer; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_count(
    sdk_handle: *const SDKHandle,
    data_contract_handle: *const DataContractHandle,
    document_type: *const c_char,
    where_json: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null() || data_contract_handle.is_null() || document_type.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle, data contract handle, or document type is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let data_contract = &*(data_contract_handle as *const DataContract);

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        let base_query = build_base_query(data_contract, document_type, where_json)?;
        let count_query = DocumentCountQuery {
            document_query: base_query,
        };

        let count = DocumentCount::fetch(&wrapper.sdk, count_query)
            .await
            .map_err(|e| FFIError::InternalError(format!("Failed to fetch count: {}", e)))?
            .map(|c| c.0)
            .unwrap_or(0);

        serde_json::to_string(&DocumentCountResult { count })
            .map_err(|e| FFIError::InternalError(format!("Failed to serialize result: {}", e)))
    });

    match result {
        Ok(json) => match CString::new(json) {
            Ok(s) => DashSDKResult::success_string(s.into_raw()),
            Err(e) => DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create CString: {}", e),
            )),
        },
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Count documents matching a query, split by an index property.
///
/// Returns a JSON string of shape `{"counts": {"<hex-key>": <number>, ...}}`.
/// Hex keys correspond to the platform-value-encoded property values from the
/// underlying split-count tree; iOS callers should hex-decode them and decode
/// against the contract's index-property type if they need a typed key.
///
/// # Safety
/// - `sdk_handle`, `data_contract_handle`, `document_type`, and `split_property` must be valid, non-null pointers.
/// - `document_type` and `split_property` must be NUL-terminated C strings valid for the duration of the call.
/// - `where_json` may be null; if non-null it must be a NUL-terminated JSON string of `[{field, operator, value}]`.
/// - On success, returns a heap-allocated C string pointer; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_split_count(
    sdk_handle: *const SDKHandle,
    data_contract_handle: *const DataContractHandle,
    document_type: *const c_char,
    split_property: *const c_char,
    where_json: *const c_char,
) -> DashSDKResult {
    if sdk_handle.is_null()
        || data_contract_handle.is_null()
        || document_type.is_null()
        || split_property.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle, data contract handle, document type, or split property is null"
                .to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let data_contract = &*(data_contract_handle as *const DataContract);

    let split_property_str = match CStr::from_ptr(split_property).to_str() {
        Ok(s) => s.to_string(),
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        let base_query = build_base_query(data_contract, document_type, where_json)?;
        let split_query = DocumentSplitCountQuery {
            document_query: base_query,
            split_property: split_property_str,
        };

        let split_counts = DocumentSplitCounts::fetch(&wrapper.sdk, split_query)
            .await
            .map_err(|e| FFIError::InternalError(format!("Failed to fetch split counts: {}", e)))?
            .map(|s| s.0)
            .unwrap_or_default();

        let counts: BTreeMap<String, u64> = split_counts
            .into_iter()
            .map(|(k, v)| (hex::encode(k), v))
            .collect();

        serde_json::to_string(&DocumentSplitCountResult { counts })
            .map_err(|e| FFIError::InternalError(format!("Failed to serialize result: {}", e)))
    });

    match result {
        Ok(json) => match CString::new(json) {
            Ok(s) => DashSDKResult::success_string(s.into_raw()),
            Err(e) => DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create CString: {}", e),
            )),
        },
        Err(e) => DashSDKResult::error(e.into()),
    }
}
