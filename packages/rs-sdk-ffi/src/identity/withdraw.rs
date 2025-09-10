//! Identity withdrawal operations

use dash_sdk::dpp::dashcore::{self, Address};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::prelude::Identity;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::str::FromStr;

use crate::identity::helpers::convert_put_settings;
use crate::sdk::SDKWrapper;
use crate::types::{DashSDKPutSettings, IdentityHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use tracing::{debug, error, info, warn};

/// Withdraw credits from identity to a Dash address
///
/// # Parameters
/// - `identity_handle`: Identity to withdraw credits from
/// - `address`: Base58-encoded Dash address to withdraw to
/// - `amount`: Amount of credits to withdraw
/// - `core_fee_per_byte`: Core fee per byte (optional, pass 0 for default)
/// - `public_key_id`: ID of the public key to use for signing (pass 0 to auto-select TRANSFER key)
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// The new balance of the identity after withdrawal
///
/// # Safety
/// - `sdk_handle`, `identity_handle`, `address`, and `signer_handle` must be valid, non-null pointers.
/// - `address` must point to a NUL-terminated C string valid for the duration of the call.
/// - `put_settings` may be null; if non-null it must be valid for the duration of the call.
/// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_withdraw(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    address: *const c_char,
    amount: u64,
    core_fee_per_byte: u32,
    public_key_id: u32,
    signer_handle: *const crate::types::SignerHandle,
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || address.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    debug!(ptr = ?sdk_handle, "dash_sdk_identity_withdraw: validating handles");
    debug!(ptr = ?identity_handle, "dash_sdk_identity_withdraw: identity_handle");
    debug!(ptr = ?address, "dash_sdk_identity_withdraw: address ptr");
    debug!(ptr = ?signer_handle, "dash_sdk_identity_withdraw: signer_handle");
    debug!(
        amount,
        core_fee_per_byte, public_key_id, "dash_sdk_identity_withdraw: parameters"
    );

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);

    // Carefully validate the identity handle
    debug!("dash_sdk_identity_withdraw: dereferencing identity handle");
    let identity = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        &*(identity_handle as *const Identity)
    })) {
        Ok(identity) => {
            debug!("dash_sdk_identity_withdraw: identity handle dereferenced");
            identity
        }
        Err(_) => {
            error!("dash_sdk_identity_withdraw: failed to dereference identity handle - invalid pointer");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Invalid identity handle - possible use after free".to_string(),
            ));
        }
    };

    let signer = &*(signer_handle as *const crate::signer::VTableSigner);

    debug!("dash_sdk_identity_withdraw: handles dereferenced successfully");

    // Try to access identity fields safely
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        debug!(id = ?identity.id(), balance = identity.balance(), keys = identity.public_keys().len(), "dash_sdk_identity_withdraw: identity summary");
    })) {
        Ok(_) => debug!("dash_sdk_identity_withdraw: identity fields accessed"),
        Err(_) => {
            error!(
                "dash_sdk_identity_withdraw: failed to access identity fields - corrupted identity"
            );
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Identity handle points to corrupted data".to_string(),
            ));
        }
    };

    let address_str = match CStr::from_ptr(address).to_str() {
        Ok(s) => {
            debug!(
                address = s,
                len = s.len(),
                "dash_sdk_identity_withdraw: address"
            );
            s
        }
        Err(e) => {
            error!(error = %e, "dash_sdk_identity_withdraw: failed to convert address C string");
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };

    // Parse the address
    debug!("dash_sdk_identity_withdraw: parsing Dash address");
    let withdraw_address =
        match Address::<dashcore::address::NetworkUnchecked>::from_str(address_str) {
            Ok(addr) => {
                debug!("dash_sdk_identity_withdraw: address parsed successfully");
                addr.assume_checked()
            }
            Err(e) => {
                error!(error = %e, "dash_sdk_identity_withdraw: failed to parse address");
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid Dash address: {}", e),
                ));
            }
        };

    // Get public key if specified (0 means auto-select TRANSFER key)
    debug!("dash_sdk_identity_withdraw: determining signing key");
    let signing_key = if public_key_id == 0 {
        debug!("dash_sdk_identity_withdraw: auto-select key (public_key_id = 0)");
        None
    } else {
        debug!(
            public_key_id,
            "dash_sdk_identity_withdraw: looking for key id"
        );
        match identity.get_public_key_by_id(public_key_id) {
            Some(key) => {
                debug!(found_key_id = public_key_id, purpose = ?key.purpose(), key_type = ?key.key_type(), "dash_sdk_identity_withdraw: found key");
                Some(key)
            }
            None => {
                error!(
                    public_key_id,
                    "dash_sdk_identity_withdraw: key id not found"
                );
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Public key with ID {} not found in identity", public_key_id),
                ));
            }
        }
    };
    debug!("dash_sdk_identity_withdraw: signing key determined");

    // Optional core fee per byte
    let core_fee = if core_fee_per_byte > 0 {
        Some(core_fee_per_byte)
    } else {
        None
    };

    debug!("dash_sdk_identity_withdraw: entering async block");

    // Check for transfer keys before proceeding
    debug!("dash_sdk_identity_withdraw: iterating public keys");
    let mut transfer_key_found = false;
    for (key_id, key) in identity.public_keys() {
        debug!(key_id, purpose = ?key.purpose(), key_type = ?key.key_type(), "dash_sdk_identity_withdraw: found key");
        if key.purpose() == dash_sdk::dpp::identity::Purpose::TRANSFER {
            transfer_key_found = true;
            debug!(key_id, "dash_sdk_identity_withdraw: found TRANSFER key");
        }
    }

    if !transfer_key_found && signing_key.is_none() {
        warn!("dash_sdk_identity_withdraw: no TRANSFER key found and no signing key specified");
    }

    let result: Result<u64, FFIError> = wrapper.runtime.block_on(async {
        debug!("dash_sdk_identity_withdraw: inside async block");

        // Convert settings
        debug!("dash_sdk_identity_withdraw: converting put settings");
        let settings = convert_put_settings(put_settings);
        debug!(has_settings = settings.is_some(), "dash_sdk_identity_withdraw: settings converted");

        // Use Withdraw trait to withdraw credits
        debug!("dash_sdk_identity_withdraw: importing WithdrawFromIdentity trait");
        use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;
        debug!("dash_sdk_identity_withdraw: trait imported");

        debug!(?withdraw_address, amount, ?core_fee, has_signing_key = signing_key.is_some(), signer_ptr = ?(signer as *const _), "dash_sdk_identity_withdraw: calling withdraw method");

        // Additional defensive check on the signing_key if present
        if let Some(key) = signing_key {
            eprintln!("🔵 dash_sdk_identity_withdraw: Signing key details:");
            eprintln!("  - Key ID: {}", key.id());
            eprintln!("  - Purpose: {:?}", key.purpose());
            eprintln!("  - Security level: {:?}", key.security_level());
            eprintln!("  - Key type: {:?}", key.key_type());
            eprintln!("  - Read only: {}", key.read_only());

            // Try to access the key data to see if it crashes here
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _data = key.data();
                debug!(len = key.data().len(), "dash_sdk_identity_withdraw: key data length");
            })) {
                Ok(_) => debug!("dash_sdk_identity_withdraw: key data accessible"),
                Err(_) => warn!("dash_sdk_identity_withdraw: key data access caused panic"),
            }
        }

        debug!("dash_sdk_identity_withdraw: calling SDK withdraw");

        let new_balance = identity
            .withdraw(
                &wrapper.sdk,
                Some(withdraw_address),
                amount,
                core_fee,
                signing_key,
                *signer,
                settings,
            )
            .await
            .map_err(|e| {
                error!(error = %e, "dash_sdk_identity_withdraw: withdraw failed");
                FFIError::InternalError(format!("Failed to withdraw credits: {}", e))
            })?;

        info!(new_balance, "dash_sdk_identity_withdraw: withdrawal successful");

        Ok(new_balance)
    });

    match result {
        Ok(new_balance) => {
            // Return the new balance as a string
            let balance_str = match CString::new(new_balance.to_string()) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult::error(
                        FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
                    )
                }
            };
            DashSDKResult::success_string(balance_str.into_raw())
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}
