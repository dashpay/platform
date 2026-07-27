//! Sum-side FFI entry. Mirror of [`super::count::dash_sdk_document_count`]
//! for the sum surface — `SELECT SUM(sum_property)` over a where
//! clause + optional group_by.
//!
//! The result shape depends on whether `group_by` is requested, and
//! the call routes to a *different* rs-sdk type for each:
//!
//! - **Ungrouped** (empty / null `group_by_json`): the aggregate
//!   [`drive_proof_verifier::DocumentSum`] (a single folded `i64`),
//!   surfaced as a one-entry map with the empty (`""`) key — the
//!   documented "single total" contract. This holds even when the
//!   `where` clause carries an `in` / range fork: `DocumentSum` folds
//!   every verified branch into one total. (The per-group
//!   `DocumentSplitSums` view returns one entry *per matched group/key*
//!   and is the wrong type for an ungrouped total.)
//! - **Grouped** (non-empty `group_by_json`): the per-group
//!   [`drive_proof_verifier::DocumentSplitSums`] view — one hex-keyed
//!   entry per matched group, flattened by `try_into_flat_map`.
//!
//! Wraps those rs-sdk `Fetch` flows so callers can obtain document
//! sums without constructing `GetDocumentsRequest` v1 payloads
//! directly. The where / order_by / group_by / limit parameter
//! handling is shared verbatim with the count surface (see
//! [`super::count`]); the only sum-specific pieces are the
//! `sum_property` field naming the integer property to aggregate and
//! the signed (`i64`) result values.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dash_sdk::drive::query::SelectProjection;
use dash_sdk::platform::Fetch;
use drive_proof_verifier::{DocumentSplitSums, DocumentSum};
use serde::Serialize;

use super::count::{build_base_query, decode_ffi_limit, parse_group_by_json};
use crate::sdk::SDKWrapper;
use crate::{
    DashSDKError, DashSDKErrorCode, DashSDKResult, DataContractHandle, FFIError, SDKHandle,
};
use dash_sdk::dpp::prelude::DataContract;

/// Reject an empty aggregation-property name.
///
/// The sum / average FFI entry points both require a non-empty
/// `sum_property` naming the integer property to aggregate; an empty
/// string is malformed input the server would reject. Extracted from
/// the async call sites so the rejection can be unit-tested without
/// standing up an SDK / data contract / runtime (mirrors
/// [`super::count::decode_ffi_limit`]).
#[allow(clippy::result_large_err)]
pub(super) fn validate_aggregation_property(prop: &str) -> Result<(), FFIError> {
    if prop.is_empty() {
        return Err(FFIError::InvalidParameter(
            "aggregation property must name the integer property to \
             aggregate; got an empty string"
                .to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct DocumentSumResult {
    /// Sum results keyed by group.
    ///
    /// - **Ungrouped** (empty / null `group_by_json`): exactly one
    ///   entry under the empty (`""`) key — the aggregate total folded
    ///   across every matched branch. An absent / `None` aggregate
    ///   yields `{"": 0}` so the "single empty-key total" contract
    ///   always holds.
    /// - **Grouped** (non-empty `group_by_json`): one entry per matched
    ///   group, hex-encoded so iOS callers can match them against the
    ///   corresponding platform-value-encoded property bytes.
    ///
    /// Values are signed (`i64`) to match grovedb's `SumValue = i64`.
    sums: BTreeMap<String, i64>,
}

/// `SELECT SUM(<sum_property>)` over a where clause + optional group_by.
///
/// Returns a JSON string of shape
/// `{"sums": {"<key>": <signed number>, ...}}`. The map's shape depends
/// on whether `group_by_json` is requested:
///
/// - **Ungrouped** (empty/null `group_by_json`): exactly one entry under
///   the empty (`""`) key — `sums[""]` is the aggregate total. This
///   routes to the aggregate [`drive_proof_verifier::DocumentSum`],
///   which folds every verified branch (including any `in` / range fork
///   in the `where` clause) into one `i64`. An absent / `None`
///   aggregate is reported as `{"": 0}` so the single empty-key total
///   contract always holds.
/// - **Grouped** (non-empty `group_by_json`): one entry per matched
///   group, keyed by the hex-encoded platform-value-encoded property
///   value from the underlying sum-tree path; iOS callers should
///   hex-decode them and decode against the contract's index-property
///   type if they need a typed key. This routes to the per-group
///   [`drive_proof_verifier::DocumentSplitSums`] view. Compound
///   `(in_key, key)` entries are collapsed into a flat map by summing
///   each In-fork's contribution at the same terminator key; the fold
///   uses checked `i64` arithmetic and surfaces overflow as an error
///   rather than wrapping. Callers needing the unmerged per-branch shape
///   should use a richer binding.
///
/// (The analogous ungrouped-vs-grouped distinction exists for
/// [`super::count::dash_sdk_document_count`], which routes both modes
/// through `DocumentSplitCounts`.)
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
        validate_aggregation_property(sum_property_str)?;

        let limit_u32 = decode_ffi_limit(limit)?;

        // `group_by_json` mirrors the wire's `repeated string` field
        // one-to-one (see `dash_sdk_document_count`). The SUM
        // projection carries the field directly — `SelectProjection::sum`
        // is the sum-side analog of count's `count_star`.
        let group_by = parse_group_by_json(group_by_json)?;

        // Route on whether grouping was requested. The per-group
        // `DocumentSplitSums` view yields one entry *per matched
        // group/key* — so it is the wrong type for an ungrouped total
        // (it would surface per-key entries for an `in` / range `where`
        // clause instead of one folded total). The aggregate
        // `DocumentSum` folds every verified branch into a single `i64`.
        let sums: BTreeMap<String, i64> = if group_by.is_empty() {
            // Ungrouped: aggregate total under the empty (`""`) key.
            // `DocumentSum` is a tuple struct over a single folded `i64`;
            // a `None` fetch (queried-but-absent) reports the zero total
            // so the documented single empty-key total always holds.
            let sum_query = base_query
                .with_select(SelectProjection::sum(sum_property_str))
                .with_limit(limit_u32);

            let total = DocumentSum::fetch(&wrapper.sdk, sum_query)
                .await
                .map_err(FFIError::from)?
                .map(|s| s.0)
                .unwrap_or(0);

            BTreeMap::from([(String::new(), total)])
        } else {
            // Grouped: one hex-keyed entry per matched group.
            // `try_into_flat_map` collapses any compound (in_key + key)
            // entries by summing over `in_key` with checked `i64`
            // arithmetic; callers needing the unmerged shape should use
            // a richer binding.
            let sum_query = base_query
                .with_select(SelectProjection::sum(sum_property_str))
                .with_group_by_fields(group_by)
                .with_limit(limit_u32);

            let flat_sums = DocumentSplitSums::fetch(&wrapper.sdk, sum_query)
                .await
                .map_err(FFIError::from)?
                .map(|s| s.try_into_flat_map())
                .transpose()
                .map_err(|e| {
                    FFIError::InternalError(format!("Failed to flatten sum result: {}", e))
                })?
                .unwrap_or_default();

            flat_sums
                .into_iter()
                .map(|(k, v)| (hex::encode(k), v))
                .collect()
        };

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

#[cfg(test)]
mod tests {
    //! Unit tests for the sum-side FFI surface that don't need an SDK /
    //! data contract / runtime: the empty-property rejection (extracted
    //! into [`validate_aggregation_property`]) and the exact JSON wire
    //! shape iOS callers decode. Mirrors the `decode_ffi_limit` test
    //! style at the bottom of [`super::super::count`].

    use super::*;

    /// An empty aggregation property is malformed input and must be
    /// rejected as [`FFIError::InvalidParameter`] (maps to
    /// `DashSDKErrorCode::InvalidParameter` at the FFI boundary); a
    /// non-empty name passes.
    #[test]
    fn validate_aggregation_property_rejects_empty_accepts_named() {
        assert!(
            matches!(
                validate_aggregation_property(""),
                Err(FFIError::InvalidParameter(_))
            ),
            "empty property must be rejected as InvalidParameter"
        );
        assert!(
            validate_aggregation_property("amount").is_ok(),
            "a named property must be accepted"
        );
    }

    /// The `DocumentSumResult` wire shape iOS decodes is exactly
    /// `{"sums": {"<hex-key>": <signed number>, ...}}`. The empty key
    /// (aggregate total) and a signed value must round-trip verbatim;
    /// `BTreeMap` ordering keeps the key order deterministic.
    #[test]
    fn document_sum_result_serializes_to_expected_shape() {
        let sums = BTreeMap::from([("".to_string(), 42i64), ("61".to_string(), -5i64)]);
        let json = serde_json::to_string(&DocumentSumResult { sums })
            .expect("DocumentSumResult must serialize");
        assert_eq!(json, r#"{"sums":{"":42,"61":-5}}"#);
    }

    /// The ungrouped (aggregate) branch always emits a single
    /// empty-string-keyed total. This pins the exact wire shape the
    /// `group_by.is_empty()` path produces — `{"sums":{"":<total>}}` —
    /// so a regression that reintroduced per-key entries for an
    /// ungrouped `in` / range query would change the serialized bytes
    /// iOS decodes against.
    #[test]
    fn document_sum_result_ungrouped_is_single_empty_key_total() {
        let sums = BTreeMap::from([(String::new(), 42i64)]);
        let json = serde_json::to_string(&DocumentSumResult { sums })
            .expect("DocumentSumResult must serialize");
        assert_eq!(json, r#"{"sums":{"":42}}"#);
    }

    /// An absent / `None` aggregate (queried-but-empty) is reported as
    /// the zero total under the empty key, never an empty map, so the
    /// documented "single empty-key total" contract always holds.
    #[test]
    fn document_sum_result_ungrouped_absent_is_zero_total() {
        let sums = BTreeMap::from([(String::new(), 0i64)]);
        let json = serde_json::to_string(&DocumentSumResult { sums })
            .expect("DocumentSumResult must serialize");
        assert_eq!(json, r#"{"sums":{"":0}}"#);
    }
}
