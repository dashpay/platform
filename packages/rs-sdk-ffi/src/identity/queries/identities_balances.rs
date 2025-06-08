//! Multiple identities balance query operations

use dash_sdk::dpp::prelude::Identifier;
use dash_sdk::platform::FetchMany;
use dash_sdk::query_types::IdentityBalance;
use dash_sdk::query_types::IdentityBalances;

use crate::sdk::SDKWrapper;
use crate::types::{DashSDKIdentityBalanceEntry, DashSDKIdentityBalanceMap, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch balances for multiple identities
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_ids`: Array of identity IDs (32-byte arrays)
/// - `identity_ids_len`: Number of identity IDs in the array
///
/// # Returns
/// DashSDKResult with data_type = IdentityBalanceMap containing identity IDs mapped to their balances
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identities_fetch_balances(
    sdk_handle: *const SDKHandle,
    identity_ids: *const [u8; 32],
    identity_ids_len: usize,
) -> DashSDKResult {
    if sdk_handle.is_null() || (identity_ids.is_null() && identity_ids_len > 0) {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle or identity IDs is null".to_string(),
        ));
    }

    if identity_ids_len == 0 {
        // Return empty map for empty input
        let map = DashSDKIdentityBalanceMap {
            entries: std::ptr::null_mut(),
            count: 0,
        };
        return DashSDKResult::success_identity_balance_map(map);
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    // Convert raw pointers to identifiers
    let identifiers: Result<Vec<Identifier>, DashSDKError> =
        std::slice::from_raw_parts(identity_ids, identity_ids_len)
            .iter()
            .map(|id_bytes| {
                Identifier::from_bytes(id_bytes).map_err(|e| {
                    DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Invalid identity ID: {}", e),
                    )
                })
            })
            .collect();

    let identifiers = match identifiers {
        Ok(ids) => ids,
        Err(e) => return DashSDKResult::error(e),
    };

    // Keep a copy of the original IDs for result mapping
    let original_ids: Vec<[u8; 32]> =
        std::slice::from_raw_parts(identity_ids, identity_ids_len).to_vec();

    let result: Result<DashSDKIdentityBalanceMap, FFIError> = wrapper.runtime.block_on(async {
        // Fetch identities balances
        let balances: IdentityBalances =
            IdentityBalance::fetch_many(&wrapper.sdk, identifiers.clone())
                .await
                .map_err(FFIError::from)?;

        // Convert to entries array
        let mut entries: Vec<DashSDKIdentityBalanceEntry> = Vec::with_capacity(identity_ids_len);

        // Process results in the same order as input
        for (i, id) in identifiers.iter().enumerate() {
            let balance = balances.get(id).and_then(|opt| *opt).unwrap_or(u64::MAX);
            entries.push(DashSDKIdentityBalanceEntry {
                identity_id: original_ids[i],
                balance,
            });
        }

        let count = entries.len();
        let entries_ptr = entries.as_mut_ptr();
        std::mem::forget(entries); // Prevent deallocation

        Ok(DashSDKIdentityBalanceMap {
            entries: entries_ptr,
            count,
        })
    });

    match result {
        Ok(map) => DashSDKResult::success_identity_balance_map(map),
        Err(e) => DashSDKResult::error(e.into()),
    }
}
