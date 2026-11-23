//! Trunk state query operations for address synchronization

use dash_sdk::platform::Fetch;
use drive_proof_verifier::types::PlatformAddressTrunkState;
use std::panic::{self, AssertUnwindSafe};

use super::address_funds_elements::address_funds_elements_into_ffi;
use crate::sdk::SDKWrapper;
use crate::types::{DashSDKLeafBoundary, DashSDKTrunkState, DashSDKTrunkStateElement, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch the trunk state of the address tree for privacy-preserving address synchronization.
///
/// The trunk state contains:
/// - Elements: Addresses with balances found at the top levels of the tree
/// - Leaf boundaries: Subtrees that require further branch queries to explore
///
/// # Parameters
/// - `sdk_handle`: SDK handle
///
/// # Returns
/// DashSDKResult with data_type = TrunkState containing elements and leaf boundaries
///
/// # Safety
/// - `sdk_handle` must be a valid, non-null pointer.
/// - On success, returns a DashSDKTrunkState pointer; caller must free it using `dash_sdk_trunk_state_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_fetch_trunk_state(
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    // SAFETY: catch_unwind is kept intentionally despite `panic = "abort"` in the release profile.
    // With panic=abort, catch_unwind is optimized away (zero cost). But keeping it:
    // 1. Acts as a safety net if the panic strategy is ever changed (e.g., for debugging)
    // 2. Documents the intent that panics must not cross this FFI boundary
    // 3. Follows defense-in-depth for FFI safety
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        dash_sdk_address_fetch_trunk_state_inner(sdk_handle)
    }));

    match result {
        Ok(result) => result,
        Err(panic_info) => {
            let panic_message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred during trunk state fetch".to_string()
            };
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Panic during trunk state fetch: {}", panic_message),
            ))
        }
    }
}

unsafe fn dash_sdk_address_fetch_trunk_state_inner(sdk_handle: *const SDKHandle) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let result: Result<DashSDKTrunkState, FFIError> = wrapper.runtime.block_on(async {
        // Fetch trunk state using Fetch trait with empty query (no parameters)
        let (trunk_state_opt, metadata) =
            PlatformAddressTrunkState::fetch_with_metadata(&wrapper.sdk, (), None)
                .await
                .map_err(FFIError::from)?;

        let trunk_state = trunk_state_opt
            .ok_or_else(|| FFIError::NotFound("Trunk state not available".to_string()))?;

        let grove_result = trunk_state.into_inner();
        let checkpoint_height = metadata.height;

        // Decode every element as address funds; an undecodable one fails
        // the call instead of being dropped.
        let elements = address_funds_elements_into_ffi(grove_result.elements.iter())?;

        // Convert leaf keys to FFI format
        // leaf_keys is a BTreeMap<Vec<u8>, LeafInfo>
        let mut leaf_boundaries: Vec<DashSDKLeafBoundary> = Vec::new();
        for (key, leaf_info) in grove_result.leaf_keys.iter() {
            let key_vec: Vec<u8> = key.clone();
            let key_len = key_vec.len();
            let key_ptr = Box::into_raw(key_vec.into_boxed_slice()) as *mut u8;

            leaf_boundaries.push(DashSDKLeafBoundary {
                key: key_ptr,
                key_len,
                hash: leaf_info.hash,
                estimated_count: leaf_info.count.unwrap_or(0),
            });
        }

        // Convert to raw arrays
        let elements_count = elements.len();
        let elements_ptr = if elements.is_empty() {
            std::ptr::null_mut()
        } else {
            Box::into_raw(elements.into_boxed_slice()) as *mut DashSDKTrunkStateElement
        };

        let leaf_boundaries_count = leaf_boundaries.len();
        let leaf_boundaries_ptr = if leaf_boundaries.is_empty() {
            std::ptr::null_mut()
        } else {
            Box::into_raw(leaf_boundaries.into_boxed_slice()) as *mut DashSDKLeafBoundary
        };

        Ok(DashSDKTrunkState {
            elements: elements_ptr,
            elements_count,
            leaf_boundaries: leaf_boundaries_ptr,
            leaf_boundaries_count,
            checkpoint_height,
        })
    });

    match result {
        Ok(state) => DashSDKResult::success_trunk_state(state),
        Err(e) => DashSDKResult::error(e.into()),
    }
}
