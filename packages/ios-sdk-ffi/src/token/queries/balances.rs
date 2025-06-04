//! Token balance query operations

use crate::{IOSSDKError, IOSSDKErrorCode, IOSSDKResult};

/// Get identity token balances
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_get_identity_balances(// TODO: Add proper parameters when migrating from main token.rs
) -> IOSSDKResult {
    IOSSDKResult::error(IOSSDKError::new(
        IOSSDKErrorCode::NotImplemented,
        "Token balance query functionality to be migrated from main token.rs".to_string(),
    ))
}
