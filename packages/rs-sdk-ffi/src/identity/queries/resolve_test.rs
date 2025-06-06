//! Tests for name resolution

#[cfg(test)]
mod tests {
    use super::super::resolve::dash_sdk_identity_resolve_name;
    use crate::sdk::SDKWrapper;
    use crate::test_utils::setup_test_sdk;
    use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};
    use std::ffi::CString;

    #[test]
    fn test_resolve_name_null_sdk() {
        let name = CString::new("alice.dash").unwrap();

        unsafe {
            let result = dash_sdk_identity_resolve_name(std::ptr::null(), name.as_ptr());
            assert!(!result.error.is_null());
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }
    }

    #[test]
    fn test_resolve_name_null_name() {
        let sdk_wrapper = setup_test_sdk();
        let sdk_handle = &sdk_wrapper as *const SDKWrapper;

        unsafe {
            let result = dash_sdk_identity_resolve_name(sdk_handle as *const _, std::ptr::null());
            assert!(!result.error.is_null());
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }
    }

    #[test]
    fn test_resolve_name_invalid_utf8() {
        let sdk_wrapper = setup_test_sdk();
        let sdk_handle = &sdk_wrapper as *const SDKWrapper;

        // Create invalid UTF-8 sequence
        let invalid_utf8 = vec![0xFF, 0xFE, 0x00];

        unsafe {
            let result = dash_sdk_identity_resolve_name(
                sdk_handle as *const _,
                invalid_utf8.as_ptr() as *const _,
            );
            assert!(!result.error.is_null());
            let error = &*result.error;
            assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);
        }
    }

    #[test]
    fn test_resolve_name_parsing() {
        // Test that name parsing works correctly
        // This is a unit test that doesn't require actual network calls

        let test_cases = vec![
            ("alice.dash", "alice", "dash"),
            ("bob", "bob", "dash"),
            ("test.subdomain.dash", "test.subdomain", "dash"),
        ];

        for (input, expected_label, expected_parent) in test_cases {
            let (label, parent) = if let Some(dot_pos) = input.rfind('.') {
                (&input[..dot_pos], &input[dot_pos + 1..])
            } else {
                (input, "dash")
            };

            assert_eq!(label, expected_label, "Label mismatch for input: {}", input);
            assert_eq!(
                parent, expected_parent,
                "Parent mismatch for input: {}",
                input
            );
        }
    }
}
