use std::ptr;

/// Create a test signer for development/testing purposes
#[no_mangle]
pub extern "C" fn swift_dash_signer_create_test() -> *mut ios_sdk_ffi::SignerHandle {
    unsafe extern "C" fn test_sign_callback(
        _identity_public_key_bytes: *const u8,
        _identity_public_key_len: usize,
        _data: *const u8,
        data_len: usize,
        result_len: *mut usize,
    ) -> *mut u8 {
        // Return a dummy signature for testing
        let dummy_signature = vec![0u8; 64]; // Typical signature size
        *result_len = dummy_signature.len();
        
        // Allocate memory that can be freed by ios_sdk_bytes_free
        let ptr = libc::malloc(dummy_signature.len()) as *mut u8;
        if !ptr.is_null() {
            std::ptr::copy_nonoverlapping(
                dummy_signature.as_ptr(),
                ptr,
                dummy_signature.len(),
            );
        }
        ptr
    }

    unsafe extern "C" fn test_can_sign_callback(
        _identity_public_key_bytes: *const u8,
        _identity_public_key_len: usize,
    ) -> bool {
        true // Can always sign in test mode
    }

    unsafe {
        ios_sdk_ffi::ios_sdk_signer_create(test_sign_callback, test_can_sign_callback)
    }
}

/// Destroy a signer
#[no_mangle]
pub unsafe extern "C" fn swift_dash_signer_destroy(handle: *mut ios_sdk_ffi::SignerHandle) {
    if !handle.is_null() {
        ios_sdk_ffi::ios_sdk_signer_destroy(handle);
    }
}