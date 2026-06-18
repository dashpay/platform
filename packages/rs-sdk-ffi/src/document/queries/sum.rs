//! Sum-side FFI entry. Mirror of [`super::count::dash_sdk_document_count`]
//! for the sum surface — `SELECT SUM(sum_property)` over a where
//! clause + optional group_by.
//!
//! Wraps the rs-sdk [`drive_proof_verifier::DocumentSplitSums::fetch`]
//! flow so callers can obtain document sums without constructing
//! `GetDocumentsRequest` v1 payloads directly. The where / order_by /
//! group_by / limit parameter handling is shared verbatim with the
//! count surface (see [`super::count`]); the only sum-specific pieces
//! are the `sum_property` field naming the integer property to
//! aggregate and the signed (`i64`) result values.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dash_sdk::drive::query::SelectProjection;
use dash_sdk::platform::Fetch;
use drive_proof_verifier::DocumentSplitSums;
use serde::Serialize;

use super::count::{build_base_query, decode_ffi_limit, parse_group_by_json};
use crate::sdk::SDKWrapper;
use crate::{
    DashSDKError, DashSDKErrorCode, DashSDKResult, DataContractHandle, FFIError, SDKHandle,
};
use dash_sdk::dpp::prelude::DataContract;

#[derive(Debug, Serialize)]
struct DocumentSumResult {
    /// Per-key sums. Keys are hex-encoded so iOS callers can match
    /// them against the corresponding platform-value-encoded property
    /// bytes. For aggregate-sum requests (empty / null `group_by_json`)
    /// this is a one-entry map with an empty key. Values are signed
    /// (`i64`) to match grovedb's `SumValue = i64`.
    sums: BTreeMap<String, i64>,
}

/// `SELECT SUM(<sum_property>)` over a where clause + optional group_by.
///
/// Returns a JSON string of shape
/// `{"sums": {"<hex-key>": <signed number>, ...}}`. Hex keys correspond
/// to the platform-value-encoded property values from the underlying
/// sum-tree path; iOS callers should hex-decode them and decode against
/// the contract's index-property type if they need a typed key. For
/// aggregate sums (empty/null `group_by_json`) the result is a one-entry
/// map with an empty key — `sums[""]` is the total.
///
/// Per-key result shapes mirror [`super::count::dash_sdk_document_count`]
/// exactly (aggregate / per-`in_field` / per-`range_field` /
/// compound `(in_field, range_field)`). Compound `(in_key, key)` entries
/// are collapsed into a flat map by summing each In-fork's contribution
/// at the same terminator key; the fold uses checked `i64` arithmetic and
/// surfaces overflow as an error rather than wrapping. Callers needing
/// the unmerged per-branch shape should use a richer binding.
///
/// # Parameters
/// - `sdk_handle`, `data_contract_handle`: valid non-null pointers.
/// - `document_type`: NUL-terminated C string naming the document type.
/// - `sum_property`: NUL-terminated C string naming the integer
///   property to sum. Must match the doctype's `documentsSummable`
///   value or a covering index's `summable: "<field>"`; rejected at
///   the server otherwise.
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
/// Same contract as [`super::count::dash_sdk_document_count`]. All
/// pointers must be valid for the duration of the call.
/// - `sdk_handle` and `data_contract_handle` must be valid, non-null pointers.
/// - `document_type` and `sum_property` must be NUL-terminated C strings valid for the duration of the call.
/// - `where_json`, `order_by_json`, and `group_by_json` may be null; if non-null they must be NUL-terminated JSON strings.
/// - On success, returns a heap-allocated C string pointer; caller must free it using SDK routines.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_document_sum(
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
                "sum_property must name the integer property to sum; got an empty string"
                    .to_string(),
            ));
        }

        let limit_u32 = decode_ffi_limit(limit)?;

        // `group_by_json` mirrors the wire's `repeated string` field
        // one-to-one (see `dash_sdk_document_count`). The SUM
        // projection carries the field directly — `SelectProjection::sum`
        // is the sum-side analog of count's `count_star`.
        let group_by = parse_group_by_json(group_by_json)?;
        let sum_query = base_query
            .with_select(SelectProjection::sum(sum_property_str))
            .with_group_by_fields(group_by)
            .with_limit(limit_u32);

        // `DocumentSplitSums::fetch` handles every sum mode — for
        // aggregate-sum requests the result is a one-entry map with an
        // empty key (so `result.sums[""]` is the total).
        // `try_into_flat_map` collapses any compound (in_key + key)
        // entries by summing over `in_key` with checked `i64`
        // arithmetic; callers needing the unmerged shape should use a
        // richer binding.
        let flat_sums = DocumentSplitSums::fetch(&wrapper.sdk, sum_query)
            .await
            .map_err(|e| FFIError::InternalError(format!("Failed to fetch sum: {}", e)))?
            .map(|s| s.try_into_flat_map())
            .transpose()
            .map_err(|e| FFIError::InternalError(format!("Failed to flatten sum result: {}", e)))?
            .unwrap_or_default();

        let sums: BTreeMap<String, i64> = flat_sums
            .into_iter()
            .map(|(k, v)| (hex::encode(k), v))
            .collect();

        serde_json::to_string(&DocumentSumResult { sums })
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
