//! Token balance query operations

use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};

/// Get identity token balances
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_get_identity_balances(// TODO: Add proper parameters when migrating from main token.rs
) -> DashSDKResult {
    DashSDKResult::error(DashSDKError::new(
        DashSDKErrorCode::NotImplemented,
        "Token balance query functionality to be migrated from main token.rs".to_string(),
    ))
}
