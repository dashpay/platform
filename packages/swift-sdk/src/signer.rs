use std::os::raw::c_uchar;

/// Swift-compatible signer interface
///
/// This represents a callback-based signer for iOS/Swift applications.
/// The actual signer implementation will be provided by the iOS app.

/// Type alias for signing callback
pub type SwiftSignCallback = unsafe extern "C" fn(
    identity_public_key_bytes: *const c_uchar,
    identity_public_key_len: usize,
    data: *const c_uchar,
    data_len: usize,
    result_len: *mut usize,
) -> *mut c_uchar;

/// Type alias for can_sign callback
pub type SwiftCanSignCallback = unsafe extern "C" fn(
    identity_public_key_bytes: *const c_uchar,
    identity_public_key_len: usize,
) -> bool;

/// Swift signer configuration
#[repr(C)]
pub struct SwiftDashSigner {
    pub sign_callback: SwiftSignCallback,
    pub can_sign_callback: SwiftCanSignCallback,
}

/// Create a new signer with callbacks
#[no_mangle]
pub extern "C" fn swift_dash_signer_create(
    sign_callback: SwiftSignCallback,
    can_sign_callback: SwiftCanSignCallback,
) -> *mut SwiftDashSigner {
    let signer = Box::new(SwiftDashSigner {
        sign_callback,
        can_sign_callback,
    });

    Box::into_raw(signer)
}

/// Free a signer
#[no_mangle]
pub unsafe extern "C" fn swift_dash_signer_free(signer: *mut SwiftDashSigner) {
    if !signer.is_null() {
        let _ = Box::from_raw(signer);
    }
}

/// Test if a signer can sign with a given key
#[no_mangle]
pub unsafe extern "C" fn swift_dash_signer_can_sign(
    signer: *const SwiftDashSigner,
    identity_public_key_bytes: *const c_uchar,
    identity_public_key_len: usize,
) -> bool {
    if signer.is_null() || identity_public_key_bytes.is_null() {
        return false;
    }

    let signer = &*signer;
    (signer.can_sign_callback)(identity_public_key_bytes, identity_public_key_len)
}

/// Sign data with a signer
#[no_mangle]
pub unsafe extern "C" fn swift_dash_signer_sign(
    signer: *const SwiftDashSigner,
    identity_public_key_bytes: *const c_uchar,
    identity_public_key_len: usize,
    data: *const c_uchar,
    data_len: usize,
    result_len: *mut usize,
) -> *mut c_uchar {
    if signer.is_null()
        || identity_public_key_bytes.is_null()
        || data.is_null()
        || result_len.is_null()
    {
        return std::ptr::null_mut();
    }

    let signer = &*signer;
    (signer.sign_callback)(
        identity_public_key_bytes,
        identity_public_key_len,
        data,
        data_len,
        result_len,
    )
}
