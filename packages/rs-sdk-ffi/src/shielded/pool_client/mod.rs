//! High-level shielded pool client FFI module.
//!
//! Wraps the commitment tree, note tracking, and key management into a single
//! opaque handle so that iOS callers never deal with Merkle paths, anchors, or
//! note selection directly.

mod bundle;
mod sync;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::sync::Mutex;

use dash_sdk::grovedb_commitment_tree::{
    ClientPersistentCommitmentTree, FullViewingKey, IncomingViewingKey, Note, Nullifier,
    PaymentAddress, Position, Scope, SpendAuthorizingKey, SpendingKey,
};
use zeroize::Zeroize;

use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, DashSDKResultDataType};

// Re-export bundle and sync FFI functions.
pub use bundle::*;
pub use sync::*;

/// Orchard key set derived from a spending key.
pub(crate) struct OrchardKeySet {
    pub fvk: FullViewingKey,
    pub ask: SpendAuthorizingKey,
    pub ivk: IncomingViewingKey,
    pub default_address: PaymentAddress,
}

/// A decrypted shielded note owned by the wallet.
pub(crate) struct ShieldedNote {
    pub note: Note,
    pub position: Position,
    pub nullifier: Nullifier,
    pub is_spent: bool,
    pub value: u64,
}

/// Opaque shielded pool client exposed across the FFI boundary.
///
/// Holds derived Orchard keys, the SQLite-backed commitment tree, and all
/// tracked notes. The commitment tree is behind a `Mutex` because
/// `ClientPersistentCommitmentTree` contains a `rusqlite::Connection` which
/// is `!Sync`. The mutex makes the overall struct `Sync` so `&self` is `Send`
/// across `.await` boundaries.
pub struct ShieldedPoolClient {
    pub(crate) keys: OrchardKeySet,
    pub(crate) notes: Vec<ShieldedNote>,
    pub(crate) commitment_tree: Mutex<ClientPersistentCommitmentTree>,
    pub(crate) last_synced_index: u64,
    pub(crate) last_nullifier_sync_height: u64,
    pub(crate) last_nullifier_sync_timestamp: u64,
}

impl ShieldedPoolClient {
    /// Recalculate the shielded balance from unspent notes.
    pub(crate) fn recalculate_balance(&self) -> u64 {
        self.notes
            .iter()
            .filter(|n| !n.is_spent)
            .map(|n| n.value)
            .sum()
    }

    /// Get unspent notes.
    pub(crate) fn unspent_notes(&self) -> Vec<&ShieldedNote> {
        self.notes.iter().filter(|n| !n.is_spent).collect()
    }
}

/// Derive an `OrchardKeySet` from raw spending key bytes.
///
/// The local copy of the key bytes is zeroized after derivation.
fn derive_key_set(sk_bytes: &[u8; 32]) -> Result<OrchardKeySet, String> {
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
    let ivk = fvk.to_ivk(Scope::External);
    let default_address = fvk.address_at(0u32, Scope::External);

    Ok(OrchardKeySet {
        fvk,
        ask,
        ivk,
        default_address,
    })
}

// ---------------------------------------------------------------------------
// FFI functions
// ---------------------------------------------------------------------------

/// Create a shielded pool client backed by SQLite at `db_path`.
///
/// The spending key is used to derive all Orchard keys (FVK, ASK, IVK, address).
/// The commitment tree resumes from its last synced position automatically.
///
/// # Safety
/// - `db_path` must be a valid NUL-terminated C string.
/// - `spending_key_bytes` must point to exactly 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_pool_client_create(
    db_path: *const c_char,
    spending_key_bytes: *const [u8; 32],
) -> DashSDKResult {
    if db_path.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "db_path is null".to_string(),
        ));
    }
    if spending_key_bytes.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "spending_key_bytes is null".to_string(),
        ));
    }

    let path_str = match CStr::from_ptr(db_path).to_str() {
        Ok(s) => s,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("db_path is not valid UTF-8: {}", e),
            ))
        }
    };

    let sk_bytes = &*spending_key_bytes;

    // Derive Orchard keys from the spending key.
    let keys = match derive_key_set(sk_bytes) {
        Ok(k) => k,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::InvalidParameter, e))
        }
    };

    // Open the SQLite-backed commitment tree. Tables are created automatically.
    let commitment_tree = match ClientPersistentCommitmentTree::open_path(path_str, 100) {
        Ok(tree) => tree,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to open commitment tree: {}", e),
            ))
        }
    };

    // Resume from persisted tree state if available.
    let last_synced_index = match commitment_tree.max_leaf_position() {
        Ok(Some(pos)) => u64::from(pos) + 1,
        Ok(None) => 0,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Failed to read tree position: {}", e),
            ))
        }
    };

    let client = ShieldedPoolClient {
        keys,
        notes: Vec::new(),
        commitment_tree: Mutex::new(commitment_tree),
        last_synced_index,
        last_nullifier_sync_height: 0,
        last_nullifier_sync_timestamp: 0,
    };

    let handle = Box::into_raw(Box::new(client));

    DashSDKResult {
        data_type: DashSDKResultDataType::BinaryData,
        data: handle as *mut c_void,
        error: std::ptr::null_mut(),
    }
}

/// Destroy and free a shielded pool client.
///
/// Also closes the SQLite connection used by the commitment tree.
///
/// # Safety
/// - `handle` must be a valid pointer returned by
///   `dash_sdk_shielded_pool_client_create` and not previously freed.
/// - It may be null (no-op).
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_pool_client_destroy(handle: *mut ShieldedPoolClient) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
}

/// Get the default Orchard payment address (hex-encoded 43 bytes).
///
/// # Safety
/// - `handle` must be a valid pointer to a `ShieldedPoolClient`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_pool_client_get_address(
    handle: *const ShieldedPoolClient,
) -> DashSDKResult {
    if handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "handle is null".to_string(),
        ));
    }

    let client = &*handle;
    let raw_bytes = client.keys.default_address.to_raw_address_bytes();
    let hex_str = hex::encode(raw_bytes);

    match CString::new(hex_str) {
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
            format!("Failed to create C string: {}", e),
        )),
    }
}

/// Get the current shielded balance (sum of unspent note values).
///
/// Returns the balance as a decimal string.
///
/// # Safety
/// - `handle` must be a valid pointer to a `ShieldedPoolClient`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_pool_client_get_balance(
    handle: *const ShieldedPoolClient,
) -> DashSDKResult {
    if handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "handle is null".to_string(),
        ));
    }

    let client = &*handle;
    let balance = client.recalculate_balance();
    let balance_str = balance.to_string();

    match CString::new(balance_str) {
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
            format!("Failed to create C string: {}", e),
        )),
    }
}
