//! Test module to diagnose transfer crash

use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::{KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, Identity};
use dash_sdk::platform::Fetch;
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Test function to diagnose the transfer crash
///
/// # Safety
/// - `sdk_handle` and `identity_id` must be valid, non-null pointers.
/// - `identity_id` must point to a NUL-terminated C string valid for the duration of the call.
/// - On success, any heap memory in the result must be freed using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_test_identity_transfer_crash(
    sdk_handle: *const SDKHandle,
    identity_id: *const c_char,
) -> DashSDKResult {
    eprintln!("🔵 dash_sdk_test_identity_transfer_crash: Starting test");

    if sdk_handle.is_null() || identity_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or identity ID is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let id_str = match CStr::from_ptr(identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let id = match Identifier::from_string(id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ));
        }
    };

    // Fetch the identity
    let identity = match wrapper.runtime.block_on(Identity::fetch(&wrapper.sdk, id)) {
        Ok(Some(identity)) => identity,
        Ok(None) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::NotFound,
                "Identity not found".to_string(),
            ));
        }
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    eprintln!("🔵 Test: Identity fetched successfully");
    eprintln!("🔵 Test: Identity balance: {}", identity.balance());
    eprintln!(
        "🔵 Test: Number of public keys: {}",
        identity.public_keys().len()
    );

    // Try to manually call get_first_public_key_matching
    eprintln!("🔵 Test: Attempting to call get_first_public_key_matching...");

    let mut security_levels = HashSet::new();
    security_levels.insert(SecurityLevel::CRITICAL);
    security_levels.insert(SecurityLevel::HIGH);
    security_levels.insert(SecurityLevel::MEDIUM);

    let mut key_types = HashSet::new();
    key_types.insert(KeyType::ECDSA_SECP256K1);
    key_types.insert(KeyType::BLS12_381);
    key_types.insert(KeyType::ECDSA_HASH160);
    key_types.insert(KeyType::BIP13_SCRIPT_HASH);
    key_types.insert(KeyType::EDDSA_25519_HASH160);

    // Note: Previous versions wrapped this in catch_unwind, which was removed because
    // with panic = "abort" in the release profile, catch_unwind is a no-op and would
    // never actually catch a panic. The identity is locally owned here (fetched above),
    // so this call should not panic.
    let key =
        identity.get_first_public_key_matching(Purpose::TRANSFER, security_levels, key_types, true);

    match key {
        Some(k) => eprintln!("🔵 Test: Found transfer key with ID: {}", k.id()),
        None => eprintln!("⚠️ Test: No transfer key found"),
    }

    eprintln!("🔵 Test: get_first_public_key_matching completed successfully");

    // If we get here, the method works fine
    eprintln!("✅ Test: All tests passed, no crash detected");

    DashSDKResult::success(std::ptr::null_mut())
}
