//! Trunk state query operations for address synchronization

use dash_sdk::platform::Fetch;
use drive_proof_verifier::types::PlatformAddressTrunkState;
use std::panic::{self, AssertUnwindSafe};

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
    // Wrap the entire function in catch_unwind to prevent panics from crashing the app
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

        // Convert elements to FFI format
        let mut elements: Vec<DashSDKTrunkStateElement> = Vec::new();
        for (key, element) in grove_result.elements.iter() {
            // Extract balance and nonce from Element
            // Element is typically Item with encoded balance/nonce
            if let Some((balance, nonce)) = extract_balance_nonce_from_element(element) {
                let key_vec: Vec<u8> = key.clone();
                let key_len = key_vec.len();
                let key_ptr = key_vec.as_ptr() as *mut u8;
                std::mem::forget(key_vec);

                elements.push(DashSDKTrunkStateElement {
                    key: key_ptr,
                    key_len,
                    nonce,
                    balance,
                });
            }
        }

        // Convert leaf keys to FFI format
        // leaf_keys is a BTreeMap<Vec<u8>, LeafInfo>
        let mut leaf_boundaries: Vec<DashSDKLeafBoundary> = Vec::new();
        for (key, leaf_info) in grove_result.leaf_keys.iter() {
            let key_vec: Vec<u8> = key.clone();
            let key_len = key_vec.len();
            let key_ptr = key_vec.as_ptr() as *mut u8;
            std::mem::forget(key_vec);

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
            let ptr = elements.as_ptr() as *mut DashSDKTrunkStateElement;
            std::mem::forget(elements);
            ptr
        };

        let leaf_boundaries_count = leaf_boundaries.len();
        let leaf_boundaries_ptr = if leaf_boundaries.is_empty() {
            std::ptr::null_mut()
        } else {
            let ptr = leaf_boundaries.as_ptr() as *mut DashSDKLeafBoundary;
            std::mem::forget(leaf_boundaries);
            ptr
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

/// Extract balance and nonce from a GroveDB Element.
/// Returns None if the element format is not recognized.
fn extract_balance_nonce_from_element(
    element: &dash_sdk::drive::grovedb::Element,
) -> Option<(u64, u32)> {
    use dash_sdk::drive::grovedb::Element;

    match element {
        Element::Item(data, _) => {
            // Address funds are encoded as: nonce (4 bytes) + balance (8 bytes)
            if data.len() >= 12 {
                let nonce = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                let balance = u64::from_be_bytes([
                    data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
                ]);
                Some((balance, nonce))
            } else if data.len() >= 8 {
                // Fallback: just balance
                let balance = u64::from_be_bytes([
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                ]);
                Some((balance, 0))
            } else {
                None
            }
        }
        Element::SumItem(balance, _) => {
            // SumItem directly contains the balance
            Some((*balance as u64, 0))
        }
        _ => None,
    }
}
