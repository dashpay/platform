//! Signer interface for iOS FFI

use crate::types::SignerHandle;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::platform_value::BinaryData;
use dash_sdk::dpp::prelude::{IdentityPublicKey, ProtocolError};
use simple_signer::SingleKeySigner;

/// C-compatible vtable for signers
#[repr(C)]
pub struct SignerVTable {
    /// Sign function pointer
    pub sign: unsafe extern "C" fn(
        signer: *const std::os::raw::c_void,
        identity_public_key_bytes: *const u8,
        identity_public_key_len: usize,
        data: *const u8,
        data_len: usize,
        result_len: *mut usize,
    ) -> *mut u8,

    /// Can sign with function pointer
    pub can_sign_with: unsafe extern "C" fn(
        signer: *const std::os::raw::c_void,
        identity_public_key_bytes: *const u8,
        identity_public_key_len: usize,
    ) -> bool,

    /// Destructor function pointer
    pub destroy: unsafe extern "C" fn(signer: *mut std::os::raw::c_void),
}

/// Generic signer that uses vtable for dynamic dispatch
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VTableSigner {
    /// Pointer to the actual signer implementation
    pub signer_ptr: *mut std::os::raw::c_void,
    /// Pointer to the vtable
    pub vtable: *const SignerVTable,
}

// SAFETY: VTableSigner can be sent between threads because:
// 1. The vtable is immutable (static)
// 2. The actual signer implementations must handle their own thread safety
unsafe impl Send for VTableSigner {}

// SAFETY: VTableSigner can be shared between threads because:
// 1. The vtable functions are thread-safe (they take immutable references)
// 2. The actual signer implementations must handle their own thread safety
unsafe impl Sync for VTableSigner {}

impl std::fmt::Debug for VTableSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VTableSigner")
            .field("signer_ptr", &self.signer_ptr)
            .field("vtable", &self.vtable)
            .finish()
    }
}

impl Signer for VTableSigner {
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        unsafe {
            // Serialize the public key
            let key_bytes =
                bincode::encode_to_vec(identity_public_key, bincode::config::standard())
                    .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

            let mut result_len: usize = 0;
            let result_ptr = ((*self.vtable).sign)(
                self.signer_ptr,
                key_bytes.as_ptr(),
                key_bytes.len(),
                data.as_ptr(),
                data.len(),
                &mut result_len,
            );

            if result_ptr.is_null() {
                return Err(ProtocolError::Generic("Signing failed".to_string()));
            }

            // Convert result to BinaryData
            let signature = std::slice::from_raw_parts(result_ptr, result_len).to_vec();

            // Free the result using the same allocator
            dash_sdk_bytes_free(result_ptr);

            Ok(BinaryData::from(signature))
        }
    }

    fn can_sign_with(&self, identity_public_key: &IdentityPublicKey) -> bool {
        unsafe {
            // Serialize the public key
            match bincode::encode_to_vec(identity_public_key, bincode::config::standard()) {
                Ok(key_bytes) => ((*self.vtable).can_sign_with)(
                    self.signer_ptr,
                    key_bytes.as_ptr(),
                    key_bytes.len(),
                ),
                Err(_) => false,
            }
        }
    }
}

/// Function pointer type for signing callback from iOS/external code
/// Returns pointer to allocated byte array (caller must free with dash_sdk_bytes_free)
/// Returns null on error
pub type SignCallback = unsafe extern "C" fn(
    signer: *const std::os::raw::c_void,
    identity_public_key_bytes: *const u8,
    identity_public_key_len: usize,
    data: *const u8,
    data_len: usize,
    result_len: *mut usize,
) -> *mut u8;

/// Function pointer type for can_sign_with callback from iOS/external code
pub type CanSignCallback = unsafe extern "C" fn(
    signer: *const std::os::raw::c_void,
    identity_public_key_bytes: *const u8,
    identity_public_key_len: usize,
) -> bool;

/// Function pointer type for destructor callback  
/// This is an Option to allow for NULL pointers from C
pub type DestroyCallback = Option<unsafe extern "C" fn(signer: *mut std::os::raw::c_void)>;

/// Create a new signer with callbacks from iOS/external code
///
/// This creates a VTableSigner that can be used for all state transitions.
/// The callbacks should handle the actual signing logic.
///
/// # Parameters
/// - `sign_callback`: Function to sign data
/// - `can_sign_callback`: Function to check if can sign with a key
/// - `destroy_callback`: Optional destructor (can be NULL)
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signer_create(
    sign_callback: SignCallback,
    can_sign_callback: CanSignCallback,
    destroy_callback: DestroyCallback, // Option type handles NULL automatically
) -> *mut SignerHandle {
    // Create a vtable on the heap so it persists
    let vtable = Box::new(SignerVTable {
        sign: sign_callback,
        can_sign_with: can_sign_callback,
        destroy: destroy_callback.unwrap_or(default_destroy),
    });

    let vtable_ptr = Box::into_raw(vtable);

    // Create the VTableSigner
    let vtable_signer = VTableSigner {
        signer_ptr: std::ptr::null_mut(), // iOS doesn't need a separate signer_ptr since callbacks handle everything
        vtable: vtable_ptr,
    };

    Box::into_raw(Box::new(vtable_signer)) as *mut SignerHandle
}

/// Default destroy function that does nothing
unsafe extern "C" fn default_destroy(_signer: *mut std::os::raw::c_void) {
    // No-op for iOS signers that don't need cleanup
}

/// Destroy a signer
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signer_destroy(handle: *mut SignerHandle) {
    if !handle.is_null() {
        let vtable_signer = Box::from_raw(handle as *mut VTableSigner);

        // Call the destructor through the vtable
        if !vtable_signer.vtable.is_null() {
            ((*vtable_signer.vtable).destroy)(vtable_signer.signer_ptr);

            // Only free the vtable if it's not a static vtable
            // Static vtables (like SINGLE_KEY_SIGNER_VTABLE) should not be freed
            // We can check if it's the static vtable by comparing the address
            let static_vtable_ptr = &SINGLE_KEY_SIGNER_VTABLE as *const SignerVTable;
            if vtable_signer.vtable != static_vtable_ptr {
                // This is a heap-allocated vtable from dash_sdk_signer_create
                let _ = Box::from_raw(vtable_signer.vtable as *mut SignerVTable);
            }
        }

        // The VTableSigner itself is dropped here
    }
}

/// Free bytes allocated by callbacks
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_bytes_free(bytes: *mut u8) {
    if !bytes.is_null() {
        // Note: This assumes iOS/external code allocates with malloc/calloc
        // If a different allocator is used, this function needs to be updated
        libc::free(bytes as *mut libc::c_void);
    }
}

// Vtable implementation for SingleKeySigner
unsafe extern "C" fn single_key_signer_sign(
    signer: *const std::os::raw::c_void,
    identity_public_key_bytes: *const u8,
    identity_public_key_len: usize,
    data: *const u8,
    data_len: usize,
    result_len: *mut usize,
) -> *mut u8 {
    let signer = &*(signer as *const SingleKeySigner);

    // Deserialize the public key
    let key_bytes = std::slice::from_raw_parts(identity_public_key_bytes, identity_public_key_len);
    let identity_public_key = match bincode::decode_from_slice::<IdentityPublicKey, _>(
        key_bytes,
        bincode::config::standard(),
    ) {
        Ok((key, _)) => key,
        Err(_) => return std::ptr::null_mut(),
    };

    let data_slice = std::slice::from_raw_parts(data, data_len);

    match signer.sign(&identity_public_key, data_slice) {
        Ok(signature) => {
            let sig_vec = signature.to_vec();
            *result_len = sig_vec.len();
            let result_ptr = libc::malloc(sig_vec.len()) as *mut u8;
            if !result_ptr.is_null() {
                std::ptr::copy_nonoverlapping(sig_vec.as_ptr(), result_ptr, sig_vec.len());
            }
            result_ptr
        }
        Err(_) => std::ptr::null_mut(),
    }
}

unsafe extern "C" fn single_key_signer_can_sign_with(
    signer: *const std::os::raw::c_void,
    identity_public_key_bytes: *const u8,
    identity_public_key_len: usize,
) -> bool {
    let signer = &*(signer as *const SingleKeySigner);

    // Deserialize the public key
    let key_bytes = std::slice::from_raw_parts(identity_public_key_bytes, identity_public_key_len);
    match bincode::decode_from_slice::<IdentityPublicKey, _>(key_bytes, bincode::config::standard())
    {
        Ok((identity_public_key, _)) => signer.can_sign_with(&identity_public_key),
        Err(_) => false,
    }
}

unsafe extern "C" fn single_key_signer_destroy(signer: *mut std::os::raw::c_void) {
    if !signer.is_null() {
        let _ = Box::from_raw(signer as *mut SingleKeySigner);
    }
}

/// Static vtable for SingleKeySigner
pub static SINGLE_KEY_SIGNER_VTABLE: SignerVTable = SignerVTable {
    sign: single_key_signer_sign,
    can_sign_with: single_key_signer_can_sign_with,
    destroy: single_key_signer_destroy,
};
