//! Unified document-count FFI for iOS / native callers.
//!
//! Wraps the rs-sdk `DocumentSplitCounts::fetch` flow (which handles
//! every count mode — total, per-`In`-value, per-distinct-value-in-
//! range, summed-over-range) so callers can obtain document counts
//! without having to construct `GetDocumentsCountRequest` payloads
//! themselves.
//!
//! The previous version exposed two functions (`dash_sdk_document_count`
//! returning a single u64, `dash_sdk_document_split_count` returning a
//! per-key map). Now that the count endpoint carries
//! `return_distinct_counts_in_range`, `order_by_ascending`, and
//! `limit`, the split path subsumes the simple-total case (total count
//! becomes a one-entry map with empty key), so we expose one entry
//! point with all the knobs.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::DataContract;
use dash_sdk::drive::query::{WhereClause, WhereOperator};
use dash_sdk::platform::documents::document_count_query::DocumentCountQuery;
use dash_sdk::platform::documents::document_query::DocumentQuery;
use dash_sdk::platform::Fetch;
use drive_proof_verifier::DocumentSplitCounts;
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
    /// Per-key counts. Keys are hex-encoded so iOS callers can match
    /// them against the corresponding platform-value-encoded property
    /// bytes. For total-count requests (no `in` clause and
    /// `return_distinct_counts_in_range = false`) this is a one-entry
    /// map with an empty key.
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
/// Returns a JSON string of shape `{"counts": {"<hex-key>": <number>, ...}}`.
/// Hex keys correspond to the platform-value-encoded property values
/// from the underlying CountTree / ProvableCountTree path; iOS callers
/// should hex-decode them and decode against the contract's index-
/// property type if they need a typed key.
///
/// For simple total counts (no `in` clause in `where_json` and
/// `return_distinct_counts_in_range = false`) the result is a one-entry
/// map with an empty key — `counts[""]` is the total.
///
/// Per-key result shapes:
/// - **`in` clause**: one entry per (deduped) value in the In array.
/// - **range clause + `return_distinct_counts_in_range = true`**: one
///   entry per distinct property value within the range. For compound
///   queries (`in` on a prefix property + range on the terminator), the
///   per-`in_key`/per-`key` entries are summed by `key` into a flat
///   map. Callers needing the unmerged compound shape should use a
///   richer binding (not yet exposed via this entry point).
///
/// # Tunables
/// - `return_distinct_counts_in_range`: when `true` AND the query has
///   a range clause, returns per-distinct-value entries instead of a
///   single sum. No-op when there's no range clause.
/// - `order_by_ascending`: `-1` = use server default (ascending),
///   `0` = descending, `1` = ascending. Affects per-`in`-value and
///   per-distinct-value-in-range entry order on the server.
/// - `limit`: `-1` = use server default (`default_query_limit`),
///   `≥ 0` = explicit cap (clamped to `max_query_limit` server-side
///   on no-proof paths, rejected if too large on prove paths).
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
    return_distinct_counts_in_range: bool,
    order_by_ascending: i32,
    limit: i64,
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

        // Sentinel decoding for the C ABI. `-1` means "unset; use
        // server-side default". The Rust-side request fields are
        // `Option<...>` so `None` here is the same as the request
        // omitting the field on the wire.
        let order_by_ascending_opt = match order_by_ascending {
            -1 => None,
            0 => Some(false),
            1 => Some(true),
            other => {
                return Err(FFIError::InternalError(format!(
                    "order_by_ascending must be -1 (default), 0 (descending), or 1 (ascending); got {other}"
                )));
            }
        };
        let limit_opt = if limit < 0 {
            None
        } else if limit > u32::MAX as i64 {
            return Err(FFIError::InternalError(format!(
                "limit {} exceeds u32::MAX",
                limit
            )));
        } else {
            Some(limit as u32)
        };

        let count_query = DocumentCountQuery {
            document_query: base_query,
            return_distinct_counts_in_range,
            order_by_ascending: order_by_ascending_opt,
            limit: limit_opt,
        };

        // `DocumentSplitCounts::fetch` handles every count mode —
        // for total-count requests the result is a one-entry map
        // with empty key (so `result.counts[""]` is the total).
        // `into_flat_map` collapses any compound (in_key + key)
        // entries by summing over `in_key`; callers needing the
        // unmerged shape should use a richer binding.
        let split_counts = DocumentSplitCounts::fetch(&wrapper.sdk, count_query)
            .await
            .map_err(|e| FFIError::InternalError(format!("Failed to fetch count: {}", e)))?
            .map(|s| s.into_flat_map())
            .unwrap_or_default();

        let counts: BTreeMap<String, u64> = split_counts
            .into_iter()
            .map(|(k, v)| (hex::encode(k), v))
            .collect();

        serde_json::to_string(&DocumentCountResult { counts })
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
