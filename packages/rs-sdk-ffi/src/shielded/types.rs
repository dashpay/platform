//! Shared FFI types for shielded pool operations.

use crate::FFIError;
use dash_sdk::dpp::shielded::{OrchardBundleParams, SerializedAction};

/// A serialized Orchard action for FFI.
#[repr(C)]
pub struct DashSDKSerializedAction {
    pub nullifier: [u8; 32],
    pub rk: [u8; 32],
    pub cmx: [u8; 32],
    pub encrypted_note: *const u8,
    pub encrypted_note_len: usize,
    pub cv_net: [u8; 32],
    pub spend_auth_sig: [u8; 64],
}

/// Orchard bundle parameters for FFI.
///
/// Shared across all 5 shielded transition types. The client constructs the
/// Orchard bundle (proof, signatures) independently and passes the raw bytes.
#[repr(C)]
pub struct DashSDKOrchardBundleParams {
    pub actions: *const DashSDKSerializedAction,
    pub actions_count: u32,
    pub anchor: [u8; 32],
    pub proof: *const u8,
    pub proof_len: usize,
    pub binding_signature: [u8; 64],
}

/// Convert FFI Orchard bundle params to Rust `OrchardBundleParams`.
///
/// # Safety
/// All pointers in the FFI struct must be valid for the specified lengths.
pub unsafe fn convert_orchard_bundle_params(
    ffi: &DashSDKOrchardBundleParams,
) -> Result<OrchardBundleParams, FFIError> {
    if ffi.actions.is_null() && ffi.actions_count > 0 {
        return Err(FFIError::InvalidParameter(
            "Orchard bundle actions pointer is null".to_string(),
        ));
    }

    if ffi.proof.is_null() && ffi.proof_len > 0 {
        return Err(FFIError::InvalidParameter(
            "Orchard bundle proof pointer is null".to_string(),
        ));
    }

    let mut actions = Vec::with_capacity(ffi.actions_count as usize);
    if ffi.actions_count > 0 {
        let actions_slice = std::slice::from_raw_parts(ffi.actions, ffi.actions_count as usize);
        for (i, action) in actions_slice.iter().enumerate() {
            if action.encrypted_note.is_null() && action.encrypted_note_len > 0 {
                return Err(FFIError::InvalidParameter(format!(
                    "Action {} encrypted_note pointer is null",
                    i
                )));
            }

            let encrypted_note = if action.encrypted_note_len > 0 {
                std::slice::from_raw_parts(action.encrypted_note, action.encrypted_note_len)
                    .to_vec()
            } else {
                Vec::new()
            };

            actions.push(SerializedAction {
                nullifier: action.nullifier,
                rk: action.rk,
                cmx: action.cmx,
                encrypted_note,
                cv_net: action.cv_net,
                spend_auth_sig: action.spend_auth_sig,
            });
        }
    }

    let proof = if ffi.proof_len > 0 {
        std::slice::from_raw_parts(ffi.proof, ffi.proof_len).to_vec()
    } else {
        Vec::new()
    };

    Ok(OrchardBundleParams {
        actions,
        anchor: ffi.anchor,
        proof,
        binding_signature: ffi.binding_signature,
    })
}
