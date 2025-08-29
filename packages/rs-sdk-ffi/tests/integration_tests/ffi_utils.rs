//! FFI-specific test utilities

use rs_sdk_ffi::*;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::ptr;

/// Create an SDK handle for testing using the mock mode with offline test vectors
pub fn create_test_sdk_handle(namespace: &str) -> *const SDKHandle {
    // Use the rs-sdk test vectors directory
    let base_dump_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("rs-sdk")
        .join("tests")
        .join("vectors");

    let dump_dir = if namespace.is_empty() {
        base_dump_dir
    } else {
        let namespace = namespace.replace(' ', "_");
        base_dump_dir.join(namespace)
    };

    let dump_dir_str = CString::new(dump_dir.to_string_lossy().as_ref()).unwrap();

    unsafe {
        let handle = dash_sdk_create_handle_with_mock(dump_dir_str.as_ptr());
        if handle.is_null() {
            panic!("Failed to create mock SDK handle");
        }
        handle as *const SDKHandle
    }
}

/// Destroy an SDK handle
pub fn destroy_test_sdk_handle(handle: *const SDKHandle) {
    unsafe {
        dash_sdk_destroy(handle as *mut SDKHandle);
    }
}

/// Convert a Rust string to a C string pointer
pub fn to_c_string(s: &str) -> CString {
    CString::new(s).expect("Failed to create CString")
}

/// Convert a C string pointer to a Rust string
pub unsafe fn from_c_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

/// Parse a DashSDKResult and extract the string data
pub unsafe fn parse_string_result(result: DashSDKResult) -> Result<Option<String>, String> {
    if !result.error.is_null() {
        let error = Box::from_raw(result.error);
        return Err(format!(
            "Error code {}: {}",
            error.code as i32,
            from_c_string(error.message).unwrap_or_default()
        ));
    }

    match result.data_type {
        DashSDKResultDataType::NoData => Ok(None),
        DashSDKResultDataType::String => {
            if result.data.is_null() {
                Ok(None)
            } else {
                let c_str = CStr::from_ptr(result.data as *const c_char);
                let string = c_str.to_string_lossy().into_owned();
                // Free the C string
                dash_sdk_string_free(result.data as *mut c_char);
                Ok(Some(string))
            }
        }
        _ => Err("Unexpected result data type".to_string()),
    }
}

/// Parse a JSON string result
pub fn parse_json_result(json: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(json).map_err(|e| format!("Failed to parse JSON: {}", e))
}

/// Test helper to assert that a result is successful and contains data
pub unsafe fn assert_success_with_data(result: DashSDKResult) -> String {
    let data = parse_string_result(result)
        .expect("Result should be successful")
        .expect("Result should contain data");
    data
}

/// Test helper to assert that a result is successful but contains no data (None)
pub unsafe fn assert_success_none(result: DashSDKResult) {
    let data = parse_string_result(result).expect("Result should be successful");
    assert!(data.is_none(), "Expected None but got data: {:?}", data);
}

/// Test helper to assert that a result is an error
pub unsafe fn assert_error(result: DashSDKResult) {
    assert!(
        parse_string_result(result).is_err(),
        "Expected error but got success"
    );
}

/// Setup logging for tests
pub fn setup_logs() {
    // Initialize logging if needed
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();
}
