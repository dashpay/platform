//! Sum-side FFI entry. Mirror of [`super::count::dash_sdk_document_count`]
//! for the sum surface — `SELECT SUM(sum_property)` over a where
//! clause + optional group_by.
//!
//! The result shape depends on whether `group_by` is requested, and
//! the call routes to a *different* rs-sdk type for each:
//!
//! - **Ungrouped** (empty / null `group_by_json`): the aggregate
//!   [`drive_proof_verifier::DocumentSum`] (a single folded `i128`),
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
//! the signed result values.
//!
//! Every sum is written as a base-10 JSON string, not a JSON number. From
//! protocol version 15 a document type can keep its sums in 128-bit
//! trees, so a sum can pass `i64`; a JSON number that large is rounded by
//! most JSON readers (Swift's `JSONSerialization`, JavaScript), while a
//! decimal string reaches every caller whole.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dash_sdk::drive::query::SelectProjection;
use dash_sdk::platform::Fetch;
use drive_proof_verifier::{DocumentSplitSums, DocumentSum};
use serde::{Serialize, Serializer};

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

/// Serialize an `i128` aggregate as its base-10 string (`"-5"`,
/// `"18446744073709551614"`) so no JSON reader rounds a value past 2^53.
pub(super) fn serialize_i128_as_decimal<S: Serializer>(
    value: &i128,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}

/// Serialize a map of `i128` aggregates with each value as its base-10
/// string; see [`serialize_i128_as_decimal`].
fn serialize_i128_map_as_decimal<S: Serializer>(
    map: &BTreeMap<String, i128>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_map(map.iter().map(|(key, value)| (key, value.to_string())))
}

#[derive(Debug, Serialize)]
struct DocumentSumResult {
    /// Sum results keyed by group.
    ///
    /// - **Ungrouped** (empty / null `group_by_json`): exactly one
    ///   entry under the empty (`""`) key — the aggregate total folded
    ///   across every matched branch. An absent / `None` aggregate
    ///   yields `{"": "0"}` so the "single empty-key total" contract
    ///   always holds.
    /// - **Grouped** (non-empty `group_by_json`): one entry per matched
    ///   group, hex-encoded so iOS callers can match them against the
    ///   corresponding platform-value-encoded property bytes.
    ///
    /// Values are signed `i128` (a 128-bit sum tree can hold more than
    /// `i64`) and serialize as base-10 strings.
    #[serde(serialize_with = "serialize_i128_map_as_decimal")]
    sums: BTreeMap<String, i128>,
}

/// `SELECT SUM(<sum_property>)` over a where clause + optional group_by.
///
/// Returns a JSON string of shape
/// `{"sums": {"<key>": "<signed decimal>", ...}}`, for example
/// `{"sums": {"": "18446744073709551614"}}` or `{"sums": {"61": "-5"}}`.
/// Each sum is an `i128` written as a base-10 string, not a JSON number,
/// so a sum past `i64` (possible from protocol version 15) reaches the
/// caller exactly; callers parse it with an `i128`-capable integer type.
/// The map's shape depends on whether `group_by_json` is requested:
///
/// - **Ungrouped** (empty/null `group_by_json`): exactly one entry under
///   the empty (`""`) key — `sums[""]` is the aggregate total. This
///   routes to the aggregate [`drive_proof_verifier::DocumentSum`],
///   which folds every verified branch (including any `in` / range fork
///   in the `where` clause) into one `i128`. An absent / `None`
///   aggregate is reported as `{"": "0"}` so the single empty-key total
///   contract always holds.
/// - **Grouped** (non-empty `group_by_json`): one entry per matched
///   group, keyed by the hex-encoded platform-value-encoded property
///   value from the underlying sum-tree path; iOS callers should
///   hex-decode them and decode against the contract's index-property
///   type if they need a typed key. This routes to the per-group
///   [`drive_proof_verifier::DocumentSplitSums`] view. Compound
///   `(in_key, key)` entries are collapsed into a flat map by summing
///   each In-fork's contribution at the same terminator key; the fold
///   uses checked `i128` arithmetic and surfaces overflow as an error
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
        // `DocumentSum` folds every verified branch into a single `i128`.
        let sums: BTreeMap<String, i128> = if group_by.is_empty() {
            // Ungrouped: aggregate total under the empty (`""`) key.
            // `DocumentSum` is a tuple struct over a single folded `i128`;
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
            // entries by summing over `in_key` with checked `i128`
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
pub(super) mod tests {
    //! Unit tests for the sum-side FFI surface: the empty-property
    //! rejection (extracted into [`validate_aggregation_property`]), the
    //! exact JSON wire shape iOS callers decode, and calls of
    //! [`dash_sdk_document_sum`] itself against a mock SDK, which run the
    //! production result projection (`try_into_flat_map` and the hex key)
    //! offline. The mock helpers are shared with the average tests.

    use super::*;
    use crate::test_utils::test_utils::destroy_mock_sdk_handle;
    use crate::types::dash_sdk_string_free;
    use crate::{dash_sdk_data_contract_destroy, dash_sdk_error_free};
    use dash_sdk::dapi_grpc::platform::v0::GetDocumentsRequest;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::dpp::version::PlatformVersion;
    use dash_sdk::mock::MockResponse;
    use dash_sdk::platform::documents::document_query::DocumentQuery;
    use dash_sdk::SdkBuilder;
    use drive_proof_verifier::SplitSumEntry;
    use std::ptr;

    /// The document history system contract's `priceUpdate` type keeps
    /// `price` aggregates per `dataContractId`.
    pub(in crate::document::queries) const DOCUMENT_TYPE: &str = "priceUpdate";
    pub(in crate::document::queries) const PRICE: &str = "price";
    pub(in crate::document::queries) const GROUP_BY_FIELD: &str = "dataContractId";
    pub(in crate::document::queries) const GROUP_BY_JSON: &str = r#"["dataContractId"]"#;
    /// Two `dataContractId` values, the keys of two carrier totals.
    pub(in crate::document::queries) const GROUP_A: [u8; 32] = [0xaa; 32];
    pub(in crate::document::queries) const GROUP_B: [u8; 32] = [0xbb; 32];

    pub(in crate::document::queries) fn document_history_contract() -> DataContract {
        load_system_data_contract(
            SystemDataContract::DocumentHistory,
            PlatformVersion::latest(),
        )
        .expect("document history contract")
    }

    /// The `where` clause of an ungrouped call: one `dataContractId` and a
    /// `$createdAt` range, which the `byContract` index (`dataContractId`,
    /// `$createdAt`, averageable `price`, range averageable) answers.
    pub(in crate::document::queries) fn ungrouped_where_json() -> String {
        serde_json::json!([
            { "field": GROUP_BY_FIELD, "operator": "==", "value": bs58::encode(GROUP_A).into_string() },
            { "field": "$createdAt", "operator": ">", "value": 1 },
        ])
        .to_string()
    }

    /// The `where` clause of a call grouped by `dataContractId`: an `in`
    /// on it and a range on `$createdAt`, so each group verifies as a
    /// carrier total keyed by its `In` value.
    pub(in crate::document::queries) fn grouped_where_json() -> String {
        serde_json::json!([
            {
                "field": GROUP_BY_FIELD,
                "operator": "in",
                "value": [
                    bs58::encode(GROUP_A).into_string(),
                    bs58::encode(GROUP_B).into_string(),
                ],
            },
            { "field": "$createdAt", "operator": ">", "value": 1 },
        ])
        .to_string()
    }

    /// The query the FFI builds for `where_json`, a null `order_by`, limit
    /// `-1`, the given projection and the given `group_by` fields (empty
    /// for an ungrouped call). The base query comes from the FFI's own
    /// [`build_base_query`], so the where clause decodes the same way.
    pub(in crate::document::queries) fn ffi_query(
        contract: &DataContract,
        where_json: &str,
        select: SelectProjection,
        group_by: &[&str],
    ) -> DocumentQuery {
        let document_type = CString::new(DOCUMENT_TYPE).expect("no NUL");
        let where_json = CString::new(where_json).expect("no NUL");
        let limit = decode_ffi_limit(-1).expect("-1 is the server default");
        let query = unsafe {
            build_base_query(
                contract,
                document_type.as_ptr(),
                where_json.as_ptr(),
                ptr::null(),
            )
        }
        .expect("the where clause decodes")
        .with_select(select);
        let query = if group_by.is_empty() {
            query
        } else {
            query.with_group_by_fields(group_by.iter().copied())
        };
        query.with_limit(limit)
    }

    /// A mock SDK handle, pinned to the latest protocol version so the
    /// aggregate request encodes, that answers `O::fetch(query)` with
    /// `answer`.
    pub(in crate::document::queries) fn mock_sdk_answering<O>(
        query: DocumentQuery,
        answer: O,
    ) -> *mut SDKHandle
    where
        O: Fetch<Query = DocumentQuery, Request = GetDocumentsRequest> + MockResponse,
    {
        let mut wrapper = SDKWrapper::new_mock();
        wrapper.sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("mock SDK");

        // Registering expectations is async, and the FFI entry point spins
        // up its own runtime and blocks on it, so this runtime must be gone
        // by the time it runs.
        let setup_runtime = tokio::runtime::Runtime::new().expect("create setup runtime");
        setup_runtime.block_on(async {
            wrapper
                .sdk
                .mock()
                .expect_fetch::<O, _>(query, Some(answer))
                .await
                .expect("register the aggregate expectation");
        });
        drop(setup_runtime);

        Box::into_raw(Box::new(wrapper)) as *mut SDKHandle
    }

    /// The JSON a successful call returned, freeing the result string; a
    /// failed call panics with its error message after freeing the error.
    pub(in crate::document::queries) unsafe fn json_of(result: DashSDKResult) -> serde_json::Value {
        if !result.error.is_null() {
            let message = CStr::from_ptr((*result.error).message)
                .to_string_lossy()
                .into_owned();
            dash_sdk_error_free(result.error);
            panic!("the call must succeed; it failed with: {message}");
        }
        assert!(!result.data.is_null(), "a successful call returns a string");
        let text = result.data as *mut c_char;
        let value = serde_json::from_str(&CStr::from_ptr(text).to_string_lossy())
            .expect("the result is JSON");
        dash_sdk_string_free(text);
        value
    }

    /// Calls an aggregate FFI entry point (`dash_sdk_document_sum` or
    /// `dash_sdk_document_average`, which share a signature) on a real
    /// contract handle with `where_json`, a null `order_by`, limit `-1` and
    /// the given `group_by` JSON, and returns its JSON. Destroys both
    /// handles.
    pub(in crate::document::queries) fn call_aggregate(
        entry_point: unsafe extern "C" fn(
            *const SDKHandle,
            *const DataContractHandle,
            *const c_char,
            *const c_char,
            *const c_char,
            *const c_char,
            *const c_char,
            i64,
        ) -> DashSDKResult,
        sdk_handle: *mut SDKHandle,
        contract: DataContract,
        where_json: &str,
        group_by_json: Option<&str>,
    ) -> serde_json::Value {
        let contract_handle = Box::into_raw(Box::new(contract)) as *mut DataContractHandle;
        let document_type = CString::new(DOCUMENT_TYPE).expect("no NUL");
        let property = CString::new(PRICE).expect("no NUL");
        let where_json = CString::new(where_json).expect("no NUL");
        let group_by = group_by_json.map(|json| CString::new(json).expect("no NUL"));
        let json = unsafe {
            json_of(entry_point(
                sdk_handle,
                contract_handle,
                document_type.as_ptr(),
                property.as_ptr(),
                where_json.as_ptr(),
                ptr::null(),
                group_by.as_ref().map_or(ptr::null(), |json| json.as_ptr()),
                -1,
            ))
        };
        unsafe { dash_sdk_data_contract_destroy(contract_handle) };
        destroy_mock_sdk_handle(sdk_handle);
        json
    }

    /// An ungrouped call returns the folded `DocumentSum` under the empty
    /// key, a total past `i64` whole as a decimal string.
    #[test]
    fn should_return_an_ungrouped_sum_past_i64_as_one_decimal_string() {
        let contract = document_history_contract();
        let where_json = ungrouped_where_json();
        let query = ffi_query(&contract, &where_json, SelectProjection::sum(PRICE), &[]);
        let sdk = mock_sdk_answering(query, DocumentSum(3 * i128::from(i64::MAX)));

        let json = call_aggregate(dash_sdk_document_sum, sdk, contract, &where_json, None);

        assert_eq!(
            json,
            serde_json::json!({ "sums": { "": "27670116110564327421" } })
        );
    }

    /// A call grouped by the `In` field whose groups verify as carrier
    /// totals, each keyed by its `In` value, returns one entry per group,
    /// keyed by the hex of that value.
    #[test]
    fn should_key_grouped_carrier_sums_by_their_in_value() {
        let contract = document_history_contract();
        let where_json = grouped_where_json();
        let query = ffi_query(
            &contract,
            &where_json,
            SelectProjection::sum(PRICE),
            &[GROUP_BY_FIELD],
        );
        let sdk = mock_sdk_answering(
            query,
            DocumentSplitSums(vec![
                SplitSumEntry {
                    in_key: None,
                    key: GROUP_A.to_vec(),
                    sum: Some(3 * i128::from(i64::MAX)),
                },
                SplitSumEntry {
                    in_key: None,
                    key: GROUP_B.to_vec(),
                    sum: Some(70005),
                },
            ]),
        );

        let json = call_aggregate(
            dash_sdk_document_sum,
            sdk,
            contract,
            &where_json,
            Some(GROUP_BY_JSON),
        );

        let mut expected = serde_json::Map::new();
        expected.insert(hex::encode(GROUP_A), "27670116110564327421".into());
        expected.insert(hex::encode(GROUP_B), "70005".into());
        assert_eq!(json, serde_json::json!({ "sums": expected }));
    }

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

    /// Serialize a sum map through the real [`DocumentSumResult`] type.
    fn sums_json(sums: BTreeMap<String, i128>) -> String {
        serde_json::to_string(&DocumentSumResult { sums })
            .expect("DocumentSumResult must serialize")
    }

    /// The `DocumentSumResult` wire shape iOS decodes is exactly
    /// `{"sums": {"<hex-key>": "<signed decimal>", ...}}`. The empty key
    /// (aggregate total) and a negative value must round-trip verbatim;
    /// `BTreeMap` ordering keeps the key order deterministic.
    #[test]
    fn document_sum_result_serializes_to_expected_shape() {
        let sums = BTreeMap::from([("".to_string(), 42i128), ("61".to_string(), -5i128)]);
        assert_eq!(sums_json(sums), r#"{"sums":{"":"42","61":"-5"}}"#);
    }

    /// The ungrouped (aggregate) branch always emits a single
    /// empty-string-keyed total. This pins the exact wire shape the
    /// `group_by.is_empty()` path produces (`{"sums":{"":"<total>"}}`)
    /// so a regression that reintroduced per-key entries for an
    /// ungrouped `in` / range query would change the serialized bytes
    /// iOS decodes against.
    #[test]
    fn document_sum_result_ungrouped_is_single_empty_key_total() {
        let sums = BTreeMap::from([(String::new(), 42i128)]);
        assert_eq!(sums_json(sums), r#"{"sums":{"":"42"}}"#);
    }

    /// An absent / `None` aggregate (queried-but-empty) is reported as
    /// the zero total under the empty key, never an empty map, so the
    /// documented "single empty-key total" contract always holds.
    #[test]
    fn document_sum_result_ungrouped_absent_is_zero_total() {
        let sums = BTreeMap::from([(String::new(), 0i128)]);
        assert_eq!(sums_json(sums), r#"{"sums":{"":"0"}}"#);
    }

    /// A sum past `i64` (a 128-bit sum tree, protocol version 15) is
    /// written whole as a decimal string, not a JSON number a reader
    /// would round.
    #[test]
    fn document_sum_result_carries_a_sum_past_i64_whole() {
        let wide = 3 * i128::from(i64::MAX);
        let sums = BTreeMap::from([(String::new(), wide)]);
        assert_eq!(sums_json(sums), r#"{"sums":{"":"27670116110564327421"}}"#);
    }

    /// A negative sum past `i64::MIN` keeps its sign and every digit.
    #[test]
    fn document_sum_result_carries_a_negative_sum_past_i64_whole() {
        let wide = 3 * i128::from(i64::MIN);
        let sums = BTreeMap::from([("61".to_string(), wide)]);
        assert_eq!(
            sums_json(sums),
            r#"{"sums":{"61":"-27670116110564327424"}}"#
        );
    }
}
