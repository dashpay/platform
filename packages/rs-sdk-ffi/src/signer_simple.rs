//! Simple private key signer for iOS FFI

use crate::types::SignerHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::platform_value::BinaryData;
use dash_sdk::dpp::prelude::{IdentityPublicKey, ProtocolError};
use ed25519_dalek::{Signature, Signer as _, SigningKey};

/// Simple signer that uses a private key directly
#[derive(Debug, Clone)]
pub struct SimplePrivateKeySigner {
    private_key: [u8; 32],
}

impl SimplePrivateKeySigner {
    pub fn new(private_key: [u8; 32]) -> Self {
        SimplePrivateKeySigner { private_key }
    }
}

impl Signer for SimplePrivateKeySigner {
    fn sign(
        &self,
        _identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let signing_key = SigningKey::from_bytes(&self.private_key);
        let signature: Signature = signing_key.sign(data);
        Ok(signature.to_bytes().to_vec().into())
    }

    fn can_sign_with(&self, _identity_public_key: &IdentityPublicKey) -> bool {
        // This simple signer can sign with any key (assumes the private key matches)
        true
    }
}

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

    let signer = SimplePrivateKeySigner::new(key_array);
    let handle = Box::into_raw(Box::new(signer)) as *mut SignerHandle;
    DashSDKResult::success(handle as *mut std::os::raw::c_void)
}