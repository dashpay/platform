//! Data contract operations
//!
//! The `dash_sdk_data_contract_create` extension that used to live
//! here has moved to `rs-platform-wallet-ffi` (see
//! `platform_wallet_create_data_contract_with_signer`). The new
//! path goes through the platform-wallet runtime, whose worker
//! thread stack (8 MB) is large enough for `rs-drive`'s post-
//! broadcast GroveDB proof recursion. The rs-sdk-ffi runtime uses
//! the iOS default ~512 KB thread stack and reliably crashed with
//! `EXC_BAD_ACCESS` at the `Op::decode` prologue during proof
//! verification — the classic stack-guard fingerprint, not memory
//! corruption.
//!
//! This module now contains:
//!   - `DashSDKDataContractInfo` — public C struct still used by
//!     other rs-sdk-ffi paths (returned by query helpers).
//!   - `dash_sdk_data_contract_destroy` — destructor for handles
//!     produced by the query layer.
//!   - The query function re-exports from `queries::*`.

mod put;
mod queries;
mod util;

use std::os::raw::c_char;

use dash_sdk::dpp::prelude::DataContract;

use crate::types::DataContractHandle;

/// Data contract information
#[repr(C)]
pub struct DashSDKDataContractInfo {
    /// Contract ID as hex string (null-terminated)
    pub id: *mut c_char,
    /// Owner ID as hex string (null-terminated)
    pub owner_id: *mut c_char,
    /// Contract version
    pub version: u32,
    /// Schema version
    pub schema_version: u32,
    /// Number of document types
    pub document_types_count: u32,
}

/// Destroy a data contract handle
///
/// # Safety
/// - `handle` must be a pointer previously returned by this SDK or null (no-op).
/// - After this call, `handle` becomes invalid and must not be used again.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_data_contract_destroy(handle: *mut DataContractHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut DataContract);
    }
}

// Re-export query functions
pub use queries::{
    dash_sdk_data_contract_fetch, dash_sdk_data_contract_fetch_history,
    dash_sdk_data_contract_fetch_json, dash_sdk_data_contracts_fetch_many,
};
