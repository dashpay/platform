//! Token information query operations

use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};

/// Get identity token information
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_token_get_identity_infos(// TODO: Add proper parameters when migrating from main token.rs
) -> DashSDKResult {
    DashSDKResult::error(DashSDKError::new(
        DashSDKErrorCode::NotImplemented,
        "Token info query functionality to be migrated from main token.rs".to_string(),
    ))
}
