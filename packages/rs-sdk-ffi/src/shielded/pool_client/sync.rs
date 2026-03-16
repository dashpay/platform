//! Note sync and nullifier sync FFI functions for the shielded pool client.

use std::ffi::CString;
use std::os::raw::c_void;

use dash_sdk::grovedb_commitment_tree::{Position, Retention};
use dash_sdk::platform::shielded::nullifier_sync::NullifierSyncCheckpoint;
use dash_sdk::platform::shielded::sync_shielded_notes;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};

use super::{ShieldedNote, ShieldedPoolClient};

/// Server-enforced chunk size -- start_index must be a multiple of this.
const CHUNK_SIZE: u64 = 2048;

/// Sync notes from the platform.
///
/// Fetches encrypted notes, trial-decrypts with IVK, appends ALL commitments
/// to the commitment tree, and tracks owned notes.
///
/// Returns JSON: `{ "newNotes": <count>, "balance": <u64> }`
///
/// # Safety
/// - `handle` must be a valid pointer to a `ShieldedPoolClient`.
/// - `sdk_handle` must be a valid pointer to an `SDKHandle`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_pool_client_sync_notes(
    handle: *mut ShieldedPoolClient,
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    if handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "handle is null".to_string(),
        ));
    }
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "sdk_handle is null".to_string(),
        ));
    }

    let client = &mut *handle;
    let wrapper = &*(sdk_handle as *const SDKWrapper);

    // Prepare the incoming viewing key for trial decryption.
    let prepared_ivk = client.keys.ivk.prepare();

    // Round down to nearest chunk boundary so the server accepts the query.
    let already_have = client.last_synced_index;
    let aligned_start = (already_have / CHUNK_SIZE) * CHUNK_SIZE;

    // Execute the async note sync on the SDK runtime.
    let result = wrapper.runtime.block_on(async {
        sync_shielded_notes(&wrapper.sdk, &prepared_ivk, aligned_start, None).await
    });

    let result = match result {
        Ok(r) => r,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to sync shielded notes: {}", e),
            ))
        }
    };

    // Append notes to the local commitment tree, skipping positions already present.
    let mut appended = 0u32;
    for (i, raw_note) in result.all_notes.iter().enumerate() {
        let global_pos = aligned_start + i as u64;
        if global_pos < already_have {
            continue; // already appended in a previous sync
        }

        let cmx_bytes: [u8; 32] = match raw_note.cmx.as_slice().try_into() {
            Ok(b) => b,
            Err(_) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InternalError,
                    "Invalid cmx length".to_string(),
                ))
            }
        };

        let is_ours = result
            .decrypted_notes
            .iter()
            .any(|dn| dn.position == global_pos);
        let retention = if is_ours {
            Retention::Marked
        } else {
            Retention::Ephemeral
        };

        let tree_result = client
            .commitment_tree
            .lock()
            .unwrap()
            .append(cmx_bytes, retention);

        if let Err(e) = tree_result {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to append note to tree: {}", e),
            ));
        }

        appended += 1;
    }

    // Checkpoint the tree if we appended anything.
    if appended > 0 {
        let checkpoint_id = result.next_start_index as u32;
        if let Err(e) = client
            .commitment_tree
            .lock()
            .unwrap()
            .checkpoint(checkpoint_id)
        {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to checkpoint tree: {}", e),
            ));
        }
    }

    // Track newly decrypted notes (skipping positions already stored).
    let mut new_note_count = 0u32;
    for dn in &result.decrypted_notes {
        if dn.position < already_have {
            continue; // already stored in a previous sync
        }

        // Compute the spending nullifier from our FVK.
        let nullifier = dn.note.nullifier(&client.keys.fvk);
        let value = dn.note.value().inner();

        client.notes.push(ShieldedNote {
            note: dn.note,
            position: Position::from(dn.position),
            nullifier,
            is_spent: false,
            value,
        });

        new_note_count += 1;
    }

    // Update sync position.
    client.last_synced_index = aligned_start + result.total_notes_scanned;

    let balance = client.recalculate_balance();

    // Return JSON result.
    let json = serde_json::json!({
        "newNotes": new_note_count,
        "balance": balance,
    });

    match CString::new(json.to_string()) {
        Ok(c_str) => {
            let ptr = c_str.into_raw();
            DashSDKResult {
                data_type: DashSDKResultDataType::String,
                data: ptr as *mut c_void,
                error: std::ptr::null_mut(),
            }
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("Failed to create JSON string: {}", e),
        )),
    }
}

/// Check which owned notes have been spent on-chain using nullifier sync.
///
/// Returns JSON: `{ "spentCount": <count>, "balance": <u64> }`
///
/// # Safety
/// - `handle` must be a valid pointer to a `ShieldedPoolClient`.
/// - `sdk_handle` must be a valid pointer to an `SDKHandle`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_pool_client_sync_nullifiers(
    handle: *mut ShieldedPoolClient,
    sdk_handle: *const SDKHandle,
) -> DashSDKResult {
    if handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "handle is null".to_string(),
        ));
    }
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "sdk_handle is null".to_string(),
        ));
    }

    let client = &mut *handle;
    let wrapper = &*(sdk_handle as *const SDKWrapper);

    // Collect unspent nullifier bytes.
    let unspent_nullifiers: Vec<[u8; 32]> = client
        .notes
        .iter()
        .filter(|n| !n.is_spent)
        .map(|n| n.nullifier.to_bytes())
        .collect();

    if unspent_nullifiers.is_empty() {
        let json = serde_json::json!({
            "spentCount": 0,
            "balance": client.recalculate_balance(),
        });
        return match CString::new(json.to_string()) {
            Ok(c_str) => {
                let ptr = c_str.into_raw();
                DashSDKResult {
                    data_type: DashSDKResultDataType::String,
                    data: ptr as *mut c_void,
                    error: std::ptr::null_mut(),
                }
            }
            Err(e) => DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to create JSON string: {}", e),
            )),
        };
    }

    // Build checkpoint from stored sync state.
    let last_sync = if client.last_nullifier_sync_height > 0 {
        Some(NullifierSyncCheckpoint {
            height: client.last_nullifier_sync_height,
            timestamp: client.last_nullifier_sync_timestamp,
        })
    } else {
        None
    };

    // Execute the async nullifier sync on the SDK runtime.
    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .sync_nullifiers(&unspent_nullifiers, None, last_sync)
            .await
    });

    let result = match result {
        Ok(r) => r,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Nullifier sync failed: {}", e),
            ))
        }
    };

    // Mark found (spent) nullifiers.
    let mut spent_count = 0u32;
    for nf_bytes in &result.found {
        for note in &mut client.notes {
            if !note.is_spent && note.nullifier.to_bytes() == *nf_bytes {
                note.is_spent = true;
                spent_count += 1;
            }
        }
    }

    // Update sync checkpoint.
    client.last_nullifier_sync_height = result.new_sync_height;
    client.last_nullifier_sync_timestamp = result.new_sync_timestamp;

    let balance = client.recalculate_balance();

    // Return JSON result.
    let json = serde_json::json!({
        "spentCount": spent_count,
        "balance": balance,
    });

    match CString::new(json.to_string()) {
        Ok(c_str) => {
            let ptr = c_str.into_raw();
            DashSDKResult {
                data_type: DashSDKResultDataType::String,
                data: ptr as *mut c_void,
                error: std::ptr::null_mut(),
            }
        }
        Err(e) => DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InternalError,
            format!("Failed to create JSON string: {}", e),
        )),
    }
}
