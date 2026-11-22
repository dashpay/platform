//! Average-side FFI entry. Mirror of [`super::sum::dash_sdk_document_sum`]
//! for the average surface — `SELECT AVG(sum_property)` over a where
//! clause + optional group_by.
//!
//! Averages reuse sum-tree indexes; the underlying server-side primitive
//! returns the verified `(count, sum)` pair. This FFI returns that pair
//! together with the average, which Rust computes exactly with
//! [`drive_proof_verifier::average_to_decimal_string`] to
//! `AVERAGE_FRACTION_DIGITS` places, so no caller divides. The sum and
//! the average are base-10 JSON strings: from protocol version 15 a sum
//! can pass `i64`, and a JSON number that large is rounded by most JSON
//! readers.
//!
//! As with the sum surface, the result shape — and the rs-sdk type the
//! call routes to — depends on whether `group_by` is requested:
//!
//! - **Ungrouped** (empty / null `group_by_json`): the aggregate
//!   [`drive_proof_verifier::DocumentAverage`] (a single folded
//!   `(count, sum)`), surfaced as a one-entry map with the empty (`""`)
//!   key — the documented "single total" contract. `DocumentAverage`
//!   folds every verified branch (including an `in` / range fork in the
//!   `where` clause) into one pair. (The per-group
//!   `DocumentSplitAverages` view returns one entry *per matched
//!   group/key* and is the wrong type for an ungrouped total.)
//! - **Grouped** (non-empty `group_by_json`): the per-group
//!   [`drive_proof_verifier::DocumentSplitAverages`] view — one
//!   hex-keyed entry per matched group, flattened by `try_into_flat_map`.
//!
//! Wraps those rs-sdk `Fetch` flows. The where / order_by / group_by /
//! limit parameter handling is shared verbatim with the count and sum
//! surfaces (see [`super::count`]); the only average-specific piece is
//! the `(count, sum, average)` entry carried per key in place of a
//! single scalar.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dash_sdk::drive::query::SelectProjection;
use dash_sdk::platform::Fetch;
use drive_proof_verifier::{average_to_decimal_string, DocumentAverage, DocumentSplitAverages};
use serde::Serialize;

use super::count::{build_base_query, decode_ffi_limit, parse_group_by_json};
use super::sum::{serialize_i128_as_decimal, validate_aggregation_property};
use crate::sdk::SDKWrapper;
use crate::{
    DashSDKError, DashSDKErrorCode, DashSDKResult, DataContractHandle, FFIError, SDKHandle,
};
use dash_sdk::dpp::prelude::DataContract;

/// Digits after the decimal point in the `average` string this FFI
/// returns. Four places is what the iOS and Android example apps showed
/// when they divided locally (`%.4f`). Private so cbindgen does not
/// export it into the C header.
const AVERAGE_FRACTION_DIGITS: u8 = 4;

/// The verified `(count, sum)` pair for a single key, plus their average.
/// `count` is a JSON number (`u64`); `sum` is a signed `i128` written as
/// a base-10 string; `average` is `sum / count` as a decimal string with
/// `AVERAGE_FRACTION_DIGITS` places (rounded half away from zero), or
/// `null` when `count` is 0.
#[derive(Debug, Serialize)]
struct AverageEntryJson {
    count: u64,
    #[serde(serialize_with = "serialize_i128_as_decimal")]
    sum: i128,
    average: Option<String>,
}

impl AverageEntryJson {
    /// Build the entry for a verified `(count, sum)` pair, with the
    /// average computed by [`average_to_decimal_string`].
    fn new(count: u64, sum: i128) -> Self {
        Self {
            count,
            sum,
            average: average_to_decimal_string(count, sum, AVERAGE_FRACTION_DIGITS),
        }
    }
}

#[derive(Debug, Serialize)]
struct DocumentAverageResult {
    /// Average `(count, sum, average)` results keyed by group.
    ///
    /// - **Ungrouped** (empty / null `group_by_json`): exactly one
    ///   entry under the empty (`""`) key — the aggregate
    ///   `(count, sum)` folded across every matched branch. An absent /
    ///   `None` aggregate yields
    ///   `{"": {"count": 0, "sum": "0", "average": null}}` so the
    ///   "single empty-key total" contract always holds.
    /// - **Grouped** (non-empty `group_by_json`): one entry per matched
    ///   group, hex-encoded so iOS callers can match them against the
    ///   corresponding platform-value-encoded property bytes.
    averages: BTreeMap<String, AverageEntryJson>,
}

/// `SELECT AVG(<sum_property>)` over a where clause + optional group_by.
///
/// Returns a JSON string of shape
/// `{"averages": {"<key>": {"count": <u64>, "sum": "<signed decimal>",
/// "average": "<decimal>" | null}, ...}}`, for example
/// `{"averages": {"": {"count": 2, "sum": "18446744073709551614",
/// "average": "9223372036854775807.0000"}}}`.
///
/// - `count` is a JSON number.
/// - `sum` is the `i128` sum written as a base-10 string, so a sum past
///   `i64` (possible from protocol version 15) reaches the caller whole.
/// - `average` is `sum / count` computed in Rust on integers by
///   [`drive_proof_verifier::average_to_decimal_string`], exact to
///   `AVERAGE_FRACTION_DIGITS` places and rounded half away from zero,
///   or `null` when `count` is 0. Callers display it as is and do not
///   divide.
///
/// The map's shape depends on whether `group_by_json` is requested:
///
/// - **Ungrouped** (empty/null `group_by_json`): exactly one entry under
///   the empty (`""`) key — `averages[""]` is the aggregate total
///   `(count, sum)`. This routes to the aggregate
///   [`drive_proof_verifier::DocumentAverage`], which folds every
///   verified branch (including any `in` / range fork in the `where`
///   clause) into one `(count, sum)`. An absent / `None` aggregate is
///   reported as `{"count": 0, "sum": "0", "average": null}` so the
///   single empty-key total contract always holds.
/// - **Grouped** (non-empty `group_by_json`): one entry per matched
///   group, keyed by the hex-encoded platform-value-encoded property
///   value from the underlying count+sum tree path. This routes to the
///   per-group [`drive_proof_verifier::DocumentSplitAverages`] view.
///   Compound `(in_key, key)` entries are collapsed into a flat map by
///   summing each In-fork's `count` and `sum` at the same terminator
///   key; both axes use checked arithmetic (`u64` for count, `i128` for
///   sum) and surface overflow as an error rather than wrapping. Callers
///   needing the unmerged per-branch shape should use a richer binding.
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
        validate_aggregation_property(sum_property_str)?;

        let limit_u32 = decode_ffi_limit(limit)?;

        // `group_by_json` mirrors the wire's `repeated string` field
        // one-to-one (see `dash_sdk_document_count`). The AVG
        // projection carries the field directly — `SelectProjection::avg`
        // is the average-side analog of count's `count_star`.
        let group_by = parse_group_by_json(group_by_json)?;

        // Route on whether grouping was requested. The per-group
        // `DocumentSplitAverages` view yields one entry *per matched
        // group/key* — the wrong type for an ungrouped total (it would
        // surface per-key entries for an `in` / range `where` clause
        // instead of one folded `(count, sum)`). The aggregate
        // `DocumentAverage` folds every verified branch into a single
        // `(count, sum)`.
        let averages: BTreeMap<String, AverageEntryJson> = if group_by.is_empty() {
            // Ungrouped: aggregate `(count, sum)` under the empty (`""`)
            // key. `DocumentAverage` carries named `count: u64` / `sum:
            // i128` fields; a `None` fetch (queried-but-absent) reports
            // the zero pair so the documented single empty-key total
            // always holds.
            let average_query = base_query
                .with_select(SelectProjection::avg(sum_property_str))
                .with_limit(limit_u32);

            let DocumentAverage { count, sum } =
                DocumentAverage::fetch(&wrapper.sdk, average_query)
                    .await
                    .map_err(FFIError::from)?
                    .unwrap_or(DocumentAverage { count: 0, sum: 0 });

            BTreeMap::from([(String::new(), AverageEntryJson::new(count, sum))])
        } else {
            // Grouped: one hex-keyed entry per matched group.
            // `try_into_flat_map` collapses any compound (in_key + key)
            // entries by summing each In-fork's count + sum at the same
            // terminator key with checked arithmetic; callers needing
            // the unmerged shape should use a richer binding.
            let average_query = base_query
                .with_select(SelectProjection::avg(sum_property_str))
                .with_group_by_fields(group_by)
                .with_limit(limit_u32);

            let flat_averages = DocumentSplitAverages::fetch(&wrapper.sdk, average_query)
                .await
                .map_err(FFIError::from)?
                .map(|s| s.try_into_flat_map())
                .transpose()
                .map_err(|e| {
                    FFIError::InternalError(format!("Failed to flatten average result: {}", e))
                })?
                .unwrap_or_default();

            flat_averages
                .into_iter()
                .map(|(k, (count, sum))| (hex::encode(k), AverageEntryJson::new(count, sum)))
                .collect()
        };

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

#[cfg(test)]
mod tests {
    //! Unit tests for the average-side FFI wire shape. The empty-property
    //! rejection is shared with the sum surface via
    //! [`super::super::sum::validate_aggregation_property`] and tested
    //! there; here we pin the `(count, sum, average)` JSON shape iOS
    //! decodes. Every entry is built with [`AverageEntryJson::new`], the
    //! same constructor the FFI call uses, so the average string is the
    //! real computed one.

    use super::*;

    /// Serialize averages through the real [`DocumentAverageResult`] type.
    fn averages_json(entries: Vec<(&str, u64, i128)>) -> String {
        let averages = entries
            .into_iter()
            .map(|(key, count, sum)| (key.to_string(), AverageEntryJson::new(count, sum)))
            .collect();
        serde_json::to_string(&DocumentAverageResult { averages })
            .expect("DocumentAverageResult must serialize")
    }

    /// The `DocumentAverageResult` wire shape iOS decodes is exactly
    /// `{"averages": {"<hex-key>": {"count": <u64>, "sum": "<decimal>",
    /// "average": "<decimal>"}, ...}}`. Field order (`count`, `sum`,
    /// `average`) must stay stable so the contract callers decode against
    /// doesn't drift.
    #[test]
    fn document_average_result_serializes_to_expected_shape() {
        assert_eq!(
            averages_json(vec![("61", 3, 30)]),
            r#"{"averages":{"61":{"count":3,"sum":"30","average":"10.0000"}}}"#
        );
    }

    /// The ungrouped (aggregate) branch always emits a single
    /// empty-string-keyed entry. This pins the exact wire shape the
    /// `group_by.is_empty()` path produces so a regression that
    /// reintroduced per-key entries for an ungrouped `in` / range query
    /// would change the serialized bytes iOS decodes against.
    #[test]
    fn document_average_result_ungrouped_is_single_empty_key_total() {
        assert_eq!(
            averages_json(vec![("", 3, 30)]),
            r#"{"averages":{"":{"count":3,"sum":"30","average":"10.0000"}}}"#
        );
    }

    /// An absent / `None` aggregate (queried-but-empty) is reported as
    /// the zero pair with a `null` average under the empty key, never an
    /// empty map, so the documented "single empty-key total" contract
    /// always holds.
    #[test]
    fn document_average_result_ungrouped_absent_is_zero_pair() {
        assert_eq!(
            averages_json(vec![("", 0, 0)]),
            r#"{"averages":{"":{"count":0,"sum":"0","average":null}}}"#
        );
    }

    /// A sum past `i64` (a 128-bit sum tree, protocol version 15) is
    /// written whole, and its average is exact to the last place rather
    /// than rounded through an `f64`.
    #[test]
    fn document_average_result_carries_a_sum_past_i64_whole() {
        let wide = 3 * i128::from(i64::MAX);
        assert_eq!(
            averages_json(vec![("", 3, wide)]),
            r#"{"averages":{"":{"count":3,"sum":"27670116110564327421","average":"9223372036854775807.0000"}}}"#
        );
    }

    /// A negative sum keeps its sign in both `sum` and `average`, and the
    /// average rounds half away from zero at the fourth place.
    #[test]
    fn document_average_result_carries_a_negative_sum() {
        assert_eq!(
            averages_json(vec![("61", 3, -5)]),
            r#"{"averages":{"61":{"count":3,"sum":"-5","average":"-1.6667"}}}"#
        );
    }
}
