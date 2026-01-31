//! Address credit withdrawal state transition
//!
//! This module provides FFI functions to withdraw credits from Platform addresses to Core (L1) addresses.

use dash_sdk::dpp::address_funds::{
    AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress,
};
use dash_sdk::dpp::dashcore::address::NetworkUnchecked;
use dash_sdk::dpp::dashcore::secp256k1::SecretKey;
use dash_sdk::dpp::dashcore::PrivateKey;
use dash_sdk::dpp::dashcore::{Address, Network};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::identity::core_script::CoreScript;
use dash_sdk::dpp::withdrawal::Pooling;
use dash_sdk::platform::transition::address_credit_withdrawal::WithdrawAddressFunds;
use std::collections::BTreeMap;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::panic::{self, AssertUnwindSafe};
use std::str::FromStr;

use super::transfer::AddressSigner;
use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKAddressInfoEntry, DashSDKAddressInfoMap, DashSDKAddressTransferInput, DashSDKPooling,
    SDKHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

impl From<DashSDKPooling> for Pooling {
    fn from(p: DashSDKPooling) -> Self {
        match p {
            DashSDKPooling::Never => Pooling::Never,
            DashSDKPooling::IfAvailable => Pooling::IfAvailable,
            DashSDKPooling::Standard => Pooling::Standard,
        }
    }
}

/// Withdraw credits from Platform addresses to a Core (L1) address
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `inputs`: Array of input addresses with amounts and private keys
/// - `inputs_count`: Number of input entries
/// - `core_address`: Base58-encoded Dash Core address to withdraw to (null-terminated C string)
/// - `core_fee_per_byte`: Core network fee per byte (0 means use default)
/// - `pooling`: Pooling strategy for the withdrawal
/// - `fee_from_input_index`: Which input index to deduct fees from (0-based, max 65535)
/// - `change_address`: Optional change address (Platform address bytes, null if not used)
/// - `change_address_len`: Length of change address bytes (0 if not used)
///
/// # Returns
/// DashSDKResult with data_type = AddressInfoMap containing updated address balances
///
/// # Safety
/// - All pointers must be valid and non-null (except change_address which can be null).
/// - Arrays must contain at least the specified count of elements.
/// - Private keys must be exactly 32 bytes.
/// - `core_address` must be a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_withdraw_funds(
    sdk_handle: *const SDKHandle,
    inputs: *const DashSDKAddressTransferInput,
    inputs_count: usize,
    core_address: *const c_char,
    core_fee_per_byte: u32,
    pooling: DashSDKPooling,
    fee_from_input_index: u16,
    change_address: *const u8,
    change_address_len: usize,
) -> DashSDKResult {
    // Wrap in catch_unwind for panic safety
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        dash_sdk_address_withdraw_funds_inner(
            sdk_handle,
            inputs,
            inputs_count,
            core_address,
            core_fee_per_byte,
            pooling,
            fee_from_input_index,
            change_address,
            change_address_len,
        )
    }));

    match result {
        Ok(result) => result,
        Err(panic_info) => {
            let panic_message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred during address withdrawal".to_string()
            };
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Panic during address withdrawal: {}", panic_message),
            ))
        }
    }
}

unsafe fn dash_sdk_address_withdraw_funds_inner(
    sdk_handle: *const SDKHandle,
    inputs: *const DashSDKAddressTransferInput,
    inputs_count: usize,
    core_address: *const c_char,
    core_fee_per_byte: u32,
    pooling: DashSDKPooling,
    fee_from_input_index: u16,
    change_address: *const u8,
    change_address_len: usize,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if inputs.is_null() || inputs_count == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Inputs array is null or empty".to_string(),
        ));
    }

    if core_address.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Core address is null".to_string(),
        ));
    }

    if (fee_from_input_index as usize) >= inputs_count {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "Fee input index {} is out of bounds (inputs count: {})",
                fee_from_input_index, inputs_count
            ),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    // Parse core address
    let core_address_str = match CStr::from_ptr(core_address).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Failed to parse core address C string: {}", e),
            ))
        }
    };

    let dash_address = match Address::<NetworkUnchecked>::from_str(core_address_str) {
        Ok(addr) => addr.assume_checked(),
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid Dash Core address '{}': {}", core_address_str, e),
            ))
        }
    };

    // Create CoreScript from the address
    let output_script = CoreScript::new(dash_address.script_pubkey());

    // Parse inputs and create signer (same as transfer)
    let mut input_map: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    let mut signer = AddressSigner::new();

    let inputs_slice = std::slice::from_raw_parts(inputs, inputs_count);
    for (i, input) in inputs_slice.iter().enumerate() {
        if input.address.is_null() || input.address_len == 0 {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Input {} has null or empty address", i),
            ));
        }

        if input.private_key.is_null() {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Input {} has null private key", i),
            ));
        }

        let address_bytes = std::slice::from_raw_parts(input.address, input.address_len);
        let address = match PlatformAddress::from_bytes(address_bytes) {
            Ok(addr) => addr,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Failed to parse input address {}: {}", i, e),
                ))
            }
        };

        // Parse private key (32 bytes)
        let pk_bytes = std::slice::from_raw_parts(input.private_key, 32);
        let secret_key = match SecretKey::from_slice(pk_bytes) {
            Ok(sk) => sk,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Failed to parse private key for input {}: {}", i, e),
                ))
            }
        };

        // Create PrivateKey (network doesn't matter for signing)
        let private_key = PrivateKey::new(secret_key, Network::Testnet);

        signer.add_key(&address, private_key);
        input_map.insert(address, input.amount);
    }

    // Parse optional change address
    let change_output: Option<(PlatformAddress, Credits)> =
        if !change_address.is_null() && change_address_len > 0 {
            let change_bytes = std::slice::from_raw_parts(change_address, change_address_len);
            match PlatformAddress::from_bytes(change_bytes) {
                Ok(addr) => {
                    // For change output, we don't know the amount upfront (it's calculated by the SDK)
                    // So we pass 0 and let the SDK calculate it
                    Some((addr, 0))
                }
                Err(e) => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Failed to parse change address: {}", e),
                    ))
                }
            }
        } else {
            None
        };

    // Create fee strategy: deduct from specified input
    let fee_strategy: AddressFundsFeeStrategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(
        fee_from_input_index,
    )];

    // Use default core fee if 0
    let core_fee = if core_fee_per_byte > 0 {
        core_fee_per_byte
    } else {
        1 // Default
    };

    // Execute the withdrawal
    let result: Result<DashSDKAddressInfoMap, FFIError> = wrapper.runtime.block_on(async {
        let address_infos = wrapper
            .sdk
            .withdraw_address_funds(
                input_map,
                change_output,
                fee_strategy,
                core_fee,
                pooling.into(),
                output_script,
                &signer,
                None,
            )
            .await
            .map_err(FFIError::from)?;

        // Convert to FFI type (same as transfer)
        let entries: Vec<DashSDKAddressInfoEntry> = address_infos
            .iter()
            .map(|(address, info_opt)| {
                let address_bytes = address.to_bytes();
                let address_len = address_bytes.len();
                let address_ptr = address_bytes.as_ptr() as *mut u8;
                std::mem::forget(address_bytes);

                // Handle Option<AddressInfo>
                let (nonce, balance) = match info_opt {
                    Some(info) => (info.nonce, info.balance),
                    None => (u32::MAX, u64::MAX), // Sentinel values for not found
                };

                DashSDKAddressInfoEntry {
                    address: address_ptr,
                    address_len,
                    nonce,
                    balance,
                }
            })
            .collect();

        let entries_len = entries.len();
        let entries_ptr = if entries.is_empty() {
            std::ptr::null_mut()
        } else {
            let boxed = entries.into_boxed_slice();
            Box::into_raw(boxed) as *mut DashSDKAddressInfoEntry
        };

        Ok(DashSDKAddressInfoMap {
            entries: entries_ptr,
            count: entries_len,
        })
    });

    match result {
        Ok(map) => DashSDKResult::success_address_info_map(map),
        Err(e) => DashSDKResult::error(e.into()),
    }
}
