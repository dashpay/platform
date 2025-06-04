//! Signer interface for iOS FFI

use crate::types::SignerHandle;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::platform_value::BinaryData;
use dash_sdk::dpp::prelude::{IdentityPublicKey, ProtocolError};

/// Function pointer type for iOS signing callback
/// Returns pointer to allocated byte array (caller must free with dash_sdk_bytes_free)
/// Returns null on error
pub type IOSSignCallback = unsafe extern "C" fn(
    identity_public_key_bytes: *const u8,
    identity_public_key_len: usize,
    data: *const u8,
    data_len: usize,
    result_len: *mut usize,
) -> *mut u8;

/// Function pointer type for iOS can_sign_with callback
pub type IOSCanSignCallback = unsafe extern "C" fn(
    identity_public_key_bytes: *const u8,
    identity_public_key_len: usize,
) -> bool;

/// iOS FFI Signer that bridges to iOS signing callbacks
#[derive(Debug, Clone, Copy)]
pub struct IOSSigner {
    sign_callback: IOSSignCallback,
    can_sign_callback: IOSCanSignCallback,
}

impl IOSSigner {
    pub fn new(sign_callback: IOSSignCallback, can_sign_callback: IOSCanSignCallback) -> Self {
        IOSSigner {
            sign_callback,
            can_sign_callback,
        }
    }
}

impl Signer for IOSSigner {
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let key_bytes = identity_public_key.data().as_slice();
        let mut result_len: usize = 0;

        let result_ptr = unsafe {
            (self.sign_callback)(
                key_bytes.as_ptr(),
                key_bytes.len(),
                data.as_ptr(),
                data.len(),
                &mut result_len,
            )
        };

        if result_ptr.is_null() {
            return Err(ProtocolError::Generic(
                "iOS signing callback returned null".to_string(),
            ));
        }

        // Convert the result to BinaryData
        let signature_bytes =
            unsafe { std::slice::from_raw_parts(result_ptr, result_len).to_vec() };

        // Free the memory allocated by iOS
        unsafe {
            dash_sdk_bytes_free(result_ptr);
        }

        Ok(signature_bytes.into())
    }

    fn can_sign_with(&self, identity_public_key: &IdentityPublicKey) -> bool {
        let key_bytes = identity_public_key.data().as_slice();

        unsafe { (self.can_sign_callback)(key_bytes.as_ptr(), key_bytes.len()) }
    }
}

/// Create a new iOS signer
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signer_create(
    sign_callback: IOSSignCallback,
    can_sign_callback: IOSCanSignCallback,
) -> *mut SignerHandle {
    let signer = IOSSigner::new(sign_callback, can_sign_callback);
    Box::into_raw(Box::new(signer)) as *mut SignerHandle
}

/// Destroy an iOS signer
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signer_destroy(handle: *mut SignerHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut IOSSigner);
    }
}

/// Free bytes allocated by iOS callbacks
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_bytes_free(bytes: *mut u8) {
    if !bytes.is_null() {
        // Note: This assumes iOS allocates with malloc/calloc
        // If iOS uses a different allocator, this function needs to be updated
        libc::free(bytes as *mut libc::c_void);
    }
}
