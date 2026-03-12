//! Builder functions that construct shielded state transitions without broadcasting.
//!
//! Each function returns the serialized `StateTransition` bytes. The caller can
//! inspect, store, or later broadcast them via `dash_sdk_shielded_broadcast`.

use crate::address::AddressSigner;
use crate::identity::{create_chain_asset_lock_proof, create_instant_asset_lock_proof};
use crate::sdk::SDKWrapper;
use crate::shielded::transitions::shield::DashSDKShieldInput;
use crate::shielded::types::{convert_orchard_bundle_params, DashSDKOrchardBundleParams};
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType, FFIError};
use dash_sdk::dpp::address_funds::{
    AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress,
};
use dash_sdk::dpp::dashcore::secp256k1::SecretKey;
use dash_sdk::dpp::dashcore::{Network, PrivateKey};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::identity::core_script::CoreScript;
use dash_sdk::dpp::prelude::AddressNonce;
use dash_sdk::dpp::serialization::PlatformSerializable;
use dash_sdk::dpp::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
use dash_sdk::dpp::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use dash_sdk::dpp::state_transition::shield_transition::methods::ShieldTransitionMethodsV0;
use dash_sdk::dpp::state_transition::shield_transition::ShieldTransition;
use dash_sdk::dpp::state_transition::shielded_transfer_transition::methods::ShieldedTransferTransitionMethodsV0;
use dash_sdk::dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use dash_sdk::dpp::state_transition::shielded_withdrawal_transition::methods::ShieldedWithdrawalTransitionMethodsV0;
use dash_sdk::dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
use dash_sdk::dpp::state_transition::unshield_transition::methods::UnshieldTransitionMethodsV0;
use dash_sdk::dpp::state_transition::unshield_transition::UnshieldTransition;
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::dpp::withdrawal::Pooling;
use dash_sdk::platform::FetchMany;
use drive_proof_verifier::types::AddressInfo;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CString;
use std::os::raw::c_void;

/// Helper to serialize a StateTransition and return as DashSDKResult with byte data.
fn serialize_state_transition(st: &StateTransition) -> DashSDKResult {
    match st.serialize_to_bytes() {
        Ok(bytes) => {
            let len = bytes.len();
            let boxed = bytes.into_boxed_slice();
            let ptr = Box::into_raw(boxed) as *mut c_void;
            // Return bytes with length encoded as a JSON string "{\"bytes\":\"<hex>\",\"len\":<n>}"
            // Actually, return raw bytes — caller gets pointer + length via the data field.
            // We'll use String data type with hex encoding for FFI safety.
            let hex_str = hex::encode(unsafe { std::slice::from_raw_parts(ptr as *const u8, len) });
            match CString::new(hex_str) {
                Ok(c_str) => {
                    // Free the raw bytes we allocated above
                    unsafe {
                        let _ =
                            Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr as *mut u8, len));
                    }
                    DashSDKResult {
                        data_type: DashSDKResultDataType::String,
                        data: c_str.into_raw() as *mut c_void,
                        error: std::ptr::null_mut(),
                    }
                }
                Err(e) => {
                    unsafe {
                        let _ =
                            Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr as *mut u8, len));
                    }
                    DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InternalError,
                        format!("Failed to create CString: {}", e),
                    ))
                }
            }
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("Failed to serialize state transition: {}", e),
        )),
    }
}

/// Build a shielded transfer (shielded-to-shielded) state transition without broadcasting.
///
/// Returns the serialized state transition as a hex-encoded string.
/// Use `dash_sdk_shielded_broadcast` to send it later.
///
/// # Safety
/// - `sdk_handle` must be a valid SDK handle.
/// - `bundle` must be a valid pointer to `DashSDKOrchardBundleParams`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_build_transfer(
    sdk_handle: *const SDKHandle,
    bundle: *const DashSDKOrchardBundleParams,
    value_balance: u64,
) -> DashSDKResult {
    if sdk_handle.is_null() || bundle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or bundle is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let orchard_bundle = match convert_orchard_bundle_params(&*bundle) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(e.into()),
    };

    let st = match ShieldedTransferTransition::try_from_bundle(
        orchard_bundle.actions,
        value_balance,
        orchard_bundle.anchor,
        orchard_bundle.proof,
        orchard_bundle.binding_signature,
        wrapper.sdk.version(),
    ) {
        Ok(st) => st,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to build shielded transfer: {}", e),
            ))
        }
    };

    serialize_state_transition(&st)
}

/// Build an unshield (shielded → platform address) state transition without broadcasting.
///
/// # Safety
/// - All pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_build_unshield(
    sdk_handle: *const SDKHandle,
    output_address: *const u8,
    output_address_len: usize,
    unshielding_amount: u64,
    bundle: *const DashSDKOrchardBundleParams,
) -> DashSDKResult {
    if sdk_handle.is_null() || bundle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or bundle is null".to_string(),
        ));
    }

    if output_address.is_null() || output_address_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Output address is null or empty".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let address_bytes = std::slice::from_raw_parts(output_address, output_address_len);
    let address = match PlatformAddress::from_bytes(address_bytes) {
        Ok(addr) => addr,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid output address: {}", e),
            ))
        }
    };

    let orchard_bundle = match convert_orchard_bundle_params(&*bundle) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(e.into()),
    };

    let st = match UnshieldTransition::try_from_bundle(
        address,
        orchard_bundle.actions,
        unshielding_amount,
        orchard_bundle.anchor,
        orchard_bundle.proof,
        orchard_bundle.binding_signature,
        wrapper.sdk.version(),
    ) {
        Ok(st) => st,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to build unshield transition: {}", e),
            ))
        }
    };

    serialize_state_transition(&st)
}

/// Build a shield (platform addresses → shielded pool) state transition without broadcasting.
///
/// Requires SDK handle to fetch address nonces from the network.
///
/// # Safety
/// - All pointers must be valid. Private keys must be exactly 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_build_shield(
    sdk_handle: *const SDKHandle,
    inputs: *const DashSDKShieldInput,
    inputs_count: u32,
    bundle: *const DashSDKOrchardBundleParams,
    amount: u64,
    fee_from_input_index: u16,
) -> DashSDKResult {
    if sdk_handle.is_null() || bundle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or bundle is null".to_string(),
        ));
    }

    if inputs.is_null() || inputs_count == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Inputs array is null or empty".to_string(),
        ));
    }

    if (fee_from_input_index as u32) >= inputs_count {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "Fee input index {} out of bounds (count: {})",
                fee_from_input_index, inputs_count
            ),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let mut input_map: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    let mut signer = AddressSigner::new();

    let inputs_slice = std::slice::from_raw_parts(inputs, inputs_count as usize);
    for (i, input) in inputs_slice.iter().enumerate() {
        if input.address.is_null() || input.address_len == 0 || input.private_key.is_null() {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Input {} has null address or private key", i),
            ));
        }

        let address_bytes = std::slice::from_raw_parts(input.address, input.address_len);
        let address = match PlatformAddress::from_bytes(address_bytes) {
            Ok(addr) => addr,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Input {} invalid address: {}", i, e),
                ))
            }
        };

        let pk_bytes = std::slice::from_raw_parts(input.private_key, 32);
        let secret_key = match SecretKey::from_slice(pk_bytes) {
            Ok(sk) => sk,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Input {} invalid private key: {}", i, e),
                ))
            }
        };

        let private_key = PrivateKey::new(secret_key, Network::Testnet);
        signer.add_key(&address, private_key);
        input_map.insert(address, input.amount);
    }

    let orchard_bundle = match convert_orchard_bundle_params(&*bundle) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(e.into()),
    };

    let fee_strategy: AddressFundsFeeStrategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(
        fee_from_input_index,
    )];

    let user_fee_increase = 0u16;

    // Fetch nonces from the network and build the transition
    let result = wrapper.runtime.block_on(async {
        // Fetch address infos to get nonces
        let addresses: BTreeSet<PlatformAddress> = input_map.keys().copied().collect();
        let address_infos = AddressInfo::fetch_many(&wrapper.sdk, addresses)
            .await
            .map_err(FFIError::SDKError)?;

        // Build inputs with nonce+1 (next nonce)
        let mut inputs_with_nonce: BTreeMap<PlatformAddress, (AddressNonce, Credits)> =
            BTreeMap::new();
        for (address, amount) in &input_map {
            let info = address_infos
                .get(address)
                .and_then(|opt| opt.as_ref())
                .ok_or_else(|| {
                    FFIError::InternalError(format!(
                        "Address not found on platform: {}",
                        hex::encode(address.to_bytes())
                    ))
                })?;
            inputs_with_nonce.insert(*address, (info.nonce + 1, *amount));
        }

        let st = ShieldTransition::try_from_bundle_with_signer(
            inputs_with_nonce,
            orchard_bundle.actions,
            amount,
            orchard_bundle.anchor,
            orchard_bundle.proof,
            orchard_bundle.binding_signature,
            fee_strategy,
            &signer,
            user_fee_increase,
            wrapper.sdk.version(),
        )
        .map_err(|e| {
            FFIError::InternalError(format!("Failed to build shield transition: {}", e))
        })?;

        Ok::<StateTransition, FFIError>(st)
    });

    match result {
        Ok(st) => serialize_state_transition(&st),
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Build a shield-from-instant-lock state transition without broadcasting.
///
/// # Safety
/// - All pointers must be valid. `private_key` must be exactly 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_build_shield_from_instant_lock(
    sdk_handle: *const SDKHandle,
    instant_lock_bytes: *const u8,
    instant_lock_len: usize,
    transaction_bytes: *const u8,
    transaction_len: usize,
    output_index: u32,
    private_key: *const u8,
    bundle: *const DashSDKOrchardBundleParams,
    value_balance: u64,
) -> DashSDKResult {
    if sdk_handle.is_null()
        || instant_lock_bytes.is_null()
        || transaction_bytes.is_null()
        || private_key.is_null()
        || bundle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required pointers are null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let asset_lock_proof = match create_instant_asset_lock_proof(
        instant_lock_bytes,
        instant_lock_len,
        transaction_bytes,
        transaction_len,
        output_index,
    ) {
        Ok(proof) => proof,
        Err(e) => return DashSDKResult::error(DashSDKError::from(e)),
    };

    let pk_bytes = std::slice::from_raw_parts(private_key, 32);

    let orchard_bundle = match convert_orchard_bundle_params(&*bundle) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(DashSDKError::from(e)),
    };

    let st = match ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle(
        asset_lock_proof,
        pk_bytes,
        orchard_bundle.actions,
        value_balance,
        orchard_bundle.anchor,
        orchard_bundle.proof,
        orchard_bundle.binding_signature,
        wrapper.sdk.version(),
    ) {
        Ok(st) => st,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to build shield from asset lock: {}", e),
            ))
        }
    };

    serialize_state_transition(&st)
}

/// Build a shield-from-chain-lock state transition without broadcasting.
///
/// # Safety
/// - All pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_build_shield_from_chain_lock(
    sdk_handle: *const SDKHandle,
    core_chain_locked_height: u32,
    out_point_bytes: *const [u8; 36],
    private_key: *const u8,
    bundle: *const DashSDKOrchardBundleParams,
    value_balance: u64,
) -> DashSDKResult {
    if sdk_handle.is_null()
        || out_point_bytes.is_null()
        || private_key.is_null()
        || bundle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required pointers are null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let asset_lock_proof =
        match create_chain_asset_lock_proof(core_chain_locked_height, out_point_bytes) {
            Ok(proof) => proof,
            Err(e) => return DashSDKResult::error(DashSDKError::from(e)),
        };

    let pk_bytes = std::slice::from_raw_parts(private_key, 32);

    let orchard_bundle = match convert_orchard_bundle_params(&*bundle) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(DashSDKError::from(e)),
    };

    let st = match ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle(
        asset_lock_proof,
        pk_bytes,
        orchard_bundle.actions,
        value_balance,
        orchard_bundle.anchor,
        orchard_bundle.proof,
        orchard_bundle.binding_signature,
        wrapper.sdk.version(),
    ) {
        Ok(st) => st,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to build shield from chain lock: {}", e),
            ))
        }
    };

    serialize_state_transition(&st)
}

/// Build a shielded withdrawal (shielded → L1) state transition without broadcasting.
///
/// # Safety
/// - All pointers must be valid.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_build_withdrawal(
    sdk_handle: *const SDKHandle,
    unshielding_amount: u64,
    bundle: *const DashSDKOrchardBundleParams,
    core_fee_per_byte: u32,
    pooling: u8,
    output_script: *const u8,
    output_script_len: usize,
) -> DashSDKResult {
    if sdk_handle.is_null() || bundle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or bundle is null".to_string(),
        ));
    }

    if output_script.is_null() || output_script_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Output script is null or empty".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let pooling_enum = match pooling {
        0 => Pooling::Never,
        1 => Pooling::IfAvailable,
        2 => Pooling::Standard,
        _ => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid pooling value: {} (expected 0, 1, or 2)", pooling),
            ))
        }
    };

    let script_bytes = std::slice::from_raw_parts(output_script, output_script_len);
    let core_script = CoreScript::new(
        dash_sdk::dpp::dashcore::blockdata::script::ScriptBuf::from_bytes(script_bytes.to_vec()),
    );

    let orchard_bundle = match convert_orchard_bundle_params(&*bundle) {
        Ok(b) => b,
        Err(e) => return DashSDKResult::error(e.into()),
    };

    let st = match ShieldedWithdrawalTransition::try_from_bundle(
        orchard_bundle.actions,
        unshielding_amount,
        orchard_bundle.anchor,
        orchard_bundle.proof,
        orchard_bundle.binding_signature,
        core_fee_per_byte,
        pooling_enum,
        core_script,
        wrapper.sdk.version(),
    ) {
        Ok(st) => st,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to build shielded withdrawal: {}", e),
            ))
        }
    };

    serialize_state_transition(&st)
}
