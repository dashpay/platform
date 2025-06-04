//! Token information query operations

use crate::{IOSSDKError, IOSSDKErrorCode, IOSSDKResult};

/// Get identity token information
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_get_identity_infos(// TODO: Add proper parameters when migrating from main token.rs
) -> IOSSDKResult {
    IOSSDKResult::error(IOSSDKError::new(
        IOSSDKErrorCode::NotImplemented,
        "Token info query functionality to be migrated from main token.rs".to_string(),
    ))
}
