//! Platform version validation utilities

use crate::utils::error::{format_error, ErrorCategory};
use dpp::version::{PlatformVersion, INITIAL_PROTOCOL_VERSION, LATEST_VERSION};
use wasm_bindgen::JsValue;

/// Minimum supported platform version
pub const MIN_PLATFORM_VERSION: u32 = INITIAL_PROTOCOL_VERSION;

/// Maximum supported platform version
pub const MAX_PLATFORM_VERSION: u32 = LATEST_VERSION;

/// Validate and get a platform version with range checks
pub fn get_platform_version_with_validation(
    version_number: u32,
) -> Result<&'static PlatformVersion, JsValue> {
    validate_platform_version(version_number)
        .map_err(|details| format_error(ErrorCategory::PlatformVersionError, &details))
}

// JsValue-free so the range checks stay testable on non-wasm targets
fn validate_platform_version(version_number: u32) -> Result<&'static PlatformVersion, String> {
    if version_number < MIN_PLATFORM_VERSION {
        return Err(format!(
            "platform version {} is below minimum supported version {}",
            version_number, MIN_PLATFORM_VERSION
        ));
    }

    if version_number > MAX_PLATFORM_VERSION {
        return Err(format!(
            "platform version {} exceeds maximum supported version {}",
            version_number, MAX_PLATFORM_VERSION
        ));
    }

    PlatformVersion::get(version_number)
        .map_err(|e| format!("failed to get platform version {}: {:?}", version_number, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_resolve_every_supported_platform_version() {
        for version in MIN_PLATFORM_VERSION..=MAX_PLATFORM_VERSION {
            let result = validate_platform_version(version);
            assert!(result.is_ok(), "Version {} should be valid", version);
        }
    }

    #[test]
    fn should_reject_platform_versions_outside_supported_range() {
        assert!(validate_platform_version(MIN_PLATFORM_VERSION - 1).is_err());
        assert!(validate_platform_version(MAX_PLATFORM_VERSION + 1).is_err());
        assert!(validate_platform_version(u32::MAX).is_err());
    }
}
