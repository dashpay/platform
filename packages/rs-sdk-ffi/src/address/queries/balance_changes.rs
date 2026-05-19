//! Recent address balance changes query operations

use dash_sdk::dpp::balances::credits::CreditOperation;
use dash_sdk::platform::query::RecentAddressBalanceChangesQuery;
use dash_sdk::platform::Fetch;
use drive_proof_verifier::types::RecentAddressBalanceChanges;
use std::panic::{self, AssertUnwindSafe};

use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKAddressBalanceChange, DashSDKBlockBalanceChanges, DashSDKCreditOperationType,
    DashSDKRecentBalanceChanges, SDKHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch recent address balance changes starting from a specific block height.
///
/// This returns all address balance changes that occurred since the specified start height.
/// Useful for syncing wallet balances after the initial sync.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `start_height`: Block height to start fetching changes from
/// - `prove`: Whether to include proofs (currently always fetches with proofs for verification)
///
/// # Returns
/// DashSDKResult with data_type = RecentBalanceChanges containing block-by-block changes
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer.
/// - On success, returns a DashSDKRecentBalanceChanges pointer; caller must free it using `dash_sdk_recent_balance_changes_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_fetch_recent_balance_changes(
    sdk_handle: *const SDKHandle,
    start_height: u64,
) -> DashSDKResult {
    // SAFETY: catch_unwind is kept intentionally despite `panic = "abort"` in the release profile.
    // With panic=abort, catch_unwind is optimized away (zero cost). But keeping it:
    // 1. Acts as a safety net if the panic strategy is ever changed (e.g., for debugging)
    // 2. Documents the intent that panics must not cross this FFI boundary
    // 3. Follows defense-in-depth for FFI safety
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        dash_sdk_address_fetch_recent_balance_changes_inner(sdk_handle, start_height)
    }));

    match result {
        Ok(result) => result,
        Err(panic_info) => {
            let panic_message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred during recent balance changes fetch".to_string()
            };
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!(
                    "Panic during recent balance changes fetch: {}",
                    panic_message
                ),
            ))
        }
    }
}

unsafe fn dash_sdk_address_fetch_recent_balance_changes_inner(
    sdk_handle: *const SDKHandle,
    start_height: u64,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let result: Result<DashSDKRecentBalanceChanges, FFIError> = wrapper.runtime.block_on(async {
        // Fetch recent balance changes using Fetch trait
        let query = RecentAddressBalanceChangesQuery::new(start_height);
        let changes_opt = RecentAddressBalanceChanges::fetch(&wrapper.sdk, query)
            .await
            .map_err(FFIError::from)?;

        let changes = changes_opt.unwrap_or_default();
        let block_changes = changes.into_inner();

        // Convert to FFI format
        let mut blocks: Vec<DashSDKBlockBalanceChanges> = Vec::new();

        for block in block_changes {
            let mut address_changes: Vec<DashSDKAddressBalanceChange> = Vec::new();

            for (address, operation) in block.changes.iter() {
                let address_bytes = address.to_bytes();
                let address_len = address_bytes.len();
                let address_ptr = Box::into_raw(address_bytes.into_boxed_slice()) as *mut u8;

                let (op_type, credits) = match operation {
                    CreditOperation::SetCredits(c) => (DashSDKCreditOperationType::SetCredits, *c),
                    CreditOperation::AddToCredits(c) => {
                        (DashSDKCreditOperationType::AddToCredits, *c)
                    }
                };

                address_changes.push(DashSDKAddressBalanceChange {
                    address: address_ptr,
                    address_len,
                    operation_type: op_type,
                    credits,
                });
            }

            let changes_count = address_changes.len();
            let changes_ptr = if address_changes.is_empty() {
                std::ptr::null_mut()
            } else {
                Box::into_raw(address_changes.into_boxed_slice())
                    as *mut DashSDKAddressBalanceChange
            };

            blocks.push(DashSDKBlockBalanceChanges {
                block_height: block.block_height,
                changes: changes_ptr,
                changes_count,
            });
        }

        let blocks_count = blocks.len();
        let blocks_ptr = if blocks.is_empty() {
            std::ptr::null_mut()
        } else {
            Box::into_raw(blocks.into_boxed_slice()) as *mut DashSDKBlockBalanceChanges
        };

        Ok(DashSDKRecentBalanceChanges {
            blocks: blocks_ptr,
            blocks_count,
        })
    });

    match result {
        Ok(changes) => DashSDKResult::success_recent_balance_changes(changes),
        Err(e) => DashSDKResult::error(e.into()),
    }
}
