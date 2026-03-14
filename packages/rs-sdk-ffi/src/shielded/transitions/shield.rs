//! Shield (platform addresses → shielded pool) FFI binding.

use crate::address::AddressSigner;
use crate::sdk::SDKWrapper;
use crate::shielded::types::{convert_orchard_bundle_params, DashSDKOrchardBundleParams};
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::address_funds::{
    AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress,
};
use dash_sdk::dpp::dashcore::secp256k1::SecretKey;
use dash_sdk::dpp::dashcore::{Network, PrivateKey};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::platform::transition::shield::ShieldFunds;
use std::collections::BTreeMap;

/// Input entry for shield operation: address + amount + private key.
#[repr(C)]
pub struct DashSDKShieldInput {
    /// Platform address bytes.
    pub address: *const u8,
    /// Length of address bytes.
    pub address_len: usize,
    /// Amount to shield from this address (in credits).
    pub amount: u64,
    /// 32-byte private key for signing.
    pub private_key: *const u8,
}

/// Shield funds from platform addresses into the shielded pool.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `inputs`: Array of input addresses with amounts and private keys
/// - `inputs_count`: Number of inputs
/// - `bundle`: Orchard bundle parameters
/// - `amount`: Total amount being shielded (must match bundle value balance)
/// - `fee_from_input_index`: Which input index to deduct fees from (0-based)
///
/// # Returns
/// `DashSDKResult` with no data on success, error on failure.
///
/// # Safety
/// - All pointers must be valid. Private keys must be exactly 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_shield_funds(
    sdk_handle: *const SDKHandle,
    inputs: *const DashSDKShieldInput,
    inputs_count: u32,
    bundle: *const DashSDKOrchardBundleParams,
    amount: u64,
    fee_from_input_index: u16,
) -> DashSDKResult {
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

    if bundle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Bundle params is null".to_string(),
        ));
    }

    if (fee_from_input_index as u32) >= inputs_count {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "Fee input index {} is out of bounds (inputs count: {})",
                fee_from_input_index, inputs_count
            ),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    // Parse inputs and create signer, preserving original order
    let mut input_map: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    let mut signer = AddressSigner::new();
    let mut ordered_addresses: Vec<PlatformAddress> = Vec::with_capacity(inputs_count as usize);

    let inputs_slice = std::slice::from_raw_parts(inputs, inputs_count as usize);
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

        // Reject duplicate addresses — BTreeMap would silently drop them
        if input_map.contains_key(&address) {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Duplicate address at input index {}", i),
            ));
        }

        let private_key = PrivateKey::new(secret_key, Network::Testnet);
        signer.add_key(&address, private_key);
        ordered_addresses.push(address);
        input_map.insert(address, input.amount);
    }

    let orchard_bundle = match convert_orchard_bundle_params(&*bundle) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(e.into()),
    };

    // Remap fee_from_input_index: the caller's index is in original input order,
    // but DeductFromInput resolves against BTreeMap's sorted key order.
    let fee_address = &ordered_addresses[fee_from_input_index as usize];
    let sorted_index = input_map
        .keys()
        .position(|k| k == fee_address)
        .expect("fee address must exist in input_map") as u16;

    let fee_strategy: AddressFundsFeeStrategy =
        vec![AddressFundsFeeStrategyStep::DeductFromInput(sorted_index)];

    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .shield_funds(
                input_map,
                orchard_bundle,
                amount,
                fee_strategy,
                &signer,
                None,
            )
            .await
            .map_err(FFIError::from)
    });

    match result {
        Ok(()) => DashSDKResult::success(std::ptr::null_mut()),
        Err(e) => DashSDKResult::error(e.into()),
    }
}
