//! Platform version validation utilities

use crate::utils::error::{format_error, ErrorCategory};
use dpp::version::PlatformVersion;
use wasm_bindgen::JsValue;

/// Minimum supported platform version
pub const MIN_PLATFORM_VERSION: u32 = 1;

/// Maximum supported platform version
/// This should be updated when new versions are released
pub const MAX_PLATFORM_VERSION: u32 = 9;

/// Validate and get a platform version with range checks
pub fn get_platform_version_with_validation(
    version_number: u32,
) -> Result<&'static PlatformVersion, JsValue> {
    // Range check
    if version_number < MIN_PLATFORM_VERSION {
        return Err(format_error(
            ErrorCategory::PlatformVersionError,
            &format!(
                "platform version {} is below minimum supported version {}",
                version_number, MIN_PLATFORM_VERSION
            ),
        ));
    }

    if version_number > MAX_PLATFORM_VERSION {
        return Err(format_error(
            ErrorCategory::PlatformVersionError,
            &format!(
                "platform version {} exceeds maximum supported version {}",
                version_number, MAX_PLATFORM_VERSION
            ),
        ));
    }

    // Get the version - this should not fail for valid range
    PlatformVersion::get(version_number).map_err(|e| {
        format_error(
            ErrorCategory::PlatformVersionError,
            &format!("failed to get platform version {}: {:?}", version_number, e),
        )
    })
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests {
    use super::*;

    #[test]
    fn test_valid_platform_versions() {
        // Test all valid versions
        for version in MIN_PLATFORM_VERSION..=MAX_PLATFORM_VERSION {
            let result = get_platform_version_with_validation(version);
            assert!(result.is_ok(), "Version {} should be valid", version);
        }
    }

    #[test]
    fn test_invalid_platform_versions() {
        // Test below minimum
        let result = get_platform_version_with_validation(0);
        assert!(result.is_err());

        // Test above maximum
        let result = get_platform_version_with_validation(MAX_PLATFORM_VERSION + 1);
        assert!(result.is_err());

        // Test very large number
        let result = get_platform_version_with_validation(u32::MAX);
        assert!(result.is_err());
    }
}
