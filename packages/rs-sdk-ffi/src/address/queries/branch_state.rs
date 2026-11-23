//! Branch state query operations for address synchronization

use dash_sdk::dapi_client::{DapiRequest, IntoInner, RequestSettings};
use dash_sdk::dapi_grpc::platform::v0::{
    get_addresses_branch_state_request, get_addresses_branch_state_response,
    GetAddressesBranchStateRequest, GetAddressesBranchStateResponse,
};
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::drive::drive::Drive;
use dash_sdk::platform::address_sync::address_branch_query_depth;
use std::panic::{self, AssertUnwindSafe};

use super::address_funds_elements::address_funds_elements_into_ffi;
use crate::sdk::SDKWrapper;
use crate::types::{DashSDKBranchState, DashSDKLeafBoundary, DashSDKTrunkStateElement, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Returned by [`dash_sdk_address_branch_query_depth`] for a null handle; no
/// valid branch query depth is 0.
const NO_BRANCH_QUERY_DEPTH: u8 = 0;

/// The depth to pass to [`dash_sdk_address_fetch_branch_state`] for a leaf
/// boundary of about `estimated_count` elements (0 when the count is
/// unknown), under the SDK's platform version.
///
/// This is the depth the address sync picks for such a leaf; see
/// `dash_sdk::platform::address_sync::address_branch_query_depth`. Returns
/// 0 for a null handle.
///
/// # Safety
/// `sdk_handle` must be null or a valid SDK handle for the duration of this
/// call.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_branch_query_depth(
    sdk_handle: *const SDKHandle,
    estimated_count: u64,
) -> u8 {
    if sdk_handle.is_null() {
        return NO_BRANCH_QUERY_DEPTH;
    }
    let wrapper = &*(sdk_handle as *const SDKWrapper);
    address_branch_query_depth(estimated_count, wrapper.sdk.version())
}

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
    // SAFETY: catch_unwind is kept intentionally despite `panic = "abort"` in the release profile.
    // With panic=abort, catch_unwind is optimized away (zero cost). But keeping it:
    // 1. Acts as a safety net if the panic strategy is ever changed (e.g., for debugging)
    // 2. Documents the intent that panics must not cross this FFI boundary
    // 3. Follows defense-in-depth for FFI safety
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

    // The query depth is a `u8` in GroveDB; a wider value is refused here
    // rather than truncated. The range itself is checked by the server and
    // by the proof verification below, from the platform version.
    let Ok(depth_u8) = u8::try_from(depth) else {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!("Depth {depth} is not a valid branch query depth"),
        ));
    };

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

        // Branch responses have no signed metadata; the caller authenticates
        // their root through a trunk proof. Address-funds leaves are item rows,
        // whose proof encoding the newest GroveDB rules preserve, so these
        // rules are compatible and strongest even for a pinned SDK. Revisit
        // this choice if item-row proof encoding changes, address-funds gains
        // subtree children, or a newer table changes the branch verification
        // method version or depth bounds.
        let platform_version = PlatformVersion::latest();

        // Verify the proof and get branch result
        let branch_result = Drive::verify_address_funds_branch_query(
            &proof_bytes,
            key_vec,
            depth_u8,
            hash_array,
            platform_version,
        )
        .map_err(|e| FFIError::InternalError(format!("Proof verification failed: {}", e)))?;

        // Decode every element as address funds; an undecodable one fails
        // the call instead of being dropped.
        let elements = address_funds_elements_into_ffi(branch_result.elements.iter())?;

        // Convert leaf keys to FFI format
        let mut leaf_boundaries: Vec<DashSDKLeafBoundary> = Vec::new();
        for (leaf_key, leaf_info) in branch_result.leaf_keys.iter() {
            let leaf_key_vec: Vec<u8> = leaf_key.clone();
            let leaf_key_len = leaf_key_vec.len();
            let leaf_key_ptr = Box::into_raw(leaf_key_vec.into_boxed_slice()) as *mut u8;

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
            Box::into_raw(elements.into_boxed_slice()) as *mut DashSDKTrunkStateElement
        };

        let leaf_boundaries_count = leaf_boundaries.len();
        let leaf_boundaries_ptr = if leaf_boundaries.is_empty() {
            std::ptr::null_mut()
        } else {
            Box::into_raw(leaf_boundaries.into_boxed_slice()) as *mut DashSDKLeafBoundary
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

#[cfg(test)]
mod tests {
    use dash_sdk::platform::address_sync::address_branch_query_depth;

    use super::dash_sdk_address_branch_query_depth;
    use crate::sdk::SDKWrapper;
    use crate::test_utils::test_utils::{create_mock_sdk_handle, destroy_mock_sdk_handle};

    #[test]
    fn should_return_zero_branch_query_depth_for_a_null_handle() {
        let depth = unsafe { dash_sdk_address_branch_query_depth(std::ptr::null(), 100) };

        assert_eq!(depth, 0);
    }

    #[test]
    fn should_return_the_sdk_branch_query_depth_for_the_handle_version() {
        let handle = create_mock_sdk_handle();
        let platform_version = unsafe { &*(handle as *const SDKWrapper) }.sdk.version();

        for estimated_count in [0, 1, 33, 54, 1_000_000] {
            let depth = unsafe { dash_sdk_address_branch_query_depth(handle, estimated_count) };
            assert_eq!(
                depth,
                address_branch_query_depth(estimated_count, platform_version)
            );
        }

        destroy_mock_sdk_handle(handle);
    }
}
