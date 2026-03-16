//! Orchard bundle building FFI functions.
//!
//! These functions construct authorized Orchard bundles internally (proof + signatures)
//! and return a heap-allocated `DashSDKOrchardBundleParams` pointer via `DashSDKResult`.
//! Free the returned bundle with `dash_sdk_shielded_bundle_params_free`.
//!
//! Since the DPP builder helpers (`build_output_only_bundle`, `build_spend_bundle`) are
//! `pub(crate)`, we replicate the bundle construction logic here using the public
//! `grovedb_commitment_tree` Builder API and `dpp::shielded` public functions
//! (`serialize_authorized_bundle`, `compute_platform_sighash`).

use std::os::raw::c_void;

use dash_sdk::dpp::identity::core_script::CoreScript;
use dash_sdk::dpp::shielded::builder::{serialize_authorized_bundle, OrchardProver};
use dash_sdk::dpp::shielded::{compute_minimum_shielded_fee, compute_platform_sighash};
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::grovedb_commitment_tree::{
    Anchor, Builder, BundleType, DashMemo, Flags as OrchardFlags, FullViewingKey, Hashable,
    MerkleHashOrchard, MerklePath, Note, NoteValue, PaymentAddress, RandomSeed, Rho, Scope,
    SpendAuthorizingKey, SpendingKey, NOTE_COMMITMENT_TREE_DEPTH,
};
use rand::rngs::OsRng;
use zeroize::Zeroize;

use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};

use super::CachedProver;

// ---------------------------------------------------------------------------
// JSON input/output structures
// ---------------------------------------------------------------------------

/// A spendable note parsed from JSON input.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SpendableNoteJson {
    /// Hex-encoded 43-byte Orchard address.
    address: String,
    /// Note value in credits.
    value: u64,
    /// Hex-encoded 32-byte Rho.
    rho: String,
    /// Hex-encoded 32-byte random seed.
    rseed: String,
    /// Position in the commitment tree.
    position: u32,
    /// Array of 32 hex-encoded 32-byte Merkle path hashes.
    merkle_path: Vec<String>,
}

/// Parsed spendable note with its Merkle path.
struct ParsedSpendableNote {
    note: Note,
    merkle_path: MerklePath,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Convert a `SerializedBundle` into a heap-allocated `DashSDKOrchardBundleParams`.
///
/// The returned pointer owns all its inner allocations (actions array, each
/// action's encrypted_note, and proof bytes). Free with
/// `dash_sdk_shielded_bundle_params_free`.
pub(crate) fn bundle_to_ffi_params(
    sb: &dash_sdk::dpp::shielded::builder::SerializedBundle,
) -> *mut crate::shielded::types::DashSDKOrchardBundleParams {
    use crate::shielded::types::{DashSDKOrchardBundleParams, DashSDKSerializedAction};

    // Build heap-allocated actions with heap-allocated encrypted_note blobs.
    let ffi_actions: Vec<DashSDKSerializedAction> = sb
        .actions
        .iter()
        .map(|a| {
            let enc_note = a.encrypted_note.clone().into_boxed_slice();
            let enc_note_len = enc_note.len();
            let enc_note_ptr = Box::into_raw(enc_note) as *const u8;
            DashSDKSerializedAction {
                nullifier: a.nullifier,
                rk: a.rk,
                cmx: a.cmx,
                encrypted_note: enc_note_ptr,
                encrypted_note_len: enc_note_len,
                cv_net: a.cv_net,
                spend_auth_sig: a.spend_auth_sig,
            }
        })
        .collect();

    let actions_count = ffi_actions.len() as u32;
    let actions_box = ffi_actions.into_boxed_slice();
    let actions_ptr = Box::into_raw(actions_box) as *const DashSDKSerializedAction;

    let proof = sb.proof.clone().into_boxed_slice();
    let proof_len = proof.len();
    let proof_ptr = Box::into_raw(proof) as *const u8;

    let params = DashSDKOrchardBundleParams {
        actions: actions_ptr,
        actions_count,
        anchor: sb.anchor,
        proof: proof_ptr,
        proof_len,
        binding_signature: sb.binding_signature,
    };

    Box::into_raw(Box::new(params))
}

/// Return a DashSDKResult containing a heap-allocated `DashSDKOrchardBundleParams`.
fn bundle_result(sb: &dash_sdk::dpp::shielded::builder::SerializedBundle) -> DashSDKResult {
    let ptr = bundle_to_ffi_params(sb);
    DashSDKResult {
        data_type: DashSDKResultDataType::BinaryData,
        data: ptr as *mut c_void,
        error: std::ptr::null_mut(),
    }
}

/// Free a heap-allocated `DashSDKOrchardBundleParams` and all its inner allocations.
///
/// # Safety
/// - `bundle` must be a pointer returned by one of the `dash_sdk_shielded_build_*_bundle`
///   functions and not previously freed.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_bundle_params_free(
    bundle: *mut crate::shielded::types::DashSDKOrchardBundleParams,
) {
    use crate::shielded::types::DashSDKSerializedAction;

    if bundle.is_null() {
        return;
    }

    let params = Box::from_raw(bundle);

    // Free each action's encrypted_note allocation.
    if !params.actions.is_null() && params.actions_count > 0 {
        let actions_ptr = params.actions as *mut DashSDKSerializedAction;
        let actions_count = params.actions_count as usize;
        let actions_slice = std::slice::from_raw_parts(actions_ptr, actions_count);
        for action in actions_slice.iter() {
            if !action.encrypted_note.is_null() && action.encrypted_note_len > 0 {
                drop(Vec::from_raw_parts(
                    action.encrypted_note as *mut u8,
                    action.encrypted_note_len,
                    action.encrypted_note_len,
                ));
            }
        }
        // Free the actions array itself.
        drop(Vec::from_raw_parts(
            actions_ptr,
            actions_count,
            actions_count,
        ));
    }

    // Free the proof allocation.
    if !params.proof.is_null() && params.proof_len > 0 {
        drop(Vec::from_raw_parts(
            params.proof as *mut u8,
            params.proof_len,
            params.proof_len,
        ));
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

/// Derive FullViewingKey and SpendAuthorizingKey from raw spending key bytes.
/// The local copy of key bytes is zeroized after derivation.
fn derive_keys(sk_bytes: &[u8; 32]) -> Result<(FullViewingKey, SpendAuthorizingKey), String> {
    let mut key_copy = *sk_bytes;
    let sk: SpendingKey = match SpendingKey::from_bytes(key_copy).into_option() {
        Some(sk) => {
            key_copy.zeroize();
            sk
        }
        None => {
            key_copy.zeroize();
            return Err("Invalid spending key bytes".to_string());
        }
    };
    let fvk = FullViewingKey::from(&sk);
    let ask = SpendAuthorizingKey::from(&sk);
    // sk is dropped here; SpendingKey may or may not zeroize on drop
    // but key_copy is already zeroed above
    Ok((fvk, ask))
}

/// Decode a hex string into a fixed-size byte array.
fn hex_to_array<const N: usize>(hex_str: &str, field_name: &str) -> Result<[u8; N], String> {
    let bytes = hex::decode(hex_str)
        .map_err(|e| format!("Failed to decode hex for {}: {}", field_name, e))?;
    bytes.try_into().map_err(|_| {
        format!(
            "{} must be {} bytes, got {}",
            field_name,
            N,
            hex_str.len() / 2
        )
    })
}

/// Parse a JSON string into a vector of `ParsedSpendableNote`.
fn parse_spendable_notes(notes_json: &str) -> Result<Vec<ParsedSpendableNote>, String> {
    let notes: Vec<SpendableNoteJson> = serde_json::from_str(notes_json)
        .map_err(|e| format!("Failed to parse notes JSON: {}", e))?;

    let mut spendable_notes = Vec::with_capacity(notes.len());

    for (i, n) in notes.iter().enumerate() {
        // Parse address to get PaymentAddress (for Note::from_parts)
        let addr_bytes: [u8; 43] = hex_to_array(&n.address, &format!("notes[{}].address", i))?;
        let payment_address = PaymentAddress::from_raw_address_bytes(&addr_bytes)
            .into_option()
            .ok_or_else(|| format!("notes[{}].address is not a valid Orchard address", i))?;

        // Parse Rho
        let rho_bytes: [u8; 32] = hex_to_array(&n.rho, &format!("notes[{}].rho", i))?;
        let rho = Rho::from_bytes(&rho_bytes)
            .into_option()
            .ok_or_else(|| format!("notes[{}].rho is not a valid Rho", i))?;

        // Parse RandomSeed
        let rseed_bytes: [u8; 32] = hex_to_array(&n.rseed, &format!("notes[{}].rseed", i))?;
        let rseed = RandomSeed::from_bytes(rseed_bytes, &rho)
            .into_option()
            .ok_or_else(|| format!("notes[{}].rseed is not a valid RandomSeed", i))?;

        // Construct Note
        let note = Note::from_parts(payment_address, NoteValue::from_raw(n.value), rho, rseed)
            .into_option()
            .ok_or_else(|| format!("notes[{}] failed to construct valid Note", i))?;

        // Parse Merkle path
        if n.merkle_path.len() != NOTE_COMMITMENT_TREE_DEPTH {
            return Err(format!(
                "notes[{}].merklePath must have {} entries, got {}",
                i,
                NOTE_COMMITMENT_TREE_DEPTH,
                n.merkle_path.len()
            ));
        }

        let mut auth_path = [MerkleHashOrchard::empty_leaf(); NOTE_COMMITMENT_TREE_DEPTH];
        for (j, hash_hex) in n.merkle_path.iter().enumerate() {
            let hash_bytes: [u8; 32] =
                hex_to_array(hash_hex, &format!("notes[{}].merklePath[{}]", i, j))?;
            auth_path[j] = MerkleHashOrchard::from_bytes(&hash_bytes)
                .into_option()
                .ok_or_else(|| {
                    format!(
                        "notes[{}].merklePath[{}] is not a valid MerkleHashOrchard",
                        i, j
                    )
                })?;
        }

        let merkle_path = MerklePath::from_parts(n.position, auth_path);

        spendable_notes.push(ParsedSpendableNote { note, merkle_path });
    }

    Ok(spendable_notes)
}

/// Parse an anchor from 32-byte pointer.
unsafe fn parse_anchor(anchor_bytes: *const [u8; 32]) -> Result<Anchor, String> {
    if anchor_bytes.is_null() {
        return Err("anchor_bytes is null".to_string());
    }
    let bytes = &*anchor_bytes;
    Anchor::from_bytes(*bytes)
        .into_option()
        .ok_or_else(|| "Invalid anchor bytes".to_string())
}

/// Parse a C string into a Rust &str.
unsafe fn parse_c_str<'a>(ptr: *const std::os::raw::c_char, name: &str) -> Result<&'a str, String> {
    if ptr.is_null() {
        return Err(format!("{} is null", name));
    }
    std::ffi::CStr::from_ptr(ptr)
        .to_str()
        .map_err(|e| format!("{} is not valid UTF-8: {}", name, e))
}

/// Build an output-only Orchard bundle (no spends). Replicates the logic from
/// `dpp::shielded::builder::build_output_only_bundle` which is `pub(crate)`.
fn build_output_only_bundle_local(
    recipient: &PaymentAddress,
    amount: u64,
    memo: [u8; 36],
    prover: &CachedProver,
) -> Result<dash_sdk::dpp::shielded::builder::SerializedBundle, String> {
    let anchor = Anchor::empty_tree();
    let mut builder = Builder::<DashMemo>::new(
        BundleType::Transactional {
            flags: OrchardFlags::SPENDS_DISABLED,
            bundle_required: false,
        },
        anchor,
    );

    builder
        .add_output(None, *recipient, NoteValue::from_raw(amount), memo)
        .map_err(|e| format!("failed to add output: {:?}", e))?;

    let bundle = prove_and_sign_bundle_local(builder, prover, &[], &[])?;
    Ok(serialize_authorized_bundle(&bundle))
}

/// An optional change output to add to the bundle.
struct ChangeOutput {
    address: PaymentAddress,
    amount: u64,
}

/// Build a spend+output Orchard bundle. Replicates the logic from
/// `dpp::shielded::builder::build_spend_bundle` which is `pub(crate)`.
///
/// If `change` is `Some`, a second output is added returning change to the sender.
#[allow(clippy::too_many_arguments)]
fn build_spend_bundle_local(
    spends: Vec<ParsedSpendableNote>,
    output_address: &PaymentAddress,
    output_amount: u64,
    memo: [u8; 36],
    change: Option<ChangeOutput>,
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    prover: &CachedProver,
    extra_sighash_data: &[u8],
) -> Result<dash_sdk::dpp::shielded::builder::SerializedBundle, String> {
    let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);

    for spend in spends {
        builder
            .add_spend(fvk.clone(), spend.note, spend.merkle_path)
            .map_err(|e| format!("failed to add spend: {:?}", e))?;
    }

    // Primary output
    builder
        .add_output(
            None,
            *output_address,
            NoteValue::from_raw(output_amount),
            memo,
        )
        .map_err(|e| format!("failed to add output: {:?}", e))?;

    // Change output (if any)
    if let Some(ch) = change {
        if ch.amount > 0 {
            builder
                .add_output(
                    None,
                    ch.address,
                    NoteValue::from_raw(ch.amount),
                    [0u8; 36], // change memo is always empty
                )
                .map_err(|e| format!("failed to add change output: {:?}", e))?;
        }
    }

    let bundle = prove_and_sign_bundle_local(
        builder,
        prover,
        std::slice::from_ref(ask),
        extra_sighash_data,
    )?;
    Ok(serialize_authorized_bundle(&bundle))
}

/// Prove and sign an Orchard bundle. Replicates the logic from
/// `dpp::shielded::builder::prove_and_sign_bundle` which is `pub(crate)`.
fn prove_and_sign_bundle_local(
    builder: Builder<DashMemo>,
    prover: &CachedProver,
    signing_keys: &[SpendAuthorizingKey],
    extra_sighash_data: &[u8],
) -> Result<
    dash_sdk::grovedb_commitment_tree::Bundle<
        dash_sdk::grovedb_commitment_tree::Authorized,
        i64,
        DashMemo,
    >,
    String,
> {
    let mut rng = OsRng;

    let (unauthorized, _) = builder
        .build::<i64>(&mut rng)
        .map_err(|e| format!("failed to build bundle: {:?}", e))?
        .ok_or_else(|| "bundle was empty after build".to_string())?;

    let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
    let sighash = compute_platform_sighash(&bundle_commitment, extra_sighash_data);

    let proven = unauthorized
        .create_proof(prover.proving_key(), &mut rng)
        .map_err(|e| format!("failed to create proof: {:?}", e))?;

    proven
        .apply_signatures(rng, sighash, signing_keys)
        .map_err(|e| format!("failed to apply signatures: {:?}", e))
}

// ---------------------------------------------------------------------------
// FFI functions
// ---------------------------------------------------------------------------

/// Build an output-only (shield) Orchard bundle.
///
/// This is the simplest bundle type: it creates a new note for the recipient
/// with no spends. Used when shielding transparent platform credits.
///
/// # Parameters
/// - `spending_key_bytes`: 32-byte spending key (recipient address derived at index 0).
/// - `amount`: Amount in credits to shield.
/// - `memo`: Optional 36-byte memo (null for zero memo).
///
/// # Returns
/// JSON string with the serialized bundle (see module docs for format).
///
/// # Safety
/// - `spending_key_bytes` must point to exactly 32 bytes.
/// - `memo`, if non-null, must point to exactly 36 bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_build_shield_bundle(
    spending_key_bytes: *const [u8; 32],
    amount: u64,
    memo: *const [u8; 36],
) -> DashSDKResult {
    if spending_key_bytes.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "spending_key_bytes is null".to_string(),
        ));
    }

    if amount == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "shield amount must be greater than zero".to_string(),
        ));
    }

    let sk_bytes = &*spending_key_bytes;
    let memo = parse_memo(memo);

    let (fvk, _ask) = match derive_keys(sk_bytes) {
        Ok(keys) => keys,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    let recipient = fvk.address_at(0u32, Scope::External);
    let prover = CachedProver;

    let sb = match build_output_only_bundle_local(&recipient, amount, memo, &prover) {
        Ok(sb) => sb,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::CryptoError,
                format!("Failed to build shield bundle: {}", e),
            ))
        }
    };

    bundle_result(&sb)
}

/// Build a shielded transfer bundle (shielded-to-shielded).
///
/// Spends existing notes and creates a new note for the recipient. Change
/// (if any) is returned to the sender's own address (derived at index 0).
///
/// # Parameters
/// - `spending_key_bytes`: 32-byte spending key of the sender.
/// - `anchor_bytes`: 32-byte Sinsemilla anchor of the commitment tree.
/// - `notes_json`: JSON array of spendable notes (see module docs for format).
/// - `recipient_addr_bytes`: Raw 43-byte Orchard address of the recipient.
/// - `recipient_addr_len`: Length of recipient address (must be 43).
/// - `transfer_amount`: Amount in credits to transfer to the recipient.
/// - `memo`: Optional 36-byte memo (null for zero memo).
///
/// # Returns
/// JSON string with the serialized bundle.
///
/// # Safety
/// - All pointers must be valid for their specified lengths.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_build_transfer_bundle(
    spending_key_bytes: *const [u8; 32],
    anchor_bytes: *const [u8; 32],
    notes_json: *const std::os::raw::c_char,
    recipient_addr_bytes: *const u8,
    recipient_addr_len: usize,
    transfer_amount: u64,
    memo: *const [u8; 36],
) -> DashSDKResult {
    if spending_key_bytes.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "spending_key_bytes is null".to_string(),
        ));
    }

    let sk_bytes = &*spending_key_bytes;
    let memo = parse_memo(memo);

    let anchor = match parse_anchor(anchor_bytes) {
        Ok(a) => a,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    let notes_str = match parse_c_str(notes_json, "notes_json") {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    let spends = match parse_spendable_notes(notes_str) {
        Ok(n) => n,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    // Parse recipient address
    if recipient_addr_bytes.is_null() || recipient_addr_len != 43 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "recipient address must be 43 bytes, got {}",
                if recipient_addr_bytes.is_null() {
                    0
                } else {
                    recipient_addr_len
                }
            ),
        ));
    }
    let addr_slice = std::slice::from_raw_parts(recipient_addr_bytes, 43);
    let mut addr_array = [0u8; 43];
    addr_array.copy_from_slice(addr_slice);
    let recipient_payment = match PaymentAddress::from_raw_address_bytes(&addr_array).into_option()
    {
        Some(a) => a,
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Invalid recipient address: not a valid Pallas curve point".to_string(),
            ))
        }
    };

    let (fvk, ask) = match derive_keys(sk_bytes) {
        Ok(keys) => keys,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    let prover = CachedProver;
    let platform_version = PlatformVersion::latest();

    // Compute total spent value with overflow check
    let total_spent: u64 = match spends
        .iter()
        .try_fold(0u64, |acc, s| acc.checked_add(s.note.value().inner()))
    {
        Some(v) => v,
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "total note values overflow u64".to_string(),
            ))
        }
    };

    // Try with no-change fee first (recipient output only). If there's surplus
    // after that, recompute with change output included.
    let no_change_actions = spends.len().max(1);
    let no_change_fee = compute_minimum_shielded_fee(no_change_actions, platform_version);
    let no_change_required = match transfer_amount.checked_add(no_change_fee) {
        Some(v) => v,
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "transfer amount + fee overflows u64".to_string(),
            ))
        }
    };

    let (min_fee, change_amount) = if no_change_required == total_spent {
        // Exact match — no change output needed
        (no_change_fee, 0u64)
    } else if no_change_required > total_spent {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "transfer amount {} + fee {} = {} exceeds total spendable value {}",
                transfer_amount, no_change_fee, no_change_required, total_spent
            ),
        ));
    } else {
        // Surplus exists — recompute with change output (adds an action)
        let with_change_actions = spends.len().max(2);
        let with_change_fee = compute_minimum_shielded_fee(with_change_actions, platform_version);
        let with_change_required = match transfer_amount.checked_add(with_change_fee) {
            Some(v) => v,
            None => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    "transfer amount + fee overflows u64".to_string(),
                ))
            }
        };
        if with_change_required > total_spent {
            // Can't afford change output fee — use no-change fee and burn the dust
            (no_change_fee, 0u64)
        } else {
            (with_change_fee, total_spent - with_change_required)
        }
    };

    // Validate required <= i64::MAX (Orchard value_balance is i64)
    let required = transfer_amount + min_fee + change_amount;
    debug_assert_eq!(required, total_spent - change_amount + change_amount); // sanity
    let value_balance = min_fee; // only fee leaves the shielded pool
    if value_balance > i64::MAX as u64 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "fee {} exceeds i64::MAX, cannot represent as Orchard value_balance",
                value_balance
            ),
        ));
    }

    // Change goes back to sender's address (derived at index 0)
    let change_output = if change_amount > 0 {
        let change_address = fvk.address_at(0u32, Scope::External);
        Some(ChangeOutput {
            address: change_address,
            amount: change_amount,
        })
    } else {
        None
    };

    // ShieldedTransfer: recipient gets transfer_amount, sender gets change, fee leaves pool.
    // No extra sighash data for transfers.
    let sb = match build_spend_bundle_local(
        spends,
        &recipient_payment,
        transfer_amount,
        memo,
        change_output,
        &fvk,
        &ask,
        anchor,
        &prover,
        &[],
    ) {
        Ok(sb) => sb,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::CryptoError,
                format!("Failed to build transfer bundle: {}", e),
            ))
        }
    };

    bundle_result(&sb)
}

/// Build an unshield bundle (shielded pool -> platform address).
///
/// Spends existing notes and creates a change output back to the sender.
/// The `output_address` receives funds via the state transition's transparent field.
///
/// # Parameters
/// - `spending_key_bytes`: 32-byte spending key.
/// - `anchor_bytes`: 32-byte Sinsemilla anchor.
/// - `notes_json`: JSON array of spendable notes.
/// - `output_addr_bytes`: Platform address bytes for the unshield recipient.
/// - `output_addr_len`: Length of `output_addr_bytes`.
/// - `unshield_amount`: Amount to unshield.
/// - `memo`: Optional 36-byte memo (null for zero memo).
///
/// # Returns
/// JSON string with the serialized bundle.
///
/// # Safety
/// - All pointers must be valid for their specified lengths.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_build_unshield_bundle(
    spending_key_bytes: *const [u8; 32],
    anchor_bytes: *const [u8; 32],
    notes_json: *const std::os::raw::c_char,
    output_addr_bytes: *const u8,
    output_addr_len: usize,
    unshield_amount: u64,
    memo: *const [u8; 36],
) -> DashSDKResult {
    if spending_key_bytes.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "spending_key_bytes is null".to_string(),
        ));
    }

    let sk_bytes = &*spending_key_bytes;
    let memo = parse_memo(memo);

    let anchor = match parse_anchor(anchor_bytes) {
        Ok(a) => a,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    let notes_str = match parse_c_str(notes_json, "notes_json") {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    let spends = match parse_spendable_notes(notes_str) {
        Ok(n) => n,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    // Parse output address bytes
    if output_addr_bytes.is_null() || output_addr_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "output_addr_bytes is null or empty".to_string(),
        ));
    }
    let addr_slice = std::slice::from_raw_parts(output_addr_bytes, output_addr_len);
    let output_address = match dash_sdk::dpp::address_funds::PlatformAddress::from_bytes(addr_slice)
    {
        Ok(a) => a,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid output address: {}", e),
            ))
        }
    };

    let (fvk, ask) = match derive_keys(sk_bytes) {
        Ok(keys) => keys,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    if unshield_amount > i64::MAX as u64 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "unshield amount {} exceeds maximum allowed value {}",
                unshield_amount,
                i64::MAX as u64
            ),
        ));
    }

    let change_payment = fvk.address_at(0u32, Scope::External);
    let prover = CachedProver;
    let platform_version = PlatformVersion::latest();

    // Compute total spent value with overflow check
    let total_spent: u64 = match spends
        .iter()
        .try_fold(0u64, |acc, s| acc.checked_add(s.note.value().inner()))
    {
        Some(v) => v,
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "total note values overflow u64".to_string(),
            ))
        }
    };

    // Compute minimum fee
    let num_actions = spends.len().max(1);
    let min_fee = compute_minimum_shielded_fee(num_actions, platform_version);

    // Validate sufficient funds for unshield + fee
    let required = match unshield_amount.checked_add(min_fee) {
        Some(v) => v,
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "unshield amount + fee overflows u64".to_string(),
            ))
        }
    };
    if required > i64::MAX as u64 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "unshield amount {} + fee {} = {} exceeds i64::MAX",
                unshield_amount, min_fee, required
            ),
        ));
    }
    if required > total_spent {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "unshield amount {} + fee {} = {} exceeds total spendable value {}",
                unshield_amount, min_fee, required, total_spent
            ),
        ));
    }

    let change_amount = total_spent - required;

    // Unshield extra_data = output_address.to_bytes()
    let extra_sighash_data = output_address.to_bytes();

    let sb = match build_spend_bundle_local(
        spends,
        &change_payment,
        change_amount,
        memo,
        None, // no second output — the unshield amount goes via transparent field
        &fvk,
        &ask,
        anchor,
        &prover,
        &extra_sighash_data,
    ) {
        Ok(sb) => sb,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::CryptoError,
                format!("Failed to build unshield bundle: {}", e),
            ))
        }
    };

    bundle_result(&sb)
}

/// Build a withdrawal bundle (shielded pool -> core L1 address).
///
/// Spends existing notes and creates a change output back to the sender.
/// The `output_script` receives funds via the state transition's withdrawal mechanism.
///
/// # Parameters
/// - `spending_key_bytes`: 32-byte spending key.
/// - `anchor_bytes`: 32-byte Sinsemilla anchor.
/// - `notes_json`: JSON array of spendable notes.
/// - `output_script`: Core chain script bytes.
/// - `output_script_len`: Length of `output_script`.
/// - `withdrawal_amount`: Amount to withdraw.
/// - `memo`: Optional 36-byte memo (null for zero memo).
/// - `core_fee_per_byte`: Core chain fee rate (unused in bundle, included for API consistency).
/// - `pooling`: Withdrawal pooling strategy (0=Never, 1=IfAvailable, 2=Standard; unused in bundle).
///
/// # Returns
/// JSON string with the serialized bundle.
///
/// # Safety
/// - All pointers must be valid for their specified lengths.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn dash_sdk_shielded_build_withdrawal_bundle(
    spending_key_bytes: *const [u8; 32],
    anchor_bytes: *const [u8; 32],
    notes_json: *const std::os::raw::c_char,
    output_script: *const u8,
    output_script_len: usize,
    withdrawal_amount: u64,
    memo: *const [u8; 36],
    _core_fee_per_byte: u32,
    _pooling: u8,
) -> DashSDKResult {
    if spending_key_bytes.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "spending_key_bytes is null".to_string(),
        ));
    }

    let sk_bytes = &*spending_key_bytes;
    let memo = parse_memo(memo);

    let anchor = match parse_anchor(anchor_bytes) {
        Ok(a) => a,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    let notes_str = match parse_c_str(notes_json, "notes_json") {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    let spends = match parse_spendable_notes(notes_str) {
        Ok(n) => n,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    // Parse output script
    if output_script.is_null() || output_script_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "output_script is null or empty".to_string(),
        ));
    }
    let script_bytes = std::slice::from_raw_parts(output_script, output_script_len);
    let core_script = CoreScript::from_bytes(script_bytes.to_vec());

    let (fvk, ask) = match derive_keys(sk_bytes) {
        Ok(keys) => keys,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    if withdrawal_amount > i64::MAX as u64 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "withdrawal amount {} exceeds maximum allowed value {}",
                withdrawal_amount,
                i64::MAX as u64
            ),
        ));
    }

    let change_payment = fvk.address_at(0u32, Scope::External);
    let prover = CachedProver;
    let platform_version = PlatformVersion::latest();

    // Compute total spent value with overflow check
    let total_spent: u64 = match spends
        .iter()
        .try_fold(0u64, |acc, s| acc.checked_add(s.note.value().inner()))
    {
        Some(v) => v,
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "total note values overflow u64".to_string(),
            ))
        }
    };

    // Compute minimum fee
    let num_actions = spends.len().max(1);
    let min_fee = compute_minimum_shielded_fee(num_actions, platform_version);

    // Validate sufficient funds for withdrawal + fee
    let required = match withdrawal_amount.checked_add(min_fee) {
        Some(v) => v,
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "withdrawal amount + fee overflows u64".to_string(),
            ))
        }
    };
    if required > i64::MAX as u64 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "withdrawal amount {} + fee {} = {} exceeds i64::MAX",
                withdrawal_amount, min_fee, required
            ),
        ));
    }
    if required > total_spent {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            format!(
                "withdrawal amount {} + fee {} = {} exceeds total spendable value {}",
                withdrawal_amount, min_fee, required, total_spent
            ),
        ));
    }

    let change_amount = total_spent - required;

    // ShieldedWithdrawal extra_data = output_script.as_bytes()
    let extra_sighash_data = core_script.as_bytes().to_vec();

    let sb = match build_spend_bundle_local(
        spends,
        &change_payment,
        change_amount,
        memo,
        None, // no second output — withdrawal goes via transparent field
        &fvk,
        &ask,
        anchor,
        &prover,
        &extra_sighash_data,
    ) {
        Ok(sb) => sb,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::CryptoError,
                format!("Failed to build withdrawal bundle: {}", e),
            ))
        }
    };

    bundle_result(&sb)
}
