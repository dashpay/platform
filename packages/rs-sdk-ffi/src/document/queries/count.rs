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

use dash_sdk::dpp::platform_value::Value;
use dash_sdk::dpp::prelude::DataContract;
use dash_sdk::drive::query::{OrderClause, SelectProjection, WhereClause, WhereOperator};
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
pub(super) unsafe fn parse_group_by_json(
    group_by_json: *const c_char,
) -> Result<Vec<String>, FFIError> {
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
pub(super) unsafe fn build_base_query(
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

/// Decode the C ABI `limit: i64` per the
/// [`dash_sdk_document_count`] contract:
///
/// - `-1` → SDK's `0` "unset" sentinel (maps to `None` on the V1
///   wire, asking the server to apply its default).
/// - `> 0` → explicit cap, returned as `u32`.
/// - `0` → rejected ([`FFIError::InternalError`]). The v1 wire
///   rejects `Some(0)` uniformly across SELECT modes (see proto
///   docs); the FFI surfaces the same rejection at decode time
///   instead of relaying through the SDK's `0`-as-unset
///   internal sentinel, where it would silently mean "use
///   server default" and contradict the `-1 = default` contract.
/// - `< -1` → rejected. Any negative value other than the
///   explicit `-1` sentinel is malformed input; the previous
///   lenient decode mapped `-2`, `-100`, etc. all to "use
///   server default", which masked caller bugs (uninitialized
///   memory, arithmetic underflow). Single-valued per input is
///   the FFI contract.
/// - `> u32::MAX` → rejected (overflow).
///
/// Extracted from the call site so the decode can be unit-
/// tested directly without standing up an SDK / data contract /
/// runtime — see the bottom-of-module tests.
pub(super) fn decode_ffi_limit(limit: i64) -> Result<u32, FFIError> {
    match limit {
        -1 => Ok(0), // SDK-internal "unset" sentinel; maps to `None` on the V1 wire.
        n if n < -1 => Err(FFIError::InternalError(format!(
            "limit {} is invalid; use -1 for server default or a positive \
             integer for an explicit cap",
            n
        ))),
        0 => Err(FFIError::InternalError(
            "limit 0 is invalid; use -1 for server default or a positive \
             integer for an explicit cap (zero-cap query is structurally \
             meaningless and is rejected on the v1 wire as well)"
                .to_string(),
        )),
        n if n > u32::MAX as i64 => Err(FFIError::InternalError(format!(
            "limit {} exceeds u32::MAX",
            n
        ))),
        n => Ok(n as u32),
    }
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
/// - `limit`: sentinel-encoded `int64` on the C ABI.
///   - `-1`: use server default
///     (`default_query_limit` on no-proof paths,
///     `crate::config::DEFAULT_QUERY_LIMIT` on the prove-distinct
///     path — the compile-time constant the SDK verifier reads,
///     so proof bytes stay deterministic across operators).
///   - `> 0`: explicit cap (clamped to `max_query_limit` on
///     no-proof paths, rejected with `InvalidLimit` if too large
///     on the prove-distinct path — silent clamping would
///     invisibly break verification).
///   - `0`: **rejected with `InvalidParameter`** at the FFI
///     boundary. The v1 wire's `optional uint32 limit` rejects
///     `Some(0)` uniformly across SELECT modes (see proto
///     docs); the FFI surfaces that contract at decode time
///     rather than relaying the value through the SDK's
///     `0`-as-unset internal sentinel where it would silently
///     mean "use server default" — that would contradict the
///     `-1 = default` contract documented here.
///   - `< -1`: **rejected with `InvalidParameter`**. Any
///     negative value other than the explicit `-1` sentinel is
///     malformed input; clients shouldn't expect it to be
///     normalized to `-1` because that hides bugs in caller
///     code that miscomputes negative values.
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

        let limit_u32 = decode_ffi_limit(limit)?;

        // `group_by_json` mirrors the wire's `repeated string`
        // field one-to-one. No FFI-side translation: callers ask
        // for exactly the per-group shape they want; the server
        // rejects unsupported `(select, group_by, where)`
        // combinations (see proto docs).
        let group_by = parse_group_by_json(group_by_json)?;
        let count_query = base_query
            .with_select(SelectProjection::count_star())
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
            .map_err(FFIError::from)?
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

#[cfg(test)]
mod tests {
    //! Unit tests for the C ABI `limit: i64` decode contract.
    //!
    //! The decode is extracted into [`decode_ffi_limit`] so these
    //! tests don't need to stand up an SDK / data contract /
    //! runtime to pin the per-input behavior — every test below
    //! exercises a single sentinel category and asserts the exact
    //! mapping the docstring on [`dash_sdk_document_count`]
    //! promises.

    use super::*;

    /// `-1` is the documented "use server default" sentinel.
    /// Maps to the SDK's internal `0` unset sentinel (translated
    /// to `None` on the V1 wire).
    #[test]
    fn decode_ffi_limit_minus_one_is_unset_sentinel() {
        assert_eq!(
            decode_ffi_limit(-1).expect("`-1` must decode to the unset sentinel"),
            0,
            "the FFI's `-1` sentinel must map to the SDK's `0` unset \
             sentinel; any other value would silently change the wire \
             representation"
        );
    }

    /// `0` is invalid at the FFI boundary — the v1 wire rejects
    /// `Some(0)` uniformly across SELECT modes, and the FFI
    /// surfaces that rejection at decode time instead of relaying
    /// through the SDK's `0`-as-unset internal sentinel (where
    /// it would silently mean "use server default" and
    /// contradict the `-1 = default` contract).
    ///
    /// This is the load-bearing test for the new tightening — a
    /// regression that re-collapses `0` into the unset sentinel
    /// (e.g. someone reverts to `if limit < 0 { 0 }`) would mask
    /// caller bugs that pass uninitialized memory.
    #[test]
    fn decode_ffi_limit_zero_is_rejected() {
        let err = decode_ffi_limit(0).expect_err("`0` must be rejected at the FFI boundary");
        let msg = err.to_string();
        assert!(
            msg.contains("limit 0 is invalid"),
            "expected explicit `limit 0 is invalid` rejection; got: {}",
            msg
        );
        assert!(
            msg.contains("-1") && msg.contains("positive"),
            "rejection message must point callers at the valid alternatives \
             (-1 for default, positive for explicit cap); got: {}",
            msg
        );
    }

    /// Any negative value other than `-1` is malformed input.
    /// The previous lenient decode mapped `-2`, `-100`, ... all
    /// to `0` (i.e. "use server default"), which masked caller
    /// bugs from arithmetic underflow or uninitialized memory.
    #[test]
    fn decode_ffi_limit_negative_other_than_minus_one_is_rejected() {
        for bad in [-2i64, -100, i64::MIN] {
            // `.err().unwrap_or_else(|| panic!(...))` rather than
            // `.expect_err(&format!(...))` — the latter trips
            // clippy::expect_fun_call (CI runs `-D warnings`).
            let err = decode_ffi_limit(bad)
                .err()
                .unwrap_or_else(|| panic!("`{}` must be rejected (not -1)", bad));
            let msg = err.to_string();
            assert!(
                msg.contains(&bad.to_string()),
                "rejection message for `{}` must include the offending \
                 value so callers can locate the bug; got: {}",
                bad,
                msg
            );
            assert!(
                msg.contains("-1") && msg.contains("positive"),
                "rejection message for `{}` must direct callers to the \
                 valid alternatives; got: {}",
                bad,
                msg
            );
        }
    }

    /// `> 0` decodes verbatim as `u32`.
    #[test]
    fn decode_ffi_limit_positive_decodes_verbatim() {
        // Edge values + a typical caller-provided cap.
        for n in [1i64, 50, 1000, u32::MAX as i64] {
            // `.unwrap_or_else(|e| panic!(...))` rather than
            // `.expect(&format!(...))` — same clippy::expect_fun_call
            // rationale as the negative test above.
            let decoded = decode_ffi_limit(n)
                .unwrap_or_else(|e| panic!("`{}` must decode to {} but errored: {}", n, n, e));
            assert_eq!(
                decoded, n as u32,
                "positive `{}` must decode unchanged; any normalization \
                 would silently shift the explicit cap callers requested",
                n
            );
        }
    }

    /// Values exceeding `u32::MAX` overflow the wire field and
    /// are rejected. Distinct from the `< -1` rejection so
    /// callers can locate overflow bugs vs. malformed-negative
    /// bugs from the error message.
    #[test]
    fn decode_ffi_limit_over_u32_max_is_rejected() {
        let too_big = u32::MAX as i64 + 1;
        let err = decode_ffi_limit(too_big)
            .expect_err("values > u32::MAX must be rejected to prevent silent truncation");
        let msg = err.to_string();
        assert!(
            msg.contains(&too_big.to_string()) && msg.contains("u32::MAX"),
            "overflow-rejection message must name both the offending value \
             AND the limit so callers can fix their caps; got: {}",
            msg
        );
    }
}
