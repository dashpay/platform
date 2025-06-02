//! Common types used across the FFI boundary

use std::os::raw::{c_char, c_void};

/// Opaque handle to an SDK instance
pub struct SDKHandle {
    _private: [u8; 0],
}

/// Opaque handle to an Identity
pub struct IdentityHandle {
    _private: [u8; 0],
}

/// Opaque handle to a Document
pub struct DocumentHandle {
    _private: [u8; 0],
}

/// Opaque handle to a DataContract
pub struct DataContractHandle {
    _private: [u8; 0],
}

/// Opaque handle to a Signer
pub struct SignerHandle {
    _private: [u8; 0],
}

/// Opaque handle to an IdentityPublicKey
pub struct IdentityPublicKeyHandle {
    _private: [u8; 0],
}

/// Network type for SDK configuration
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IOSSDKNetwork {
    /// Mainnet
    Mainnet = 0,
    /// Testnet
    Testnet = 1,
    /// Devnet
    Devnet = 2,
    /// Local development network
    Local = 3,
}

/// SDK configuration
#[repr(C)]
pub struct IOSSDKConfig {
    /// Network to connect to
    pub network: IOSSDKNetwork,
    /// Skip asset lock proof verification (for testing)
    pub skip_asset_lock_proof_verification: bool,
    /// Number of retries for failed requests
    pub request_retry_count: u32,
    /// Timeout for requests in milliseconds
    pub request_timeout_ms: u64,
}

/// Result type for FFI functions that return data
#[repr(C)]
pub struct IOSSDKResult {
    /// Pointer to the result data (null on error)
    pub data: *mut c_void,
    /// Error information (null on success)
    pub error: *mut super::IOSSDKError,
}

impl IOSSDKResult {
    /// Create a success result
    pub fn success(data: *mut c_void) -> Self {
        IOSSDKResult {
            data,
            error: std::ptr::null_mut(),
        }
    }

    /// Create an error result
    pub fn error(error: super::IOSSDKError) -> Self {
        IOSSDKResult {
            data: std::ptr::null_mut(),
            error: Box::into_raw(Box::new(error)),
        }
    }
}

/// Identity information
#[repr(C)]
pub struct IOSSDKIdentityInfo {
    /// Identity ID as hex string (null-terminated)
    pub id: *mut c_char,
    /// Balance in credits
    pub balance: u64,
    /// Revision number
    pub revision: u64,
    /// Public keys count
    pub public_keys_count: u32,
}

/// Document information
#[repr(C)]
pub struct IOSSDKDocumentInfo {
    /// Document ID as hex string (null-terminated)
    pub id: *mut c_char,
    /// Owner ID as hex string (null-terminated)
    pub owner_id: *mut c_char,
    /// Data contract ID as hex string (null-terminated)
    pub data_contract_id: *mut c_char,
    /// Document type (null-terminated)
    pub document_type: *mut c_char,
    /// Revision number
    pub revision: u64,
    /// Created at timestamp (milliseconds since epoch)
    pub created_at: i64,
    /// Updated at timestamp (milliseconds since epoch)
    pub updated_at: i64,
}

/// Free a string allocated by the FFI
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_string_free(s: *mut c_char) {
    if !s.is_null() {
        let _ = std::ffi::CString::from_raw(s);
    }
}

/// Free an identity info structure
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_info_free(info: *mut IOSSDKIdentityInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);
    ios_sdk_string_free(info.id);
}

/// Free a document info structure
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_document_info_free(info: *mut IOSSDKDocumentInfo) {
    if info.is_null() {
        return;
    }

    let info = Box::from_raw(info);
    ios_sdk_string_free(info.id);
    ios_sdk_string_free(info.owner_id);
    ios_sdk_string_free(info.data_contract_id);
    ios_sdk_string_free(info.document_type);
}
