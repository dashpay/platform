//! Average-side FFI entry. Mirror of [`super::sum::dash_sdk_document_sum`]
//! for the average surface — `SELECT AVG(sum_property)` over a where
//! clause + optional group_by.
//!
//! Averages reuse sum-tree indexes; the underlying server-side primitive
//! returns the `(count, sum)` pair and the client divides. This FFI
//! exposes the raw pair (not a pre-divided average) so iOS/Swift
//! callers can pick their own precision representation.
//!
//! Wraps the rs-sdk
//! [`drive_proof_verifier::DocumentSplitAverages::fetch`] flow. The
//! where / order_by / group_by / limit parameter handling is shared
//! verbatim with the count and sum surfaces (see [`super::count`]); the
//! only average-specific piece is the `(count, sum)` pair carried per
//! key in place of a single scalar.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dash_sdk::drive::query::SelectProjection;
use dash_sdk::platform::Fetch;
use drive_proof_verifier::DocumentSplitAverages;
use serde::Serialize;

use super::count::{build_base_query, decode_ffi_limit, parse_group_by_json};
use crate::sdk::SDKWrapper;
use crate::{
    DashSDKError, DashSDKErrorCode, DashSDKResult, DataContractHandle, FFIError, SDKHandle,
};
use dash_sdk::dpp::prelude::DataContract;

/// The verified `(count, sum)` pair for a single key. Callers divide
/// `sum / count` locally to obtain the average at their chosen
/// precision. `count` is unsigned (`u64`); `sum` is signed (`i64`) to
/// match grovedb's `SumValue = i64`.
#[derive(Debug, Serialize)]
struct AverageEntryJson {
    count: u64,
    sum: i64,
}

#[derive(Debug, Serialize)]
struct DocumentAverageResult {
    /// Per-key `(count, sum)` pairs. Keys are hex-encoded so iOS
    /// callers can match them against the corresponding
    /// platform-value-encoded property bytes. For aggregate-average
    /// requests (empty / null `group_by_json`) this is a one-entry map
    /// with an empty key.
    averages: BTreeMap<String, AverageEntryJson>,
}

/// `SELECT AVG(<sum_property>)` over a where clause + optional group_by.
///
/// Returns a JSON string of shape
/// `{"averages": {"<hex-key>": {"count": <u64>, "sum": <i64>}, ...}}`.
/// The raw `(count, sum)` pair is returned rather than a pre-divided
/// average so callers can pick their own precision representation
/// (`sum / count`). Hex keys correspond to the platform-value-encoded
/// property values from the underlying count+sum tree path. For
/// aggregate averages (empty/null `group_by_json`) the result is a
/// one-entry map with an empty key — `averages[""]` is the total
/// `(count, sum)`.
///
/// Per-key result shapes mirror [`super::count::dash_sdk_document_count`]
/// exactly. Compound `(in_key, key)` entries are collapsed into a flat
/// map by summing each In-fork's `count` and `sum` at the same
/// terminator key; both axes use checked arithmetic (`u64` for count,
/// `i64` for sum) and surface overflow as an error rather than wrapping.
/// Callers needing the unmerged per-branch shape should use a richer
/// binding.
///
/// # Parameters
/// - `sdk_handle`, `data_contract_handle`: valid non-null pointers.
/// - `document_type`: NUL-terminated C string naming the document type.
/// - `sum_property`: NUL-terminated C string naming the integer
///   property to average. Same field rules as `dash_sdk_document_sum`
///   — averages reuse sum-tree indexes (the doctype's
///   `documentsSummable` value or a covering index's `summable:
///   "<field>"`); rejected at the server otherwise.
/// - `where_json`: NUL-terminated JSON `[{field, operator, value}]` or
///   null.
/// - `order_by_json`: NUL-terminated JSON `[{field, direction}]` or
///   null.
/// - `group_by_json`: NUL-terminated JSON `["<field>", ...]` or null.
/// - `limit`: sentinel-encoded `int64` — `-1` for server default,
///   `> 0` for an explicit cap, `0` / `< -1` rejected. See
///   [`super::count::dash_sdk_document_count`] for the full contract.
///
/// # Safety
/// Same contract as [`super::sum::dash_sdk_document_sum`]. All
/// pointers must be valid for the duration of the call.
/// - `sdk_handle` and `data_contract_handle` must be valid, non-null pointers.
/// - `document_type` and `sum_property` must be NUL-terminated C strings valid for the duration of the call.
/// - `where_json`, `order_by_json`, and `group_by_json` may be null; if non-null they must be NUL-terminated JSON strings.
/// - On success, returns a heap-allocated C string pointer; caller must free it using SDK routines.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_average(
    sdk_handle: *const SDKHandle,
    data_contract_handle: *const DataContractHandle,
    document_type: *const c_char,
    sum_property: *const c_char,
    where_json: *const c_char,
    order_by_json: *const c_char,
    group_by_json: *const c_char,
    limit: i64,
) -> DashSDKResult {
    if sdk_handle.is_null()
        || data_contract_handle.is_null()
        || document_type.is_null()
        || sum_property.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle, data contract handle, document type, or sum property is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let data_contract = &*(data_contract_handle as *const DataContract);

    let result: Result<String, FFIError> = wrapper.runtime.block_on(async {
        let base_query = build_base_query(data_contract, document_type, where_json, order_by_json)?;

        let sum_property_str = CStr::from_ptr(sum_property)
            .to_str()
            .map_err(FFIError::from)?;
        if sum_property_str.is_empty() {
            return Err(FFIError::InvalidParameter(
                "sum_property must name the integer property to average; got an empty string"
                    .to_string(),
            ));
        }

        let limit_u32 = decode_ffi_limit(limit)?;

        // `group_by_json` mirrors the wire's `repeated string` field
        // one-to-one (see `dash_sdk_document_count`). The AVG
        // projection carries the field directly — `SelectProjection::avg`
        // is the average-side analog of count's `count_star`.
        let group_by = parse_group_by_json(group_by_json)?;
        let average_query = base_query
            .with_select(SelectProjection::avg(sum_property_str))
            .with_group_by_fields(group_by)
            .with_limit(limit_u32);

        // `DocumentSplitAverages::fetch` handles every average mode —
        // for aggregate-average requests the result is a one-entry map
        // with an empty key (so `result.averages[""]` is the total).
        // `try_into_flat_map` collapses any compound (in_key + key)
        // entries by summing each In-fork's count + sum at the same
        // terminator key with checked arithmetic; callers needing the
        // unmerged shape should use a richer binding.
        let flat_averages = DocumentSplitAverages::fetch(&wrapper.sdk, average_query)
            .await
            .map_err(|e| FFIError::InternalError(format!("Failed to fetch average: {}", e)))?
            .map(|s| s.try_into_flat_map())
            .transpose()
            .map_err(|e| {
                FFIError::InternalError(format!("Failed to flatten average result: {}", e))
            })?
            .unwrap_or_default();

        let averages: BTreeMap<String, AverageEntryJson> = flat_averages
            .into_iter()
            .map(|(k, (count, sum))| (hex::encode(k), AverageEntryJson { count, sum }))
            .collect();

        serde_json::to_string(&DocumentAverageResult { averages })
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
