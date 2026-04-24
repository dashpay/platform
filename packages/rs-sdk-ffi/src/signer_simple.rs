//! Simple private key signer for iOS FFI

use crate::signer::{signer_handle_from_single_key, VTableSigner};
use crate::types::SignerHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
use dash_sdk::dpp::dashcore::Network;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use simple_signer::SingleKeySigner;
use zeroize::Zeroizing;

// Use the workspace-canonical async-sync bridge from `dash-async` instead of
// spinning up an ad-hoc tokio runtime. `block_on` handles no-runtime,
// current-thread, and multi-thread flavors correctly (see PR #3497/#3432).
use dash_async::block_on;

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

    // Copy the input data into an owned buffer so the signing future has no
    // borrowed references and can satisfy `dash_async::block_on`'s
    // `Send + 'static` bounds.
    let data_owned: Vec<u8> = std::slice::from_raw_parts(data, data_len).to_vec();

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

    // Bridge the async Signer API into this synchronous FFI entry point via
    // the workspace's canonical async-sync bridge (`dash_async::block_on`),
    // which is runtime-aware and safe regardless of whether a tokio runtime
    // is already active. `block_on` requires `Send + 'static` futures, so
    // we hand ownership of the key and data buffer into the future and
    // reconstruct the `&VTableSigner` from a pointer-sized integer inside
    // the future body.
    //
    // SAFETY: `signer_handle` is a valid `*const VTableSigner` for the
    // duration of this FFI call, and `block_on` blocks the caller until the
    // future completes, so the underlying allocation cannot be freed by the
    // C caller while we hold the reference. `VTableSigner` is `Send + Sync`.
    let signer_addr = signer_handle as usize;
    let result = block_on(async move {
        let signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
        signer.sign(&dummy_key, &data_owned).await
    });

    match result {
        Ok(Ok(signature)) => {
            let sig_vec = signature.to_vec();
            let sig_len = sig_vec.len();
            let sig_ptr = sig_vec.leak().as_mut_ptr();

            let boxed = Box::new(DashSDKSignature {
                signature: sig_ptr,
                signature_len: sig_len,
            });

            DashSDKResult::success(Box::into_raw(boxed) as *mut std::os::raw::c_void)
        }
        Ok(Err(e)) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::CryptoError,
            format!("Failed to sign: {}", e),
        )),
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("Async bridge failed during signing: {}", e),
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
