//! Utility functions

use std::ffi::CString;
use std::os::raw::c_char;

/// Convert a Rust string to a C string
pub(crate) fn rust_string_to_c(s: String) -> Result<*mut c_char, std::ffi::NulError> {
    CString::new(s).map(|c_str| c_str.into_raw())
}
