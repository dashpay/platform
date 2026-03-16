//! Orchard note trial decryption FFI function.

use std::ffi::CString;
use std::os::raw::c_void;

use dash_sdk::grovedb_commitment_tree::{
    FullViewingKey, PreparedIncomingViewingKey, Scope, SpendingKey,
};
use drive_proof_verifier::types::ShieldedEncryptedNote;

use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};

/// Encrypted note parsed from JSON input.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptedNoteJson {
    /// Hex-encoded 32-byte note commitment.
    cmx: String,
    /// Hex-encoded 32-byte nullifier.
    nullifier: String,
    /// Hex-encoded 216-byte encrypted note data.
    encrypted_note: String,
}

/// Decrypted note for JSON output.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DecryptedNoteJson {
    /// Position index of this note in the input array.
    position: usize,
    /// Note value in credits.
    value: u64,
    /// Hex-encoded 32-byte nullifier.
    nullifier: String,
    /// Hex-encoded 32-byte note commitment.
    cmx: String,
    /// Hex-encoded 43-byte Orchard address.
    address: String,
    /// Hex-encoded 32-byte Rho.
    rho: String,
    /// Hex-encoded 32-byte random seed.
    rseed: String,
}

/// Try to decrypt encrypted notes using the spending key's incoming viewing key.
///
/// Performs compact trial decryption on each note. Only notes that successfully
/// decrypt (i.e., belong to the viewer) are included in the result.
///
/// # Parameters
/// - `spending_key_bytes`: 32-byte spending key.
/// - `notes_json`: JSON array of encrypted notes:
///   `[{ "cmx": "hex32", "nullifier": "hex32", "encryptedNote": "hex216" }]`
///
/// # Returns
/// JSON array of successfully decrypted notes:
/// ```json
/// [{ "position": idx, "value": u64, "nullifier": "hex32", "cmx": "hex32",
///    "address": "hex43", "rho": "hex32", "rseed": "hex32" }]
/// ```
///
/// # Safety
/// - `spending_key_bytes` must point to exactly 32 bytes.
/// - `notes_json` must be a valid null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_decrypt_notes(
    spending_key_bytes: *const [u8; 32],
    notes_json: *const std::os::raw::c_char,
) -> DashSDKResult {
    if spending_key_bytes.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "spending_key_bytes is null".to_string(),
        ));
    }

    if notes_json.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "notes_json is null".to_string(),
        ));
    }

    let sk_bytes = &*spending_key_bytes;

    // Derive incoming viewing key
    let sk = match SpendingKey::from_bytes(*sk_bytes).into_option() {
        Some(sk) => sk,
        None => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Invalid spending key bytes".to_string(),
            ));
        }
    };
    let fvk = FullViewingKey::from(&sk);
    let ivk = PreparedIncomingViewingKey::new(&fvk.to_ivk(Scope::External));

    // Parse JSON input
    let notes_str = match std::ffi::CStr::from_ptr(notes_json).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("notes_json is not valid UTF-8: {}", e),
            ));
        }
    };

    let encrypted_notes: Vec<EncryptedNoteJson> = match serde_json::from_str(notes_str) {
        Ok(n) => n,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Failed to parse notes JSON: {}", e),
            ));
        }
    };

    let mut decrypted = Vec::new();

    for (idx, enc) in encrypted_notes.iter().enumerate() {
        // Decode hex fields — skip malformed entries with a debug log
        let cmx_bytes = match hex::decode(&enc.cmx) {
            Ok(b) => b,
            Err(e) => {
                tracing::debug!("Skipping note {}: invalid cmx hex: {}", idx, e);
                continue;
            }
        };
        let nf_bytes = match hex::decode(&enc.nullifier) {
            Ok(b) => b,
            Err(e) => {
                tracing::debug!("Skipping note {}: invalid nullifier hex: {}", idx, e);
                continue;
            }
        };
        let enc_note_bytes = match hex::decode(&enc.encrypted_note) {
            Ok(b) => b,
            Err(e) => {
                tracing::debug!("Skipping note {}: invalid encrypted_note hex: {}", idx, e);
                continue;
            }
        };

        let shielded_note = ShieldedEncryptedNote {
            cmx: cmx_bytes,
            nullifier: nf_bytes,
            encrypted_note: enc_note_bytes,
        };

        // Try trial decryption using the SDK's decrypt function
        if let Some((note, address)) =
            dash_sdk::platform::shielded::try_decrypt_note(&ivk, &shielded_note)
        {
            let rho_bytes = note.rho().to_bytes();
            let rseed_bytes = note.rseed().as_bytes();
            let addr_bytes = address.to_raw_address_bytes();

            decrypted.push(DecryptedNoteJson {
                position: idx,
                value: note.value().inner(),
                nullifier: enc.nullifier.clone(),
                cmx: enc.cmx.clone(),
                address: hex::encode(addr_bytes),
                rho: hex::encode(rho_bytes),
                rseed: hex::encode(rseed_bytes),
            });
        }
    }

    let json_str = match serde_json::to_string(&decrypted) {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to serialize decrypted notes: {}", e),
            ));
        }
    };

    match CString::new(json_str) {
        Ok(c_str) => DashSDKResult {
            data_type: DashSDKResultDataType::String,
            data: c_str.into_raw() as *mut c_void,
            error: std::ptr::null_mut(),
        },
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("Failed to create CString: {}", e),
        )),
    }
}
