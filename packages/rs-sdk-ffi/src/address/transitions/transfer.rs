//! Address funds transfer state transition
//!
//! This module provides FFI functions to transfer funds between Platform addresses.

use dash_sdk::dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress, AddressWitness};
use dash_sdk::dpp::dashcore::secp256k1::SecretKey;
use dash_sdk::dpp::dashcore::{PrivateKey, Network};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::platform_value::BinaryData;
use dash_sdk::dpp::ProtocolError;
use dash_sdk::platform::transition::transfer_address_funds::TransferAddressFunds;
use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};

use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKAddressInfoEntry, DashSDKAddressInfoMap, DashSDKAddressTransferInput,
    DashSDKAddressTransferOutput, SDKHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Simple address signer that holds private keys for addresses
#[derive(Debug)]
pub struct AddressSigner {
    /// Maps address hash to private key
    keys: BTreeMap<[u8; 20], PrivateKey>,
}

impl AddressSigner {
    pub fn new() -> Self {
        Self {
            keys: BTreeMap::new(),
        }
    }

    pub fn add_key(&mut self, address: &PlatformAddress, private_key: PrivateKey) {
        let hash = match address {
            PlatformAddress::P2pkh(hash) => *hash,
            PlatformAddress::P2sh(hash) => *hash,
        };
        self.keys.insert(hash, private_key);
    }
}

impl Signer<PlatformAddress> for AddressSigner {
    fn sign(&self, key: &PlatformAddress, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        let hash = match key {
            PlatformAddress::P2pkh(hash) => hash,
            PlatformAddress::P2sh(hash) => hash,
        };

        let private_key = self.keys.get(hash).ok_or_else(|| {
            ProtocolError::Generic(format!(
                "No private key found for address hash {}",
                hex::encode(hash)
            ))
        })?;

        // Sign the data using dashcore signer
        let signature = dash_sdk::dpp::dashcore::signer::sign(data, private_key.inner.as_ref())
            .map_err(|e| ProtocolError::Generic(format!("Signing failed: {}", e)))?;

        Ok(BinaryData::new(signature.to_vec()))
    }

    fn sign_create_witness(
        &self,
        key: &PlatformAddress,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        let hash = match key {
            PlatformAddress::P2pkh(hash) => hash,
            PlatformAddress::P2sh(hash) => hash,
        };

        let private_key = self.keys.get(hash).ok_or_else(|| {
            ProtocolError::Generic(format!(
                "No private key found for address hash {}",
                hex::encode(hash)
            ))
        })?;

        // Sign the data
        let signature = dash_sdk::dpp::dashcore::signer::sign(data, private_key.inner.as_ref())
            .map_err(|e| ProtocolError::Generic(format!("Signing failed: {}", e)))?;

        // Create P2PKH witness (most common for single key)
        Ok(AddressWitness::P2pkh {
            signature: BinaryData::new(signature.to_vec()),
        })
    }

    fn can_sign_with(&self, key: &PlatformAddress) -> bool {
        let hash = match key {
            PlatformAddress::P2pkh(hash) => hash,
            PlatformAddress::P2sh(hash) => hash,
        };
        self.keys.contains_key(hash)
    }
}

/// Transfer funds between Platform addresses
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `inputs`: Array of input addresses with amounts and private keys
/// - `inputs_count`: Number of input entries
/// - `outputs`: Array of output addresses with amounts
/// - `outputs_count`: Number of output entries
/// - `fee_from_input_index`: Which input index to deduct fees from (0-based, max 65535)
///
/// # Returns
/// DashSDKResult with data_type = AddressInfoMap containing updated address balances
///
/// # Safety
/// - All pointers must be valid and non-null.
/// - Arrays must contain at least the specified count of elements.
/// - Private keys must be exactly 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_transfer_funds(
    sdk_handle: *const SDKHandle,
    inputs: *const DashSDKAddressTransferInput,
    inputs_count: usize,
    outputs: *const DashSDKAddressTransferOutput,
    outputs_count: usize,
    fee_from_input_index: u16,
) -> DashSDKResult {
    // Wrap in catch_unwind for panic safety
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        dash_sdk_address_transfer_funds_inner(
            sdk_handle,
            inputs,
            inputs_count,
            outputs,
            outputs_count,
            fee_from_input_index,
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
                "Unknown panic occurred during address transfer".to_string()
            };
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Panic during address transfer: {}", panic_message),
            ))
        }
    }
}

unsafe fn dash_sdk_address_transfer_funds_inner(
    sdk_handle: *const SDKHandle,
    inputs: *const DashSDKAddressTransferInput,
    inputs_count: usize,
    outputs: *const DashSDKAddressTransferOutput,
    outputs_count: usize,
    fee_from_input_index: u16,
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

    if outputs.is_null() || outputs_count == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Outputs array is null or empty".to_string(),
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

    // Parse inputs and create signer
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

    // Parse outputs
    let mut output_map: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    let outputs_slice = std::slice::from_raw_parts(outputs, outputs_count);
    for (i, output) in outputs_slice.iter().enumerate() {
        if output.address.is_null() || output.address_len == 0 {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Output {} has null or empty address", i),
            ));
        }

        let address_bytes = std::slice::from_raw_parts(output.address, output.address_len);
        let address = match PlatformAddress::from_bytes(address_bytes) {
            Ok(addr) => addr,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Failed to parse output address {}: {}", i, e),
                ))
            }
        };

        output_map.insert(address, output.amount);
    }

    // Create fee strategy: deduct from specified input
    let fee_strategy: AddressFundsFeeStrategy =
        vec![AddressFundsFeeStrategyStep::DeductFromInput(fee_from_input_index)];

    // Execute the transfer
    let result: Result<DashSDKAddressInfoMap, FFIError> = wrapper.runtime.block_on(async {
        let address_infos = wrapper
            .sdk
            .transfer_address_funds(input_map, output_map, fee_strategy, &signer, None)
            .await
            .map_err(FFIError::from)?;

        // Convert to FFI type
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
