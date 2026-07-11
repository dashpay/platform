//! Address top-up from asset lock state transition
//!
//! This module provides FFI functions to top up Platform addresses using asset lock proofs.

use dash_sdk::dpp::address_funds::{
    AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress,
};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::platform::transition::top_up_address::TopUpAddress;
use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};

use crate::address::transitions::transfer::AddressSigner;
use crate::identity::{
    convert_put_settings, create_chain_asset_lock_proof, create_instant_asset_lock_proof,
    parse_private_key,
};
use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKAddressInfoEntry, DashSDKAddressInfoMap, DashSDKAddressTransferOutput,
    DashSDKPutSettings, SDKHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Asset lock proof type
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashSDKAssetLockProofType {
    /// Instant lock proof
    Instant = 0,
    /// Chain lock proof
    Chain = 1,
}

/// Top up Platform addresses using an asset lock proof
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `proof_type`: Type of asset lock proof (Instant or Chain)
/// - For Instant: `instant_lock_bytes`, `instant_lock_len`, `transaction_bytes`, `transaction_len`, `output_index`
/// - For Chain: `core_chain_locked_height`, `out_point_bytes` (36 bytes)
/// - `asset_lock_private_key`: Private key for the asset lock (32 bytes)
/// - `outputs`: Array of output addresses with optional amounts
/// - `outputs_count`: Number of output entries
/// - `fee_from_input_index`: Which input index to deduct fees from (0-based, max 65535)
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// DashSDKResult with data_type = AddressInfoMap containing updated address balances
///
/// # Safety
/// - All pointers must be valid and non-null (except put_settings which can be null).
/// - Arrays must contain at least the specified count of elements.
/// - Private key must be exactly 32 bytes.
/// - For Chain proof: out_point_bytes must be exactly 36 bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_top_up_from_asset_lock(
    sdk_handle: *const SDKHandle,
    proof_type: DashSDKAssetLockProofType,
    // Instant lock parameters
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    // Chain lock parameters
    core_chain_locked_height: u32,
    out_point_bytes: *const [u8; 36],
    // Common parameters
    asset_lock_private_key: *const [u8; 32],
    outputs: *const DashSDKAddressTransferOutput,
    outputs_count: usize,
    fee_from_input_index: u16,
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // SAFETY: catch_unwind is kept intentionally despite `panic = "abort"` in the release profile.
    // With panic=abort, catch_unwind is optimized away (zero cost). But keeping it:
    // 1. Acts as a safety net if the panic strategy is ever changed (e.g., for debugging)
    // 2. Documents the intent that panics must not cross this FFI boundary
    // 3. Follows defense-in-depth for FFI safety
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        dash_sdk_address_top_up_from_asset_lock_inner(
            sdk_handle,
            proof_type,
            instant_lock_bytes,
            instant_lock_len,
            transaction_bytes,
            transaction_len,
            output_index,
            core_chain_locked_height,
            out_point_bytes,
            asset_lock_private_key,
            outputs,
            outputs_count,
            fee_from_input_index,
            put_settings,
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
                "Unknown panic occurred during address top-up".to_string()
            };
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Panic during address top-up: {}", panic_message),
            ))
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn dash_sdk_address_top_up_from_asset_lock_inner(
    sdk_handle: *const SDKHandle,
    proof_type: DashSDKAssetLockProofType,
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    core_chain_locked_height: u32,
    out_point_bytes: *const [u8; 36],
    asset_lock_private_key: *const [u8; 32],
    outputs: *const DashSDKAddressTransferOutput,
    outputs_count: usize,
    fee_from_input_index: u16,
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if asset_lock_private_key.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Asset lock private key is null".to_string(),
        ));
    }

    if outputs.is_null() || outputs_count == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Outputs array is null or empty".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    // Create asset lock proof based on type
    let asset_lock_proof = match proof_type {
        DashSDKAssetLockProofType::Instant => {
            if instant_lock_bytes.is_null() || transaction_bytes.is_null() {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    "Instant lock requires instant_lock_bytes and transaction_bytes".to_string(),
                ));
            }
            match create_instant_asset_lock_proof(
                instant_lock_bytes,
                instant_lock_len,
                transaction_bytes,
                transaction_len,
                output_index,
            ) {
                Ok(proof) => proof,
                Err(e) => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Failed to create instant asset lock proof: {}", e),
                    ))
                }
            }
        }
        DashSDKAssetLockProofType::Chain => {
            if out_point_bytes.is_null() {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    "Chain lock requires out_point_bytes".to_string(),
                ));
            }
            match create_chain_asset_lock_proof(core_chain_locked_height, out_point_bytes) {
                Ok(proof) => proof,
                Err(e) => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Failed to create chain asset lock proof: {}", e),
                    ))
                }
            }
        }
    };

    // Parse private key
    let private_key = match parse_private_key(asset_lock_private_key) {
        Ok(key) => key,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Failed to parse private key: {}", e),
            ))
        }
    };

    // Parse outputs
    let mut output_map: BTreeMap<PlatformAddress, Option<Credits>> = BTreeMap::new();
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

        // Amount is optional (None means SDK will calculate)
        let amount = if output.amount > 0 {
            Some(output.amount)
        } else {
            None
        };

        output_map.insert(address, amount);
    }

    // Create fee strategy: deduct from input (index 0, since we have no inputs)
    // For asset lock top-up, fees are typically deducted from the asset lock output
    let fee_strategy: AddressFundsFeeStrategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(
        fee_from_input_index,
    )];

    // Create signer (empty since we don't have address inputs for asset lock top-up)
    let signer = AddressSigner::new();

    // Convert settings
    let settings = convert_put_settings(put_settings);

    // Execute the top-up
    let result: Result<DashSDKAddressInfoMap, FFIError> = wrapper.runtime.block_on(async {
        // Use TopUpAddress trait - we need to call it on the output map
        let address_infos = output_map
            .top_up(
                &wrapper.sdk,
                asset_lock_proof,
                private_key,
                fee_strategy,
                &signer,
                settings,
            )
            .await
            .map_err(FFIError::from)?;

        // Convert to FFI type (same as transfer)
        let (address_infos, _proof_height) = address_infos;
        // The `_proof_height` pin matters to callers that PERSIST these
        // absolutes (the platform-wallet reconcile seam); this raw debug
        // path only renders the proof entries, so it drops the height.
        let entries: Vec<DashSDKAddressInfoEntry> = address_infos
            .iter()
            .map(|(address, info_opt)| {
                let address_bytes = address.to_bytes();
                let address_len = address_bytes.len();
                let address_ptr = Box::into_raw(address_bytes.into_boxed_slice()) as *mut u8;

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
