//! Branch state query operations for address synchronization

use dash_sdk::dapi_client::{DapiRequest, IntoInner, RequestSettings};
use dash_sdk::dapi_grpc::platform::v0::{
    get_addresses_branch_state_request, get_addresses_branch_state_response,
    GetAddressesBranchStateRequest,
};
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::drive::drive::Drive;
use std::panic::{self, AssertUnwindSafe};

use crate::sdk::SDKWrapper;
use crate::types::{DashSDKBranchState, DashSDKLeafBoundary, DashSDKTrunkStateElement, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch the branch state of a subtree in the address tree.
///
/// This is used after a trunk state query to explore subtrees indicated by leaf boundaries.
/// The result contains elements (addresses with balances) and deeper leaf boundaries.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `key`: Leaf boundary key bytes from trunk state
/// - `key_len`: Length of key bytes
/// - `depth`: Query depth (how deep to explore)
/// - `expected_hash`: Expected hash of the subtree root (32 bytes, for proof verification)
/// - `checkpoint_height`: Block height from trunk state response for consistency
///
/// # Returns
/// DashSDKResult with data_type = BranchState containing elements and leaf boundaries
///
/// # Safety
/// - `sdk_handle`, `key`, and `expected_hash` must be valid, non-null pointers.
/// - `key` must point to a valid byte array of length `key_len`.
/// - `expected_hash` must point to exactly 32 bytes.
/// - On success, returns a DashSDKBranchState pointer; caller must free it using `dash_sdk_branch_state_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_fetch_branch_state(
    sdk_handle: *const SDKHandle,
    key: *const u8,
    key_len: usize,
    depth: u32,
    expected_hash: *const u8,
    checkpoint_height: u64,
) -> DashSDKResult {
    // Wrap the entire function in catch_unwind to prevent panics from crashing the app
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        dash_sdk_address_fetch_branch_state_inner(
            sdk_handle,
            key,
            key_len,
            depth,
            expected_hash,
            checkpoint_height,
        )
    }));

    match result {
        Ok(result) => result,
        Err(panic_info) => {
            let panic_message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred during branch state fetch".to_string()
            };
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Panic during branch state fetch: {}", panic_message),
            ))
        }
    }
}

unsafe fn dash_sdk_address_fetch_branch_state_inner(
    sdk_handle: *const SDKHandle,
    key: *const u8,
    key_len: usize,
    depth: u32,
    expected_hash: *const u8,
    checkpoint_height: u64,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if key.is_null() || key_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Key is null or empty".to_string(),
        ));
    }

    if expected_hash.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Expected hash is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    // Convert inputs to Rust types
    let key_slice = std::slice::from_raw_parts(key, key_len);
    let key_vec: Vec<u8> = key_slice.to_vec();

    let hash_slice = std::slice::from_raw_parts(expected_hash, 32);
    let mut hash_array: [u8; 32] = [0u8; 32];
    hash_array.copy_from_slice(hash_slice);

    let result: Result<DashSDKBranchState, FFIError> = wrapper.runtime.block_on(async {
        // Build the request
        let request = GetAddressesBranchStateRequest {
            version: Some(get_addresses_branch_state_request::Version::V0(
                get_addresses_branch_state_request::GetAddressesBranchStateRequestV0 {
                    key: key_vec.clone(),
                    depth,
                    checkpoint_height,
                },
            )),
        };

        // Execute the request
        use dash_sdk::dapi_grpc::platform::v0::GetAddressesBranchStateResponse;
        let response: GetAddressesBranchStateResponse = request
            .execute(&wrapper.sdk, RequestSettings::default())
            .await
            .map_err(|e| FFIError::InternalError(format!("Branch state request failed: {}", e)))?
            .into_inner();

        // Extract merk proof
        let proof_bytes = match response.version {
            Some(get_addresses_branch_state_response::Version::V0(v0)) => v0.merk_proof,
            None => {
                return Err(FFIError::InternalError(
                    "Empty response version".to_string(),
                ));
            }
        };

        // Get platform version
        let platform_version = PlatformVersion::latest();

        // Verify the proof and get branch result
        let branch_result = Drive::verify_address_funds_branch_query(
            &proof_bytes,
            key_vec,
            depth as u8,
            hash_array,
            platform_version,
        )
        .map_err(|e| FFIError::InternalError(format!("Proof verification failed: {}", e)))?;

        // Convert elements to FFI format
        let mut elements: Vec<DashSDKTrunkStateElement> = Vec::new();
        for (elem_key, element) in branch_result.elements.iter() {
            if let Some((balance, nonce)) = extract_balance_nonce_from_element(element) {
                let elem_key_vec: Vec<u8> = elem_key.clone();
                let elem_key_len = elem_key_vec.len();
                let elem_key_ptr = elem_key_vec.as_ptr() as *mut u8;
                std::mem::forget(elem_key_vec);

                elements.push(DashSDKTrunkStateElement {
                    key: elem_key_ptr,
                    key_len: elem_key_len,
                    nonce,
                    balance,
                });
            }
        }

        // Convert leaf keys to FFI format
        let mut leaf_boundaries: Vec<DashSDKLeafBoundary> = Vec::new();
        for (leaf_key, leaf_info) in branch_result.leaf_keys.iter() {
            let leaf_key_vec: Vec<u8> = leaf_key.clone();
            let leaf_key_len = leaf_key_vec.len();
            let leaf_key_ptr = leaf_key_vec.as_ptr() as *mut u8;
            std::mem::forget(leaf_key_vec);

            leaf_boundaries.push(DashSDKLeafBoundary {
                key: leaf_key_ptr,
                key_len: leaf_key_len,
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

        Ok(DashSDKBranchState {
            elements: elements_ptr,
            elements_count,
            leaf_boundaries: leaf_boundaries_ptr,
            leaf_boundaries_count,
        })
    });

    match result {
        Ok(state) => DashSDKResult::success_branch_state(state),
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
