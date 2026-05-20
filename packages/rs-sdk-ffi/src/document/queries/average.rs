//! Average-side FFI entry. Mirror of [`super::sum::dash_sdk_document_sum`]
//! for the average surface — `SELECT AVG(sum_property)` over a where
//! clause + optional group_by.
//!
//! Averages reuse sum-tree indexes; the underlying server-side primitive
//! returns the `(count, sum)` pair and the client divides. This FFI
//! exposes the raw pair (not a pre-divided average) so iOS/Swift
//! callers can pick their own precision representation.
//!
//! **Status**: skeleton — same gating as `dash_sdk_document_sum`. The
//! actual call into rs-sdk's
//! [`drive_proof_verifier::DocumentSplitAverages::fetch`] depends on
//! grovedb PR 670 landing `verify_aggregate_count_and_sum_query` and
//! the rs-drive executor bodies being filled in. Until then this entry
//! returns a typed `NotImplemented` error so iOS / Swift callers can
//! encode against the stable API and see a clear "feature not yet
//! shipped" rather than a crash.
//!
//! Once those dependencies land:
//!  1. Replace the `NotImplemented` error with a body mirroring
//!     `dash_sdk_document_sum`.
//!  2. Substitute `DocumentSplitSums::fetch` →
//!     `DocumentSplitAverages::fetch`.
//!  3. Keep the `sum_property` parameter (averages aggregate the same
//!     integer property; the divisor is the implicit count).
//!  4. Return JSON of `{"averages": {"<hex-key>": {"count": u64,
//!     "sum": i64}, ...}}` — callers divide.

use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DataContractHandle, SDKHandle};
use std::os::raw::c_char;

/// `SELECT AVG(<sum_property>)` over a where clause + optional group_by.
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
/// - `limit`: -1 for server default, >= 0 for explicit cap.
///
/// # Safety
/// Same contract as [`super::sum::dash_sdk_document_sum`]. All
/// pointers must be valid for the duration of the call.
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
    let _ = (
        sdk_handle,
        data_contract_handle,
        document_type,
        sum_property,
        where_json,
        order_by_json,
        group_by_json,
        limit,
    );
    DashSDKResult::error(DashSDKError::new(
        DashSDKErrorCode::NotImplemented,
        "dash_sdk_document_average: not yet implemented. Waits on grovedb PR 670 (\
             verify_aggregate_count_and_sum_query) and the rs-drive count+sum executor \
             bodies in drive_document_sum_query/executors/. Same gating as \
             dash_sdk_document_sum — see the rs-drive `grovedb_pr_670` catalog module \
             for the full dependency list."
            .to_string(),
    ))
}
