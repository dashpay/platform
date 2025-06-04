//! Token unfreeze operations

use crate::{IOSSDKError, IOSSDKErrorCode, IOSSDKResult};

/// Unfreeze token transfers
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_token_unfreeze(
    // TODO: Add proper parameters when migrating from main token.rs
) -> IOSSDKResult {
    IOSSDKResult::error(IOSSDKError::new(
        IOSSDKErrorCode::NotImplemented,
        "Token unfreeze functionality to be migrated from main token.rs".to_string(),
    ))
}