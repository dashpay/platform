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
use dash_sdk::dpp::identity::signer::Signer;

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

    eprintln!("🔵 dash_sdk_identity_withdraw: Validating handles...");
    eprintln!(
        "🔵 dash_sdk_identity_withdraw: sdk_handle = {:p}",
        sdk_handle
    );
    eprintln!(
        "🔵 dash_sdk_identity_withdraw: identity_handle = {:p}",
        identity_handle
    );
    eprintln!("🔵 dash_sdk_identity_withdraw: address = {:p}", address);
    eprintln!(
        "🔵 dash_sdk_identity_withdraw: signer_handle = {:p}",
        signer_handle
    );
    eprintln!("🔵 dash_sdk_identity_withdraw: amount = {}", amount);
    eprintln!(
        "🔵 dash_sdk_identity_withdraw: core_fee_per_byte = {}",
        core_fee_per_byte
    );
    eprintln!(
        "🔵 dash_sdk_identity_withdraw: public_key_id = {}",
        public_key_id
    );

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);

    // Carefully validate the identity handle
    eprintln!("🔵 dash_sdk_identity_withdraw: About to dereference identity handle...");
    let identity = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        &*(identity_handle as *const Identity)
    })) {
        Ok(identity) => {
            eprintln!("🔵 dash_sdk_identity_withdraw: Identity handle dereferenced successfully");
            identity
        }
        Err(_) => {
            eprintln!("❌ dash_sdk_identity_withdraw: Failed to dereference identity handle - invalid pointer");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Invalid identity handle - possible use after free".to_string(),
            ));
        }
    };

    let signer = &*(signer_handle as *const crate::signer::VTableSigner);

    eprintln!("🔵 dash_sdk_identity_withdraw: All handles dereferenced successfully");

    // Try to access identity fields safely
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        eprintln!(
            "🔵 dash_sdk_identity_withdraw: Identity ID = {:?}",
            identity.id()
        );
        eprintln!(
            "🔵 dash_sdk_identity_withdraw: Identity balance = {}",
            identity.balance()
        );
        eprintln!(
            "🔵 dash_sdk_identity_withdraw: Number of public keys = {}",
            identity.public_keys().len()
        );
    })) {
        Ok(_) => eprintln!("🔵 dash_sdk_identity_withdraw: Identity fields accessed successfully"),
        Err(_) => {
            eprintln!("❌ dash_sdk_identity_withdraw: Failed to access identity fields - corrupted identity");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Identity handle points to corrupted data".to_string(),
            ));
        }
    };

    let address_str = match CStr::from_ptr(address).to_str() {
        Ok(s) => {
            eprintln!("🔵 dash_sdk_identity_withdraw: address = '{}'", s);
            eprintln!(
                "🔵 dash_sdk_identity_withdraw: address length = {}",
                s.len()
            );
            // Debug each character
            for (i, ch) in s.chars().enumerate() {
                eprintln!(
                    "🔵 dash_sdk_identity_withdraw: char[{}] = '{}' (U+{:04X})",
                    i, ch, ch as u32
                );
            }
            s
        }
        Err(e) => {
            eprintln!(
                "❌ dash_sdk_identity_withdraw: Failed to convert address C string: {}",
                e
            );
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };

    // Parse the address
    eprintln!("🔵 dash_sdk_identity_withdraw: Parsing Dash address...");
    let withdraw_address =
        match Address::<dashcore::address::NetworkUnchecked>::from_str(address_str) {
            Ok(addr) => {
                eprintln!("🔵 dash_sdk_identity_withdraw: Address parsed successfully");
                addr.assume_checked()
            }
            Err(e) => {
                eprintln!(
                    "❌ dash_sdk_identity_withdraw: Failed to parse address: {}",
                    e
                );
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid Dash address: {}", e),
                ));
            }
        };

    // Get public key if specified (0 means auto-select TRANSFER key)
    eprintln!("🔵 dash_sdk_identity_withdraw: Determining signing key...");
    let signing_key = if public_key_id == 0 {
        eprintln!("🔵 dash_sdk_identity_withdraw: Using auto-select (public_key_id = 0)");
        None
    } else {
        eprintln!(
            "🔵 dash_sdk_identity_withdraw: Looking for key with ID {}",
            public_key_id
        );
        match identity.get_public_key_by_id(public_key_id.into()) {
            Some(key) => {
                eprintln!(
                    "🔵 dash_sdk_identity_withdraw: Found key with ID {}",
                    public_key_id
                );
                eprintln!(
                    "🔵 dash_sdk_identity_withdraw: Key purpose: {:?}",
                    key.purpose()
                );
                eprintln!(
                    "🔵 dash_sdk_identity_withdraw: Key type: {:?}",
                    key.key_type()
                );
                Some(key)
            }
            None => {
                eprintln!(
                    "❌ dash_sdk_identity_withdraw: Key with ID {} not found!",
                    public_key_id
                );
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Public key with ID {} not found in identity", public_key_id),
                ));
            }
        }
    };
    eprintln!("🔵 dash_sdk_identity_withdraw: Signing key determined");

    // Optional core fee per byte
    let core_fee = if core_fee_per_byte > 0 {
        Some(core_fee_per_byte)
    } else {
        None
    };

    eprintln!("🔵 dash_sdk_identity_withdraw: About to enter async block");

    // Check for transfer keys before proceeding
    eprintln!("🔵 dash_sdk_identity_withdraw: Iterating through identity public keys...");
    let mut transfer_key_found = false;
    for (key_id, key) in identity.public_keys() {
        eprintln!(
            "🔵 dash_sdk_identity_withdraw: Found key {}: purpose={:?}, type={:?}",
            key_id,
            key.purpose(),
            key.key_type()
        );
        if key.purpose() == dash_sdk::dpp::identity::Purpose::TRANSFER {
            transfer_key_found = true;
            eprintln!(
                "🔵 dash_sdk_identity_withdraw: Found TRANSFER key with ID {}",
                key_id
            );
        }
    }

    if !transfer_key_found && signing_key.is_none() {
        eprintln!("⚠️ dash_sdk_identity_withdraw: WARNING - No transfer key found and no signing key specified!");
    }

    let result: Result<u64, FFIError> = wrapper.runtime.block_on(async {
        eprintln!("🔵 dash_sdk_identity_withdraw: Inside async block");

        // Convert settings
        eprintln!("🔵 dash_sdk_identity_withdraw: Converting put settings");
        let settings = convert_put_settings(put_settings);
        eprintln!(
            "🔵 dash_sdk_identity_withdraw: Settings converted: {:?}",
            settings.is_some()
        );

        // Use Withdraw trait to withdraw credits
        eprintln!("🔵 dash_sdk_identity_withdraw: Importing WithdrawFromIdentity trait");
        use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;
        eprintln!("🔵 dash_sdk_identity_withdraw: Trait imported");

        eprintln!("🔵 dash_sdk_identity_withdraw: About to call withdraw method");
        eprintln!("🔵 dash_sdk_identity_withdraw: Parameters:");
        eprintln!("  - withdraw_address: {:?}", withdraw_address);
        eprintln!("  - amount: {}", amount);
        eprintln!("  - core_fee: {:?}", core_fee);
        eprintln!("  - signing_key present: {}", signing_key.is_some());
        eprintln!("  - signer: {:p}", signer as *const _);

        // Additional defensive check on the signing_key if present
        if let Some(ref key) = signing_key {
            eprintln!("🔵 dash_sdk_identity_withdraw: Signing key details:");
            eprintln!("  - Key ID: {}", key.id());
            eprintln!("  - Purpose: {:?}", key.purpose());
            eprintln!("  - Security level: {:?}", key.security_level());
            eprintln!("  - Key type: {:?}", key.key_type());
            eprintln!("  - Read only: {}", key.read_only());

            // Try to access the key data to see if it crashes here
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _data = key.data();
                eprintln!("  - Key data length: {} bytes", key.data().len());
            })) {
                Ok(_) => eprintln!("  - Key data is accessible"),
                Err(_) => eprintln!("  ❌ Key data access caused panic!"),
            }
        }

        eprintln!("🔵 dash_sdk_identity_withdraw: About to call SDK's withdraw method");

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
                eprintln!("❌ dash_sdk_identity_withdraw: withdraw failed: {}", e);
                FFIError::InternalError(format!("Failed to withdraw credits: {}", e))
            })?;

        eprintln!(
            "🔵 dash_sdk_identity_withdraw: Withdrawal successful! New balance: {}",
            new_balance
        );

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
