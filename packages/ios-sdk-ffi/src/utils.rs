//! Utility functions

use std::ffi::CString;
use std::os::raw::c_char;

/// Convert a Rust string to a C string
pub(crate) fn rust_string_to_c(s: String) -> Result<*mut c_char, std::ffi::NulError> {
    CString::new(s).map(|c_str| c_str.into_raw())
}

/// Generate a new mnemonic
#[no_mangle]
pub extern "C" fn ios_sdk_generate_mnemonic() -> *mut c_char {
    // Note: This is a placeholder implementation.
    // In a production environment, you would want to:
    // 1. Add the `bip39` crate as a dependency
    // 2. Use proper cryptographically secure random generation
    // 3. Generate a proper BIP39 mnemonic
    //
    // Example with bip39 crate:
    // ```
    // use bip39::{Mnemonic, Language};
    // let mnemonic = Mnemonic::generate_in(Language::English, 12).unwrap();
    // return CString::new(mnemonic.to_string()).unwrap().into_raw();
    // ```
    
    // For now, return a sample valid BIP39 mnemonic for testing
    let sample_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    match CString::new(sample_mnemonic) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Generate a mnemonic with specified word count
#[no_mangle]
pub extern "C" fn ios_sdk_generate_mnemonic_with_word_count(word_count: u32) -> *mut c_char {
    // Validate word count (BIP39 supports 12, 15, 18, 21, or 24 words)
    let valid_word_counts = [12, 15, 18, 21, 24];
    if !valid_word_counts.contains(&word_count) {
        return std::ptr::null_mut();
    }
    
    // Note: This is a placeholder. In production, use proper BIP39 generation
    // with the specified word count
    let sample_mnemonic = match word_count {
        12 => "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        15 => "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon ability",
        18 => "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon agent",
        21 => "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon admit",
        24 => "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
        _ => return std::ptr::null_mut(),
    };
    
    match CString::new(sample_mnemonic) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Validate a mnemonic phrase
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_validate_mnemonic(mnemonic: *const c_char) -> bool {
    if mnemonic.is_null() {
        return false;
    }
    
    let mnemonic_str = match std::ffi::CStr::from_ptr(mnemonic).to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    
    // Basic validation: check word count
    let word_count = mnemonic_str.split_whitespace().count();
    let valid_word_counts = [12, 15, 18, 21, 24];
    
    // Note: In production, you would use the bip39 crate to properly validate
    // against the BIP39 wordlist and verify checksum
    // Example:
    // ```
    // use bip39::Mnemonic;
    // Mnemonic::from_phrase(mnemonic_str, Language::English).is_ok()
    // ```
    
    valid_word_counts.contains(&word_count)
}