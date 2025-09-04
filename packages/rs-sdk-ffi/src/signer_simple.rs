//! Simple private key signer for iOS FFI

use crate::types::SignerHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::dpp::dashcore::Network;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use simple_signer::SingleKeySigner;
use std::collections::BTreeMap;

/// Create a signer from a private key
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signer_create_from_private_key(
    private_key: *const u8,
    private_key_len: usize,
) -> DashSDKResult {
    if private_key.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Private key is null".to_string(),
        ));
    }

    if private_key_len != 32 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!("Private key must be 32 bytes, got {}", private_key_len),
        ));
    }

    // Convert the pointer to an array
    let key_slice = std::slice::from_raw_parts(private_key, 32);
    let mut key_array: [u8; 32] = [0; 32];
    key_array.copy_from_slice(key_slice);

    // network won't matter here
    let signer = match SingleKeySigner::new_from_slice(key_array.as_slice(), Network::Dash) {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e));
        }
    };

    // Create a VTableSigner that wraps the SingleKeySigner
    let vtable_signer = crate::signer::VTableSigner {
        signer_ptr: Box::into_raw(Box::new(signer)) as *mut std::os::raw::c_void,
        vtable: &crate::signer::SINGLE_KEY_SIGNER_VTABLE,
    };

    let handle = Box::into_raw(Box::new(vtable_signer)) as *mut SignerHandle;
    DashSDKResult::success(handle as *mut std::os::raw::c_void)
}

/// Signature result structure
#[repr(C)]
pub struct DashSDKSignature {
    pub signature: *mut u8,
    pub signature_len: usize,
}

/// Sign data with a signer
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signer_sign(
    signer_handle: *mut SignerHandle,
    data: *const u8,
    data_len: usize,
) -> DashSDKResult {
    if signer_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Signer handle is null".to_string(),
        ));
    }

    if data.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Data is null".to_string(),
        ));
    }

    // Treat the handle as a VTableSigner and use its Signer impl
    let signer = &*(signer_handle as *const crate::signer::VTableSigner);
    let data_slice = std::slice::from_raw_parts(data, data_len);

    // Create a dummy identity public key for signing
    // The SingleKeySigner doesn't actually use the key data, just needs one to satisfy the trait
    let dummy_key = IdentityPublicKey::V0(
        dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0 {
            id: 0,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            data: vec![0; 33].into(),
            read_only: false,
            disabled_at: None,
            contract_bounds: None,
        },
    );

    match signer.sign(&dummy_key, data_slice) {
        Ok(signature) => {
            let sig_vec = signature.to_vec();
            let sig_len = sig_vec.len();
            let sig_ptr = sig_vec.leak().as_mut_ptr();

            let result = Box::new(DashSDKSignature {
                signature: sig_ptr,
                signature_len: sig_len,
            });

            DashSDKResult::success(Box::into_raw(result) as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::CryptoError,
            format!("Failed to sign: {}", e),
        )),
    }
}

/// Free a signature
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signature_free(signature: *mut DashSDKSignature) {
    if !signature.is_null() {
        let sig = Box::from_raw(signature);
        if !sig.signature.is_null() {
            // Reconstruct the Vec to properly deallocate
            let _ = Vec::from_raw_parts(sig.signature, sig.signature_len, sig.signature_len);
        }
    }
}
