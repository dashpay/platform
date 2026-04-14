//! Simple private key signer for iOS FFI

use crate::signer::{signer_handle_from_single_key, VTableSigner};
use crate::types::SignerHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::dpp::dashcore::Network;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use simple_signer::SingleKeySigner;
use zeroize::Zeroizing;

/// Create a signer from a private key.
///
/// Internally wraps a `SingleKeySigner` as a native (non-callback) FFI
/// signer — there is no C-callback bounce on this path. The returned
/// handle still conforms to `Signer<IdentityPublicKey>` via the unified
/// `VTableSigner` wrapper so it can be passed to every other FFI entry
/// point that takes a `SignerHandle`.
///
/// # Safety
/// - `private_key` must be a valid pointer to at least 32 readable bytes.
/// - The function reads exactly `private_key_len` bytes; it must be 32.
/// - The returned handle inside DashSDKResult must be freed with
///   `dash_sdk_signer_destroy`.
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

    // Copy into a zeroizing buffer so the key material doesn't linger on
    // the stack after we hand it off to SingleKeySigner.
    let key_slice = std::slice::from_raw_parts(private_key, 32);
    let mut key_array = Zeroizing::new([0u8; 32]);
    key_array.copy_from_slice(key_slice);

    // Network doesn't matter for signing purposes.
    let signer = match SingleKeySigner::new_from_slice(key_array.as_slice(), Network::Mainnet) {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e));
        }
    };

    let handle = signer_handle_from_single_key(signer);
    DashSDKResult::success(handle as *mut std::os::raw::c_void)
}

/// Signature result structure.
#[repr(C)]
pub struct DashSDKSignature {
    pub signature: *mut u8,
    pub signature_len: usize,
}

/// Sign data with a signer.
///
/// # Safety
/// - `signer_handle` must be a valid pointer obtained from this SDK and not
///   previously destroyed.
/// - `data` must be a valid pointer to `data_len` readable bytes.
/// - The returned signature pointer inside DashSDKResult must be freed with
///   `dash_sdk_signature_free`.
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

    let signer = &*(signer_handle as *const VTableSigner);
    let data_slice = std::slice::from_raw_parts(data, data_len);

    // Create a dummy identity public key for signing. SingleKeySigner
    // doesn't actually use the key data, just needs one to satisfy the
    // trait contract.
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

    // The signer trait is async. This function is a synchronous C entry
    // point, so we bridge by spinning up a current-thread runtime just
    // long enough to drive the signing future. This is safe because
    // SingleKeySigner's async fn is non-blocking (pure CPU work).
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create runtime for signing: {}", e),
            ));
        }
    };

    match runtime.block_on(signer.sign(&dummy_key, data_slice)) {
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

/// Free a signature.
///
/// # Safety
/// - `signature` must be a valid pointer returned by this SDK, or null for
///   no-op.
/// - After this call the pointer must not be used again.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_signature_free(signature: *mut DashSDKSignature) {
    if !signature.is_null() {
        let sig = Box::from_raw(signature);
        if !sig.signature.is_null() {
            // Reconstruct the Vec to properly deallocate.
            let _ = Vec::from_raw_parts(sig.signature, sig.signature_len, sig.signature_len);
        }
    }
}
