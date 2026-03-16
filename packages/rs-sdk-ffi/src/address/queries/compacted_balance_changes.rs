//! Recent compacted address balance changes query operations

use dash_sdk::dpp::balances::credits::BlockAwareCreditOperation;
use dash_sdk::platform::query::RecentCompactedAddressBalanceChangesQuery;
use dash_sdk::platform::Fetch;
use drive_proof_verifier::types::RecentCompactedAddressBalanceChanges;
use std::panic::{self, AssertUnwindSafe};

use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKBlockAwareCreditOperationType, DashSDKBlockHeightCreditEntry,
    DashSDKCompactedAddressChange, DashSDKCompactedBalanceChanges, DashSDKCompactedBlockRange,
    SDKHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch recent compacted address balance changes starting from a specific block height.
///
/// This returns compacted (merged) address balance changes since the specified start height.
/// Useful for efficient syncing when blocks have been merged into ranges.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `start_block_height`: Block height to start fetching changes from
///
/// # Returns
/// DashSDKResult with data_type = CompactedBalanceChanges containing range-by-range compacted changes
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer.
/// - On success, returns a DashSDKCompactedBalanceChanges pointer; caller must free it using `dash_sdk_compacted_balance_changes_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_fetch_compacted_balance_changes(
    sdk_handle: *const SDKHandle,
    start_block_height: u64,
) -> DashSDKResult {
    // Wrap the entire function in catch_unwind to prevent panics from crashing the app
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        dash_sdk_address_fetch_compacted_balance_changes_inner(sdk_handle, start_block_height)
    }));

    match result {
        Ok(result) => result,
        Err(panic_info) => {
            let panic_message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred during compacted balance changes fetch".to_string()
            };
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!(
                    "Panic during compacted balance changes fetch: {}",
                    panic_message
                ),
            ))
        }
    }
}

unsafe fn dash_sdk_address_fetch_compacted_balance_changes_inner(
    sdk_handle: *const SDKHandle,
    start_block_height: u64,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let result: Result<DashSDKCompactedBalanceChanges, FFIError> =
        wrapper.runtime.block_on(async {
            // Fetch compacted balance changes using Fetch trait
            let query = RecentCompactedAddressBalanceChangesQuery::new(start_block_height);
            let changes_opt = RecentCompactedAddressBalanceChanges::fetch(&wrapper.sdk, query)
                .await
                .map_err(FFIError::from)?;

            let changes = changes_opt.unwrap_or_default();
            let compacted_ranges = changes.into_inner();

            // Convert to FFI format
            let mut ranges: Vec<DashSDKCompactedBlockRange> = Vec::new();

            for range in compacted_ranges {
                let mut address_changes: Vec<DashSDKCompactedAddressChange> = Vec::new();

                for (address, operation) in range.changes.iter() {
                    let address_bytes = address.to_bytes();
                    let address_len = address_bytes.len();
                    let address_ptr =
                        Box::into_raw(address_bytes.into_boxed_slice()) as *mut u8;

                    let (op_type, set_value, entries_ptr, entries_count) = match operation {
                        BlockAwareCreditOperation::SetCredits(credits) => (
                            DashSDKBlockAwareCreditOperationType::BlockAwareSetCredits,
                            *credits,
                            std::ptr::null_mut(),
                            0,
                        ),
                        BlockAwareCreditOperation::AddToCreditsOperations(map) => {
                            let entries: Vec<DashSDKBlockHeightCreditEntry> = map
                                .iter()
                                .map(|(height, credits)| DashSDKBlockHeightCreditEntry {
                                    block_height: *height,
                                    credits: *credits,
                                })
                                .collect();

                            let count = entries.len();
                            let ptr = if entries.is_empty() {
                                std::ptr::null_mut()
                            } else {
                                Box::into_raw(entries.into_boxed_slice())
                                    as *mut DashSDKBlockHeightCreditEntry
                            };

                            (
                                DashSDKBlockAwareCreditOperationType::BlockAwareAddToCreditsOperations,
                                0,
                                ptr,
                                count,
                            )
                        }
                    };

                    address_changes.push(DashSDKCompactedAddressChange {
                        address: address_ptr,
                        address_len,
                        operation_type: op_type,
                        set_credits_value: set_value,
                        add_entries: entries_ptr,
                        add_entries_count: entries_count,
                    });
                }

                let changes_count = address_changes.len();
                let changes_ptr = if address_changes.is_empty() {
                    std::ptr::null_mut()
                } else {
                    Box::into_raw(address_changes.into_boxed_slice())
                        as *mut DashSDKCompactedAddressChange
                };

                ranges.push(DashSDKCompactedBlockRange {
                    start_block_height: range.start_block_height,
                    end_block_height: range.end_block_height,
                    changes: changes_ptr,
                    changes_count,
                });
            }

            let ranges_count = ranges.len();
            let ranges_ptr = if ranges.is_empty() {
                std::ptr::null_mut()
            } else {
                Box::into_raw(ranges.into_boxed_slice()) as *mut DashSDKCompactedBlockRange
            };

            Ok(DashSDKCompactedBalanceChanges {
                ranges: ranges_ptr,
                ranges_count,
            })
        });

    match result {
        Ok(changes) => DashSDKResult::success_compacted_balance_changes(changes),
        Err(e) => DashSDKResult::error(e.into()),
    }
}
