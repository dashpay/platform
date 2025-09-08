//! Identity credit transfer operations

use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, Identity};
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::identity::helpers::convert_put_settings;
use crate::sdk::SDKWrapper;
use crate::types::{DashSDKPutSettings, IdentityHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Result structure for credit transfer operations
#[repr(C)]
pub struct DashSDKTransferCreditsResult {
    /// Sender's final balance after transfer
    pub sender_balance: u64,
    /// Receiver's final balance after transfer
    pub receiver_balance: u64,
}

/// Transfer credits from one identity to another
///
/// # Parameters
/// - `from_identity_handle`: Identity to transfer credits from
/// - `to_identity_id`: Base58-encoded ID of the identity to transfer credits to
/// - `amount`: Amount of credits to transfer
/// - `public_key_id`: ID of the public key to use for signing (pass 0 to auto-select TRANSFER key)
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// DashSDKTransferCreditsResult with sender and receiver final balances on success
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_transfer_credits(
    sdk_handle: *mut SDKHandle,
    from_identity_handle: *const IdentityHandle,
    to_identity_id: *const c_char,
    amount: u64,
    public_key_id: u32,
    signer_handle: *const crate::types::SignerHandle,
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || from_identity_handle.is_null()
        || to_identity_id.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    eprintln!("🔵 dash_sdk_identity_transfer_credits: Validating handles...");
    eprintln!(
        "🔵 dash_sdk_identity_transfer_credits: sdk_handle = {:p}",
        sdk_handle
    );
    eprintln!(
        "🔵 dash_sdk_identity_transfer_credits: from_identity_handle = {:p}",
        from_identity_handle
    );
    eprintln!(
        "🔵 dash_sdk_identity_transfer_credits: signer_handle = {:p}",
        signer_handle
    );

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);

    // Carefully validate the identity handle
    eprintln!("🔵 dash_sdk_identity_transfer_credits: About to dereference identity handle...");
    let from_identity = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        &*(from_identity_handle as *const Identity)
    })) {
        Ok(identity) => {
            eprintln!(
                "🔵 dash_sdk_identity_transfer_credits: Identity handle dereferenced successfully"
            );
            identity
        }
        Err(_) => {
            eprintln!("❌ dash_sdk_identity_transfer_credits: Failed to dereference identity handle - invalid pointer");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Invalid identity handle - possible use after free".to_string(),
            ));
        }
    };

    let signer = &*(signer_handle as *const crate::signer::VTableSigner);

    eprintln!("🔵 dash_sdk_identity_transfer_credits: All handles dereferenced successfully");
    eprintln!(
        "🔵 dash_sdk_identity_transfer_credits: public_key_id = {}",
        public_key_id
    );

    // Try to access identity fields safely
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        eprintln!(
            "🔵 dash_sdk_identity_transfer_credits: Identity ID = {:?}",
            from_identity.id()
        );
        eprintln!(
            "🔵 dash_sdk_identity_transfer_credits: Identity balance = {}",
            from_identity.balance()
        );
    })) {
        Ok(_) => eprintln!(
            "🔵 dash_sdk_identity_transfer_credits: Identity fields accessed successfully"
        ),
        Err(_) => {
            eprintln!("❌ dash_sdk_identity_transfer_credits: Failed to access identity fields - corrupted identity");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Identity handle points to corrupted data".to_string(),
            ));
        }
    };

    let to_identity_id_str = match CStr::from_ptr(to_identity_id).to_str() {
        Ok(s) => {
            eprintln!(
                "🔵 dash_sdk_identity_transfer_credits: to_identity_id = '{}'",
                s
            );
            eprintln!(
                "🔵 dash_sdk_identity_transfer_credits: to_identity_id length = {}",
                s.len()
            );
            // Debug each character
            for (i, ch) in s.chars().enumerate() {
                eprintln!(
                    "🔵 dash_sdk_identity_transfer_credits: char[{}] = '{}' (U+{:04X})",
                    i, ch, ch as u32
                );
            }
            s
        }
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let to_id = match Identifier::from_string(to_identity_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            eprintln!(
                "❌ dash_sdk_identity_transfer_credits: Failed to parse to_identity_id: {}",
                e
            );
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid to_identity_id: {}", e),
            ));
        }
    };

    // Get public key if specified (0 means auto-select TRANSFER key)
    eprintln!("🔵 dash_sdk_identity_transfer_credits: Determining signing key...");
    let signing_key = if public_key_id == 0 {
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Using auto-select (public_key_id = 0)");
        None
    } else {
        eprintln!(
            "🔵 dash_sdk_identity_transfer_credits: Looking for key with ID {}",
            public_key_id
        );
        match from_identity.get_public_key_by_id(public_key_id) {
            Some(key) => {
                eprintln!(
                    "🔵 dash_sdk_identity_transfer_credits: Found key with ID {}",
                    public_key_id
                );
                eprintln!(
                    "🔵 dash_sdk_identity_transfer_credits: Key purpose: {:?}",
                    key.purpose()
                );
                eprintln!(
                    "🔵 dash_sdk_identity_transfer_credits: Key type: {:?}",
                    key.key_type()
                );
                Some(key)
            }
            None => {
                eprintln!(
                    "❌ dash_sdk_identity_transfer_credits: Key with ID {} not found!",
                    public_key_id
                );
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Public key with ID {} not found in identity", public_key_id),
                ));
            }
        }
    };
    eprintln!("🔵 dash_sdk_identity_transfer_credits: Signing key determined");

    eprintln!("🔵 dash_sdk_identity_transfer_credits: About to enter async block");

    let result: Result<DashSDKTransferCreditsResult, FFIError> = wrapper.runtime.block_on(async {
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Inside async block");
        // Convert settings
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Converting put settings");
        let settings = convert_put_settings(put_settings);
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Settings converted: {:?}", settings.is_some());

        // Use TransferToIdentity trait to transfer credits
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Importing TransferToIdentity trait");
        use dash_sdk::platform::transition::transfer::TransferToIdentity;
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Trait imported");

        eprintln!("🔵 dash_sdk_identity_transfer_credits: About to call transfer_credits method");
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Parameters:");
        eprintln!("  - to_id: {:?}", to_id);
        eprintln!("  - amount: {}", amount);
        eprintln!("  - signing_key present: {}", signing_key.is_some());
        eprintln!("  - signer: {:p}", signer as *const _);

        // Additional defensive checks before calling transfer_credits
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Performing defensive checks...");

        // Check if we can iterate through public keys
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Iterating through identity public keys...");
        let mut transfer_key_found = false;
        for (key_id, key) in from_identity.public_keys() {
            eprintln!("🔵 dash_sdk_identity_transfer_credits: Found key {}: purpose={:?}", key_id, key.purpose());
            if key.purpose() == dash_sdk::dpp::identity::Purpose::TRANSFER {
                transfer_key_found = true;
                eprintln!("🔵 dash_sdk_identity_transfer_credits: Found TRANSFER key with ID {}", key_id);
            }
        }

        if !transfer_key_found && signing_key.is_none() {
            eprintln!("⚠️ dash_sdk_identity_transfer_credits: WARNING - No transfer key found and no signing key specified!");
        }

        eprintln!("🔵 dash_sdk_identity_transfer_credits: Defensive checks complete");

        // Additional check on the signing_key if present
        if let Some(key) = signing_key {
            eprintln!("🔵 dash_sdk_identity_transfer_credits: Signing key details:");
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

        eprintln!("🔵 dash_sdk_identity_transfer_credits: About to call SDK's transfer_credits method");
        eprintln!("🔵 dash_sdk_identity_transfer_credits: This will internally call IdentityCreditTransferTransition::try_from_identity");

        let transfer_result = from_identity
            .transfer_credits(&wrapper.sdk, to_id, amount, signing_key, *signer, settings)
            .await;

        eprintln!("🔵 dash_sdk_identity_transfer_credits: transfer_credits returned: {:?}", transfer_result.is_ok());

        let (sender_balance, receiver_balance) = transfer_result
            .map_err(|e| {
                eprintln!("❌ dash_sdk_identity_transfer_credits: transfer_credits failed: {}", e);
                FFIError::InternalError(format!("Failed to transfer credits: {}", e))
            })?;

        eprintln!("🔵 dash_sdk_identity_transfer_credits: Transfer successful!");
        eprintln!("  - sender_balance: {}", sender_balance);
        eprintln!("  - receiver_balance: {}", receiver_balance);

        Ok(DashSDKTransferCreditsResult {
            sender_balance,
            receiver_balance,
        })
    });

    match result {
        Ok(transfer_result) => {
            let result_ptr = Box::into_raw(Box::new(transfer_result));
            DashSDKResult::success(result_ptr as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Free a transfer credits result structure
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_transfer_credits_result_free(
    result: *mut DashSDKTransferCreditsResult,
) {
    if !result.is_null() {
        let _ = Box::from_raw(result);
    }
}
