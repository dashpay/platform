//! Signer interface for iOS FFI

use crate::types::SignerHandle;
use crate::signer_simple;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::platform_value::BinaryData;
use dash_sdk::dpp::prelude::{IdentityPublicKey, ProtocolError};
use simple_signer::SingleKeySigner;

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
    
    // Create a VTableSigner that wraps the IOSSigner
    let vtable_signer = VTableSigner {
        signer_ptr: Box::into_raw(Box::new(signer)) as *mut std::os::raw::c_void,
        vtable: &IOS_SIGNER_VTABLE,
    };
    
    Box::into_raw(Box::new(vtable_signer)) as *mut SignerHandle
}

/// Destroy a signer
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signer_destroy(handle: *mut SignerHandle) {
    if !handle.is_null() {
        // Try to cast as VTableSigner first
        let vtable_signer = Box::from_raw(handle as *mut VTableSigner);
        
        // Call the destructor through the vtable
        if !vtable_signer.vtable.is_null() {
            ((*vtable_signer.vtable).destroy)(vtable_signer.signer_ptr);
        }
        
        // The VTableSigner itself is dropped here
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
            let key_bytes = bincode::encode_to_vec(identity_public_key, bincode::config::standard())
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
                Ok(key_bytes) => {
                    ((*self.vtable).can_sign_with)(
                        self.signer_ptr,
                        key_bytes.as_ptr(),
                        key_bytes.len(),
                    )
                }
                Err(_) => false,
            }
        }
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
    let identity_public_key = match bincode::decode_from_slice::<IdentityPublicKey, _>(key_bytes, bincode::config::standard()) {
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
    match bincode::decode_from_slice::<IdentityPublicKey, _>(key_bytes, bincode::config::standard()) {
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

// Vtable implementation for IOSSigner
unsafe extern "C" fn ios_signer_sign(
    signer: *const std::os::raw::c_void,
    identity_public_key_bytes: *const u8,
    identity_public_key_len: usize,
    data: *const u8,
    data_len: usize,
    result_len: *mut usize,
) -> *mut u8 {
    let signer = &*(signer as *const IOSSigner);
    
    // Deserialize the public key
    let key_bytes = std::slice::from_raw_parts(identity_public_key_bytes, identity_public_key_len);
    let identity_public_key = match bincode::decode_from_slice::<IdentityPublicKey, _>(key_bytes, bincode::config::standard()) {
        Ok((key, _)) => key,
        Err(_) => return std::ptr::null_mut(),
    };
    
    let data_slice = std::slice::from_raw_parts(data, data_len);
    
    match signer.sign(&identity_public_key, data_slice) {
        Ok(signature) => {
            let sig_vec = signature.to_vec();
            *result_len = sig_vec.len();
            // IOSSigner already returns malloc'd memory, so we use its callback directly
            (signer.sign_callback)(
                identity_public_key_bytes,
                identity_public_key_len,
                data,
                data_len,
                result_len,
            )
        }
        Err(_) => std::ptr::null_mut(),
    }
}

unsafe extern "C" fn ios_signer_can_sign_with(
    signer: *const std::os::raw::c_void,
    identity_public_key_bytes: *const u8,
    identity_public_key_len: usize,
) -> bool {
    let signer = &*(signer as *const IOSSigner);
    (signer.can_sign_callback)(identity_public_key_bytes, identity_public_key_len)
}

unsafe extern "C" fn ios_signer_destroy(signer: *mut std::os::raw::c_void) {
    if !signer.is_null() {
        let _ = Box::from_raw(signer as *mut IOSSigner);
    }
}

/// Static vtable for IOSSigner
pub static IOS_SIGNER_VTABLE: SignerVTable = SignerVTable {
    sign: ios_signer_sign,
    can_sign_with: ios_signer_can_sign_with,
    destroy: ios_signer_destroy,
};
