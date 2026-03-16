//! Bundle building FFI functions for the shielded pool client.
//!
//! Uses the commitment tree's witness generation and the DPP public builder
//! functions to construct authorized Orchard bundles. iOS receives opaque
//! `DashSDKOrchardBundleParams` handles and never sees Merkle paths.

use std::os::raw::c_void;

use dash_sdk::dpp::address_funds::OrchardAddress;
use dash_sdk::dpp::identity::core_script::CoreScript;
use dash_sdk::dpp::shielded::builder::{
    build_shielded_transfer_transition, build_shielded_withdrawal_transition,
    build_unshield_transition, serialize_authorized_bundle, SpendableNote,
};
use dash_sdk::dpp::shielded::compute_minimum_shielded_fee;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::dpp::withdrawal::Pooling;

// Access crypto items from the sibling `crypto` module via `super`.
// pool_client is a child of shielded, and crypto is also a child of shielded.
// In Rust, child modules can access private siblings through `super`.
use super::super::crypto::{bundle_to_ffi_params, CachedProver};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};

use super::ShieldedPoolClient;

/// Select notes to cover the requested amount using a greedy algorithm
/// (largest-value first).
fn select_notes_for_amount(
    state: &super::ShieldedPoolState,
    amount: u64,
) -> Result<(Vec<&super::ShieldedNote>, u64), String> {
    let unspent = state.unspent_notes();

    if unspent.is_empty() {
        return Err("No unspent shielded notes available".to_string());
    }

    let total_available: u64 = unspent.iter().map(|n| n.value).sum();
    if total_available < amount {
        return Err(format!(
            "Insufficient shielded balance: have {}, need {}",
            total_available, amount
        ));
    }

    let mut sorted = unspent;
    sorted.sort_by(|a, b| b.value.cmp(&a.value));

    let mut selected = Vec::new();
    let mut accumulated = 0u64;

    for note in sorted {
        selected.push(note);
        accumulated += note.value;
        if accumulated >= amount {
            break;
        }
    }

    Ok((selected, accumulated))
}

/// Convert a `PaymentAddress` to an `OrchardAddress`.
fn payment_address_to_orchard(
    addr: &dash_sdk::grovedb_commitment_tree::PaymentAddress,
) -> Result<OrchardAddress, String> {
    let raw = addr.to_raw_address_bytes();
    OrchardAddress::from_raw_bytes(&raw).map_err(|e| format!("Invalid Orchard address: {}", e))
}

/// Get Merkle witnesses and anchor for the selected notes from the locked state.
fn get_witnesses_and_anchor(
    state: &super::ShieldedPoolState,
    selected: &[&super::ShieldedNote],
) -> Result<
    (
        Vec<SpendableNote>,
        dash_sdk::grovedb_commitment_tree::Anchor,
    ),
    String,
> {
    let tree = &state.commitment_tree;

    let spends = selected
        .iter()
        .map(|note| {
            let merkle_path = tree
                .witness(note.position, 0)
                .map_err(|e| format!("Failed to get Merkle witness: {}", e))?
                .ok_or("No Merkle path available for note")?;
            Ok(SpendableNote {
                note: note.note,
                merkle_path,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let anchor = tree
        .anchor()
        .map_err(|e| format!("Failed to get tree anchor: {}", e))?;

    Ok((spends, anchor))
}

/// Return a `DashSDKResult` containing a heap-allocated `DashSDKOrchardBundleParams`.
fn bundle_result(sb: &dash_sdk::dpp::shielded::builder::SerializedBundle) -> DashSDKResult {
    let ptr = bundle_to_ffi_params(sb);
    DashSDKResult {
        data_type: DashSDKResultDataType::BinaryData,
        data: ptr as *mut c_void,
        error: std::ptr::null_mut(),
    }
}

/// Parse the optional 36-byte memo pointer. Returns `[0u8; 36]` if null.
unsafe fn parse_memo(memo: *const [u8; 36]) -> [u8; 36] {
    if memo.is_null() {
        [0u8; 36]
    } else {
        *memo
    }
}

// ---------------------------------------------------------------------------
// FFI functions
// ---------------------------------------------------------------------------

/// Build a shield bundle (transparent -> shielded). No tree witness needed.
///
/// Creates an output-only bundle that deposits `amount` credits into the
/// client's default shielded address.
///
/// # Safety
/// - `handle` must be a valid pointer to a `ShieldedPoolClient`.
/// - `memo`, if non-null, must point to exactly 36 bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_pool_client_build_shield_bundle(
    handle: *const ShieldedPoolClient,
    amount: u64,
    memo: *const [u8; 36],
) -> DashSDKResult {
    if handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "handle is null".to_string(),
        ));
    }

    if amount == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "shield amount must be greater than zero".to_string(),
        ));
    }

    let client = &*handle;
    let memo = parse_memo(memo);
    let prover = CachedProver;

    let recipient = &client.keys.default_address;

    // Build output-only bundle using the local builder (same as the existing
    // `dash_sdk_shielded_build_shield_bundle` pattern).
    use dash_sdk::dpp::shielded::builder::OrchardProver;
    use dash_sdk::dpp::shielded::compute_platform_sighash;
    use dash_sdk::grovedb_commitment_tree::{
        Anchor, Builder, BundleType, DashMemo, Flags as OrchardFlags, NoteValue,
    };
    use rand::rngs::OsRng;

    let anchor = Anchor::empty_tree();
    let mut builder = Builder::<DashMemo>::new(
        BundleType::Transactional {
            flags: OrchardFlags::SPENDS_DISABLED,
            bundle_required: false,
        },
        anchor,
    );

    if let Err(e) = builder.add_output(None, *recipient, NoteValue::from_raw(amount), memo) {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::CryptoError,
            format!("Failed to add output: {:?}", e),
        ));
    }

    let mut rng = OsRng;
    let (unauthorized, _) = match builder.build::<i64>(&mut rng) {
        Ok(Some(pair)) => pair,
        Ok(None) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::CryptoError,
                "Bundle was empty after build".to_string(),
            ))
        }
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::CryptoError,
                format!("Failed to build bundle: {:?}", e),
            ))
        }
    };

    let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
    let sighash = compute_platform_sighash(&bundle_commitment, &[]);

    let proven = match unauthorized.create_proof(prover.proving_key(), &mut rng) {
        Ok(p) => p,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::CryptoError,
                format!("Failed to create proof: {:?}", e),
            ))
        }
    };

    let authorized = match proven.apply_signatures(rng, sighash, &[]) {
        Ok(b) => b,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::CryptoError,
                format!("Failed to apply signatures: {:?}", e),
            ))
        }
    };

    let sb = serialize_authorized_bundle(&authorized);
    bundle_result(&sb)
}

/// Build a transfer bundle (shielded -> shielded).
///
/// Selects notes, gets witnesses from the commitment tree, and builds an
/// authorized bundle using the DPP builder.
///
/// # Safety
/// - `handle` must be a valid pointer to a `ShieldedPoolClient`.
/// - `recipient_address` must point to `recipient_address_len` bytes (43 expected).
/// - `memo`, if non-null, must point to exactly 36 bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_pool_client_build_transfer_bundle(
    handle: *const ShieldedPoolClient,
    recipient_address: *const u8,
    recipient_address_len: usize,
    amount: u64,
    memo: *const [u8; 36],
) -> DashSDKResult {
    if handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "handle is null".to_string(),
        ));
    }

    if recipient_address.is_null() || recipient_address_len != 43 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "recipient address must be 43 bytes, got {}",
                if recipient_address.is_null() {
                    0
                } else {
                    recipient_address_len
                }
            ),
        ));
    }

    if amount == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "transfer amount must be greater than zero".to_string(),
        ));
    }

    let client = &*handle;
    let memo = parse_memo(memo);
    let prover = CachedProver;
    let platform_version = PlatformVersion::latest();

    // Parse recipient address.
    let addr_slice = std::slice::from_raw_parts(recipient_address, 43);
    let mut addr_array = [0u8; 43];
    addr_array.copy_from_slice(addr_slice);
    let recipient_addr = match OrchardAddress::from_raw_bytes(&addr_array) {
        Ok(a) => a,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid recipient address: {}", e),
            ))
        }
    };

    // Lock state once for note selection and witness generation.
    let state = client.state.lock().unwrap();

    // Fee-aware note selection: select enough to cover amount + estimated fee.
    let (mut selected, mut total) = match select_notes_for_amount(&state, amount) {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    // Transfer has recipient + change = at least 2 outputs.
    let num_actions = selected.len().max(2);
    let estimated_fee = compute_minimum_shielded_fee(num_actions, platform_version);
    let required = match amount.checked_add(estimated_fee) {
        Some(v) => v,
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "amount + fee overflows".to_string(),
            ))
        }
    };

    // Re-select if initial selection doesn't cover amount + fee.
    if total < required {
        match select_notes_for_amount(&state, required) {
            Ok((reselected, retotal)) => {
                selected = reselected;
                total = retotal;
                let _ = total; // suppress unused warning
            }
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    e,
                ))
            }
        }
    }

    // Get witnesses and anchor from locked state.
    let (spends, anchor) = match get_witnesses_and_anchor(&state, &selected) {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::CryptoError, e)),
    };

    // Drop state lock before expensive proof generation.
    drop(state);

    let change_addr = match payment_address_to_orchard(&client.keys.default_address) {
        Ok(a) => a,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InternalError, e))
        }
    };

    let st = match build_shielded_transfer_transition(
        spends,
        &recipient_addr,
        amount,
        &change_addr,
        &client.keys.fvk,
        &client.keys.ask,
        anchor,
        &prover,
        memo,
        None,
        platform_version,
    ) {
        Ok(st) => st,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::CryptoError,
                format!("Failed to build shielded transfer: {}", e),
            ))
        }
    };

    // Extract the serialized bundle from the state transition.
    match extract_bundle_from_st(&st) {
        Ok(sb) => bundle_result(&sb),
        Err(e) => DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::CryptoError, e)),
    }
}

/// Build an unshield bundle (shielded -> platform address).
///
/// # Safety
/// - `handle` must be a valid pointer to a `ShieldedPoolClient`.
/// - `output_address` must point to `output_address_len` bytes.
/// - `memo`, if non-null, must point to exactly 36 bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_pool_client_build_unshield_bundle(
    handle: *const ShieldedPoolClient,
    output_address: *const u8,
    output_address_len: usize,
    amount: u64,
    memo: *const [u8; 36],
) -> DashSDKResult {
    if handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "handle is null".to_string(),
        ));
    }

    if output_address.is_null() || output_address_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "output_address is null or empty".to_string(),
        ));
    }

    if amount == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "unshield amount must be greater than zero".to_string(),
        ));
    }

    let client = &*handle;
    let memo = parse_memo(memo);
    let prover = CachedProver;
    let platform_version = PlatformVersion::latest();

    // Parse output platform address.
    let addr_slice = std::slice::from_raw_parts(output_address, output_address_len);
    let platform_addr = match dash_sdk::dpp::address_funds::PlatformAddress::from_bytes(addr_slice)
    {
        Ok(a) => a,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid output address: {}", e),
            ))
        }
    };

    // Lock state once for note selection and witness generation.
    let state = client.state.lock().unwrap();

    // Fee-aware note selection: select enough to cover amount + estimated fee.
    let (mut selected, mut total) = match select_notes_for_amount(&state, amount) {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    // Unshield has change output = at least 1 output.
    let num_actions = selected.len().max(1);
    let estimated_fee = compute_minimum_shielded_fee(num_actions, platform_version);
    let required = match amount.checked_add(estimated_fee) {
        Some(v) => v,
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "amount + fee overflows".to_string(),
            ))
        }
    };

    // Re-select if initial selection doesn't cover amount + fee.
    if total < required {
        match select_notes_for_amount(&state, required) {
            Ok((reselected, retotal)) => {
                selected = reselected;
                total = retotal;
                let _ = total;
            }
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    e,
                ))
            }
        }
    }

    // Get witnesses and anchor from locked state.
    let (spends, anchor) = match get_witnesses_and_anchor(&state, &selected) {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::CryptoError, e)),
    };

    // Drop state lock before expensive proof generation.
    drop(state);

    let change_addr = match payment_address_to_orchard(&client.keys.default_address) {
        Ok(a) => a,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InternalError, e))
        }
    };

    let st = match build_unshield_transition(
        spends,
        platform_addr,
        amount,
        &change_addr,
        &client.keys.fvk,
        &client.keys.ask,
        anchor,
        &prover,
        memo,
        None,
        platform_version,
    ) {
        Ok(st) => st,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::CryptoError,
                format!("Failed to build unshield transition: {}", e),
            ))
        }
    };

    match extract_bundle_from_st(&st) {
        Ok(sb) => bundle_result(&sb),
        Err(e) => DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::CryptoError, e)),
    }
}

/// Build a withdrawal bundle (shielded -> L1 core address).
///
/// # Safety
/// - `handle` must be a valid pointer to a `ShieldedPoolClient`.
/// - `output_script` must point to `output_script_len` bytes.
/// - `memo`, if non-null, must point to exactly 36 bytes.
/// - `pooling`: 0=Never, 1=IfAvailable, 2=Standard.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn dash_sdk_shielded_pool_client_build_withdrawal_bundle(
    handle: *const ShieldedPoolClient,
    output_script: *const u8,
    output_script_len: usize,
    amount: u64,
    memo: *const [u8; 36],
    core_fee_per_byte: u32,
    pooling: u8,
) -> DashSDKResult {
    if handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "handle is null".to_string(),
        ));
    }

    if output_script.is_null() || output_script_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "output_script is null or empty".to_string(),
        ));
    }

    if amount == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "withdrawal amount must be greater than zero".to_string(),
        ));
    }

    let client = &*handle;
    let memo = parse_memo(memo);
    let prover = CachedProver;
    let platform_version = PlatformVersion::latest();

    // Parse output script.
    let script_bytes = std::slice::from_raw_parts(output_script, output_script_len);
    let core_script = CoreScript::from_bytes(script_bytes.to_vec());

    // Map pooling byte to enum.
    let pooling_enum = match pooling {
        0 => Pooling::Never,
        1 => Pooling::IfAvailable,
        _ => Pooling::Standard,
    };

    // Lock state once for note selection and witness generation.
    let state = client.state.lock().unwrap();

    // Fee-aware note selection: select enough to cover amount + estimated fee.
    let (mut selected, mut total) = match select_notes_for_amount(&state, amount) {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    // Withdrawal has change output = at least 1 output.
    let num_actions = selected.len().max(1);
    let estimated_fee = compute_minimum_shielded_fee(num_actions, platform_version);
    let required = match amount.checked_add(estimated_fee) {
        Some(v) => v,
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "amount + fee overflows".to_string(),
            ))
        }
    };

    // Re-select if initial selection doesn't cover amount + fee.
    if total < required {
        match select_notes_for_amount(&state, required) {
            Ok((reselected, retotal)) => {
                selected = reselected;
                total = retotal;
                let _ = total;
            }
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    e,
                ))
            }
        }
    }

    // Get witnesses and anchor from locked state.
    let (spends, anchor) = match get_witnesses_and_anchor(&state, &selected) {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::CryptoError, e)),
    };

    // Drop state lock before expensive proof generation.
    drop(state);

    let change_addr = match payment_address_to_orchard(&client.keys.default_address) {
        Ok(a) => a,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InternalError, e))
        }
    };

    let st = match build_shielded_withdrawal_transition(
        spends,
        amount,
        core_script,
        core_fee_per_byte,
        pooling_enum,
        &change_addr,
        &client.keys.fvk,
        &client.keys.ask,
        anchor,
        &prover,
        memo,
        None,
        platform_version,
    ) {
        Ok(st) => st,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::CryptoError,
                format!("Failed to build shielded withdrawal: {}", e),
            ))
        }
    };

    match extract_bundle_from_st(&st) {
        Ok(sb) => bundle_result(&sb),
        Err(e) => DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::CryptoError, e)),
    }
}

/// Extract the serialized Orchard bundle from a state transition.
///
/// The DPP builder functions return `StateTransition` objects that contain
/// the bundle fields (actions, anchor, proof, binding_signature) directly
/// in their inner V0 structs. We destructure them to produce a
/// `SerializedBundle` that can be converted to FFI params.
fn extract_bundle_from_st(
    st: &dash_sdk::dpp::state_transition::StateTransition,
) -> Result<dash_sdk::dpp::shielded::builder::SerializedBundle, String> {
    use dash_sdk::dpp::shielded::SerializedAction;
    use dash_sdk::dpp::state_transition::StateTransition;

    // All shielded transition variants store the same bundle fields in
    // their V0 structs. Extract a (actions, anchor, proof, binding_sig) tuple.
    let (actions, anchor, proof, binding_signature): (
        &[SerializedAction],
        [u8; 32],
        &[u8],
        [u8; 64],
    ) = match st {
        StateTransition::ShieldedTransfer(t) => match t {
            dash_sdk::dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition::V0(v0) => {
                (&v0.actions, v0.anchor, &v0.proof, v0.binding_signature)
            }
        },
        StateTransition::Unshield(t) => match t {
            dash_sdk::dpp::state_transition::unshield_transition::UnshieldTransition::V0(v0) => {
                (&v0.actions, v0.anchor, &v0.proof, v0.binding_signature)
            }
        },
        StateTransition::ShieldedWithdrawal(t) => match t {
            dash_sdk::dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition::V0(v0) => {
                (&v0.actions, v0.anchor, &v0.proof, v0.binding_signature)
            }
        },
        _ => return Err("Unexpected state transition type".to_string()),
    };

    // Both SerializedBundle and the transition V0 structs use the same
    // SerializedAction type from dpp::shielded, so we just clone.
    Ok(dash_sdk::dpp::shielded::builder::SerializedBundle {
        actions: actions.to_vec(),
        flags: 0,
        value_balance: 0,
        anchor,
        proof: proof.to_vec(),
        binding_signature,
    })
}
