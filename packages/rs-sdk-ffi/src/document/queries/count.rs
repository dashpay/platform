//! Unified document-count FFI for iOS / native callers.
//!
//! Wraps the rs-sdk `DocumentSplitCounts::fetch` flow (which
//! handles every count mode — total, per-group entries, summed
//! aggregate) so callers can obtain document counts without
//! constructing `GetDocumentsRequest` v1 payloads directly.
//!
//! Surface mirrors the v1 wire shape one-to-one: callers pass
//! `where_json`, optional `order_by_json`, optional
//! `group_by_json` (`[]` → aggregate, `["<field>"]` → per-group
//! entries, `["<in_field>", "<range_field>"]` → compound
//! distinct), and `limit`. The split path subsumes the simple-
//! total case (`group_by_json = null` returns a one-entry map
//! with an empty key), so one entry point covers every count
//! mode the server supports.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dash_sdk::dapi_grpc::platform::v0::get_documents_request::get_documents_request_v1::Select;
use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::DataContract;
use dash_sdk::drive::query::{OrderClause, WhereClause, WhereOperator};
use dash_sdk::platform::documents::document_query::DocumentQuery;
use dash_sdk::platform::Fetch;
use drive_proof_verifier::DocumentSplitCounts;
use serde::{Deserialize, Serialize};

use crate::sdk::SDKWrapper;
use crate::types::{DataContractHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

#[derive(Debug, Deserialize)]
struct WhereClauseJson {
    field: String,
    operator: String,
    value: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OrderClauseJson {
    field: String,
    /// `"asc"` (default) or `"desc"`. Direction strings match the
    /// regular document-fetch FFI surface so callers can reuse their
    /// JSON shapes between count and fetch.
    direction: String,
}

#[derive(Debug, Serialize)]
struct DocumentCountResult {
    /// Per-key counts. Keys are hex-encoded so iOS callers can match
    /// them against the corresponding platform-value-encoded property
    /// bytes. For total-count requests (empty / null `group_by_json`)
    /// this is a one-entry map with an empty key.
    counts: BTreeMap<String, u64>,
}

/// Map the wire/JSON operator token to a `WhereOperator`.
///
/// Accepts the full range-operator surface drive's
/// `range_clause_to_query_item` supports (`between`,
/// `betweenExcludeBounds`, `betweenExcludeLeft`,
/// `betweenExcludeRight` — value must be a 2-element array
/// `[lower, upper]`), so iOS/Swift callers can issue every range
/// shape the count endpoint's prove and no-proof paths verify
/// against. Operator names match the wasm bindings'
/// `parse_where_operator` for cross-language parity. Camel-case is
/// the canonical wire form, with kebab-case (`between-exclude-*`)
/// and lower-snake-case (`between_exclude_*`) aliases accepted as
/// a convenience for callers that already normalize to those styles.
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
        // Range bounds: value is `[lower, upper]`. Drive's
        // `range_clause_to_query_item` validates the 2-element
        // array + ordered bounds.
        "between" => Ok(WhereOperator::Between),
        "betweenExcludeBounds" | "between-exclude-bounds" | "between_exclude_bounds" => {
            Ok(WhereOperator::BetweenExcludeBounds)
        }
        "betweenExcludeLeft" | "between-exclude-left" | "between_exclude_left" => {
            Ok(WhereOperator::BetweenExcludeLeft)
        }
        "betweenExcludeRight" | "between-exclude-right" | "between_exclude_right" => {
            Ok(WhereOperator::BetweenExcludeRight)
        }
        _ => Err(FFIError::InternalError(format!(
            "Unknown where operator: {}",
            op
        ))),
    }
}

#[allow(clippy::result_large_err)]
fn parse_order_direction(direction: &str) -> Result<bool, FFIError> {
    match direction {
        "asc" | "ascending" => Ok(true),
        "desc" | "descending" => Ok(false),
        _ => Err(FFIError::InternalError(format!(
            "Unknown order_by direction: {} (use \"asc\" or \"desc\")",
            direction
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

/// Parse the optional `group_by_json` C string parameter into a
/// `Vec<String>`. `null` and empty string are accepted as
/// equivalent to "no grouping" (aggregate count). Valid input
/// is a JSON array of field-name strings, e.g.:
///
/// - `null` or `""` → `[]` (aggregate)
/// - `"[\"color\"]"` → `["color"]` (per-distinct-`color` entries)
/// - `"[\"category\",\"color\"]"` → `["category", "color"]`
///   (compound distinct entries; only valid for
///   `(in_field, range_field)` shapes — other multi-field
///   group_by values return `QuerySyntaxError::Unsupported`)
///
/// Mirrors the wire-level `group_by: repeated string` field on
/// `GetDocumentsRequestV1` directly — no implicit translation,
/// no transform, no SDK-internal helper between FFI and wire.
#[allow(clippy::result_large_err)]
unsafe fn parse_group_by_json(group_by_json: *const c_char) -> Result<Vec<String>, FFIError> {
    if group_by_json.is_null() {
        return Ok(Vec::new());
    }
    let s = CStr::from_ptr(group_by_json)
        .to_str()
        .map_err(FFIError::from)?;
    if s.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(s)
        .map_err(|e| FFIError::InternalError(format!("Invalid group_by JSON: {}", e)))
}

#[allow(clippy::result_large_err)]
unsafe fn build_base_query(
    data_contract: &DataContract,
    document_type: *const c_char,
    where_json: *const c_char,
    order_by_json: *const c_char,
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

    if !order_by_json.is_null() {
        let order_str = CStr::from_ptr(order_by_json)
            .to_str()
            .map_err(FFIError::from)?;
        if !order_str.is_empty() {
            let clauses: Vec<OrderClauseJson> = serde_json::from_str(order_str)
                .map_err(|e| FFIError::InternalError(format!("Invalid order_by JSON: {}", e)))?;
            for clause in clauses {
                let ascending = parse_order_direction(&clause.direction)?;
                query = query.with_order_by(OrderClause {
                    field: clause.field,
                    ascending,
                });
            }
        }
    }

    Ok(query)
}

/// Count documents matching a query.
///
/// Returns a JSON string of shape
/// `{"counts": {"<hex-key>": <number>, ...}}`. Hex keys
/// correspond to the platform-value-encoded property values from
/// the underlying CountTree / ProvableCountTree path; iOS callers
/// should hex-decode them and decode against the contract's
/// index-property type if they need a typed key.
///
/// For simple total counts (empty/null `group_by_json`) the
/// result is a one-entry map with an empty key — `counts[""]`
/// is the total.
///
/// Per-key result shapes:
/// - **`group_by_json = ["<in_field>"]`** (where `<in_field>`
///   is constrained by an `in` clause): one entry per (deduped)
///   value in the In array.
/// - **`group_by_json = ["<range_field>"]`** (where
///   `<range_field>` is constrained by a range clause): one
///   entry per distinct property value within the range.
/// - **`group_by_json = ["<in_field>", "<range_field>"]`** for
///   compound queries (`in` on a prefix property + range on the
///   terminator): per-`(in_key, key)` entries are summed by `key`
///   into a flat map. Callers needing the unmerged compound
///   shape should use a richer binding (not yet exposed via this
///   entry point).
///
/// # Tunables
/// - `group_by_json`: optional JSON array of field names mirroring
///   the wire `group_by` field directly. Null/empty → aggregate
///   count. See per-key shape rules above and the proto docs for
///   the supported `(select, group_by, where)` combinations; any
///   combination outside that set returns
///   `QuerySyntaxError::Unsupported`.
/// - `order_by_json`: optional JSON `[{"field": "<name>", "direction":
///   "asc"|"desc"}]`. The first clause's direction controls
///   split-mode entry ordering server-side; on the
///   `RangeDistinctProof` prove path it is part of the path-query
///   bytes the SDK reconstructs to verify the proof (prover and
///   verifier must agree — empty `order_by` defaults to ascending
///   on both sides). On the `PointLookupProof` path
///   (`(In, prove, no-range)`) order_by is not consulted: the
///   path-query builder sorts In keys lex-ascending
///   unconditionally for prove/no-proof parity. Null or empty →
///   no orderBy (ascending default for split-mode entry
///   direction).
/// - `limit`: `-1` = use server default
///   (`default_query_limit` on no-proof paths,
///   `crate::config::DEFAULT_QUERY_LIMIT` on the prove-distinct
///   path — the compile-time constant the SDK verifier reads,
///   so proof bytes stay deterministic across operators). `≥ 0`
///   = explicit cap (clamped to `max_query_limit` on no-proof
///   paths, rejected with `InvalidLimit` if too large on the
///   prove-distinct path — silent clamping would invisibly break
///   verification).
///
/// # Safety
/// - `sdk_handle` and `data_contract_handle` must be valid, non-null pointers.
/// - `document_type` must be a NUL-terminated C string valid for the duration of the call.
/// - `where_json` may be null; if non-null it must be a NUL-terminated JSON string of `[{field, operator, value}]`.
/// - `order_by_json` may be null; if non-null it must be a NUL-terminated JSON string of `[{field, direction}]`.
/// - `group_by_json` may be null; if non-null it must be a NUL-terminated JSON string of `["<field>", ...]`.
/// - On success, returns a heap-allocated C string pointer; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_count(
    sdk_handle: *const SDKHandle,
    data_contract_handle: *const DataContractHandle,
    document_type: *const c_char,
    where_json: *const c_char,
    order_by_json: *const c_char,
    group_by_json: *const c_char,
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
        let base_query = build_base_query(data_contract, document_type, where_json, order_by_json)?;

        // Sentinel decoding for the C ABI. `-1` means "unset; use
        // server-side default". The DocumentQuery `limit` field is
        // a `u32` with `0` as its "unset" sentinel (translated to
        // `None` on the V1 wire's `optional uint32`), so the FFI
        // `-1` maps to `0`.
        let limit_u32: u32 = if limit < 0 {
            0
        } else if limit > u32::MAX as i64 {
            return Err(FFIError::InternalError(format!(
                "limit {} exceeds u32::MAX",
                limit
            )));
        } else {
            limit as u32
        };

        // `group_by_json` mirrors the wire's `repeated string`
        // field one-to-one. No FFI-side translation: callers ask
        // for exactly the per-group shape they want; the server
        // rejects unsupported `(select, group_by, where)`
        // combinations (see proto docs).
        let group_by = parse_group_by_json(group_by_json)?;
        let count_query = base_query
            .with_select(Select::Count)
            .with_group_by_fields(group_by)
            .with_limit(limit_u32);

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
