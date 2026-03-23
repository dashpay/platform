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

    /// Optional custom deallocator for sign result buffers.
    ///
    /// When the `sign` callback returns a buffer, this function will be called
    /// to free it. If `None`, `libc::free` is used as a fallback (which requires
    /// the buffer to have been allocated with `malloc`/`calloc`).
    ///
    /// **Swift / Kotlin callers** that allocate with their own allocator (e.g.
    /// `UnsafeMutablePointer<UInt8>.allocate`) **must** supply a matching
    /// deallocator here to avoid undefined behavior.
    pub free_result: Option<unsafe extern "C" fn(data: *mut u8, len: usize)>,
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

impl Signer<IdentityPublicKey> for VTableSigner {
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

            // Convert result to BinaryData (copy before freeing)
            let signature = std::slice::from_raw_parts(result_ptr, result_len).to_vec();

            // Free the result using the vtable's custom deallocator if provided,
            // otherwise fall back to libc::free (which requires malloc-allocated memory).
            if let Some(free_fn) = (*self.vtable).free_result {
                free_fn(result_ptr, result_len);
            } else {
                dash_sdk_bytes_free(result_ptr);
            }

            Ok(BinaryData::from(signature))
        }
    }

    fn sign_create_witness(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<dash_sdk::dpp::address_funds::AddressWitness, ProtocolError> {
        // Sign the data first
        let signature = self.sign(identity_public_key, data)?;
        // Create P2PKH witness from signature (the most common case for single key signers)
        Ok(dash_sdk::dpp::address_funds::AddressWitness::P2pkh { signature })
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

/// Optional custom deallocator for sign result buffers.
/// When provided, this function is called to free the buffer returned by the
/// `sign` callback instead of the default `libc::free`. This is an `Option` so
/// that passing `NULL` from C selects the default behavior.
pub type FreeResultCallback = Option<unsafe extern "C" fn(data: *mut u8, len: usize)>;

/// Create a new signer with callbacks from iOS/external code
///
/// This creates a VTableSigner that can be used for all state transitions.
/// The callbacks should handle the actual signing logic.
///
/// # Parameters
/// - `sign_callback`: Function to sign data
/// - `can_sign_callback`: Function to check if can sign with a key
/// - `destroy_callback`: Optional destructor (can be NULL)
/// - `free_result_callback`: Optional custom deallocator for buffers returned by
///   `sign_callback`. When NULL, `libc::free` is used. Swift/Kotlin callers that
///   allocate with their own allocator **must** supply a matching deallocator.
/// # Safety
/// - Callback function pointers must be valid and follow the required ABI and lifetime for the duration of use.
/// - The returned `SignerHandle` must be destroyed with `dash_sdk_signer_destroy` to avoid leaks.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signer_create(
    sign_callback: SignCallback,
    can_sign_callback: CanSignCallback,
    destroy_callback: DestroyCallback, // Option type handles NULL automatically
    free_result_callback: FreeResultCallback, // Option type handles NULL automatically
) -> *mut SignerHandle {
    // Create a vtable on the heap so it persists
    let vtable = Box::new(SignerVTable {
        sign: sign_callback,
        can_sign_with: can_sign_callback,
        destroy: destroy_callback.unwrap_or(default_destroy),
        free_result: free_result_callback,
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
/// # Safety
/// - `handle` must be a valid pointer previously returned by this SDK and not yet destroyed.
/// - It may be null (no-op). After this call the handle must not be used again.
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

/// Free bytes that were allocated with `malloc`/`calloc`.
///
/// This is the **default** deallocator used when `SignerVTable::free_result` is
/// `None`. If your callback allocates memory with a different allocator (e.g.
/// Swift's `UnsafeMutablePointer.allocate`), supply a custom `free_result`
/// function in the vtable instead of relying on this function.
///
/// # Safety
/// - `bytes` must be a pointer allocated with `malloc` or `calloc`, or null.
/// - It may be null (no-op). After this call the pointer must not be used again.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_bytes_free(bytes: *mut u8) {
    if !bytes.is_null() {
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
///
/// `free_result` is `None` because `single_key_signer_sign` allocates its
/// result buffer with `libc::malloc`, so `libc::free` (the default) is correct.
pub static SINGLE_KEY_SIGNER_VTABLE: SignerVTable = SignerVTable {
    sign: single_key_signer_sign,
    can_sign_with: single_key_signer_can_sign_with,
    destroy: single_key_signer_destroy,
    free_result: None,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Global flag set by the custom free callback during tests.
    static CUSTOM_FREE_CALLED: AtomicBool = AtomicBool::new(false);

    /// Custom deallocator that records that it was called and then delegates to
    /// `libc::free` (valid because the test sign callback allocates with
    /// `libc::malloc`).
    unsafe extern "C" fn test_custom_free(data: *mut u8, _len: usize) {
        CUSTOM_FREE_CALLED.store(true, Ordering::SeqCst);
        libc::free(data as *mut libc::c_void);
    }

    /// Sign callback for tests -- allocates the result with `libc::malloc`.
    unsafe extern "C" fn test_sign(
        _signer: *const std::os::raw::c_void,
        _identity_public_key_bytes: *const u8,
        _identity_public_key_len: usize,
        _data: *const u8,
        _data_len: usize,
        result_len: *mut usize,
    ) -> *mut u8 {
        let sig = [0xABu8; 64];
        *result_len = sig.len();
        let ptr = libc::malloc(sig.len()) as *mut u8;
        if !ptr.is_null() {
            std::ptr::copy_nonoverlapping(sig.as_ptr(), ptr, sig.len());
        }
        ptr
    }

    /// Can-sign callback for tests.
    unsafe extern "C" fn test_can_sign(
        _signer: *const std::os::raw::c_void,
        _identity_public_key_bytes: *const u8,
        _identity_public_key_len: usize,
    ) -> bool {
        true
    }

    /// Destroy callback for tests (no-op).
    unsafe extern "C" fn test_destroy(_signer: *mut std::os::raw::c_void) {}

    #[test]
    fn custom_free_result_callback_is_invoked() {
        // Reset global flag
        CUSTOM_FREE_CALLED.store(false, Ordering::SeqCst);

        let vtable = SignerVTable {
            sign: test_sign,
            can_sign_with: test_can_sign,
            destroy: test_destroy,
            free_result: Some(test_custom_free),
        };

        let signer = VTableSigner {
            signer_ptr: std::ptr::null_mut(),
            vtable: &vtable,
        };

        // Build a minimal identity public key to satisfy the Signer trait.
        use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dash_sdk::dpp::identity::{KeyType, Purpose, SecurityLevel};
        use dash_sdk::dpp::platform_value::BinaryData;

        let ipk = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            disabled_at: None,
            contract_bounds: None,
        });

        let result = signer.sign(&ipk, &[1, 2, 3]);
        assert!(result.is_ok(), "sign should succeed");
        assert_eq!(result.unwrap().len(), 64);
        assert!(
            CUSTOM_FREE_CALLED.load(Ordering::SeqCst),
            "custom free_result callback must have been invoked"
        );
    }

    #[test]
    fn default_free_result_when_none() {
        // When free_result is None, VTableSigner::sign should fall back to
        // dash_sdk_bytes_free (libc::free) without crashing.
        let vtable = SignerVTable {
            sign: test_sign,
            can_sign_with: test_can_sign,
            destroy: test_destroy,
            free_result: None,
        };

        let signer = VTableSigner {
            signer_ptr: std::ptr::null_mut(),
            vtable: &vtable,
        };

        use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dash_sdk::dpp::identity::{KeyType, Purpose, SecurityLevel};
        use dash_sdk::dpp::platform_value::BinaryData;

        let ipk = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![0u8; 33]),
            disabled_at: None,
            contract_bounds: None,
        });

        let result = signer.sign(&ipk, &[4, 5, 6]);
        assert!(result.is_ok(), "sign with None free_result should succeed");
        assert_eq!(result.unwrap().len(), 64);
    }
}
