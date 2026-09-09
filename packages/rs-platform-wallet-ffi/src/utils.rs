use crate::check_ptr;
use crate::error::*;
use std::os::raw::c_uchar;

/// Free bytes allocated by FFI functions
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_bytes_free(bytes: *mut c_uchar, len: usize) {
    if !bytes.is_null() && len > 0 {
        unsafe {
            let _ = Vec::from_raw_parts(bytes, len, len);
        }
    }
}

/// Generate random identifier into a 32-byte out-buffer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_generate_random_identifier(
    out_id: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_id);
    let id = dpp::prelude::Identifier::random();
    unsafe { crate::types::write_identifier(out_id, &id) };
    PlatformWalletFFIResult::ok()
}

/// Compute hash160 (RIPEMD160(SHA256(data))) of the input bytes.
///
/// Exposed so the Swift side can stamp a 20-byte public-key hash onto
/// the keychain `IdentityPrivateKeyMetadata` row at write time, without
/// pulling in a third-party RIPEMD-160 implementation in Swift
/// (CommonCrypto + CryptoKit don't expose it; the only RIPEMD-160 in
/// the iOS toolchain we already link is via `dashcore::hashes`, so we
/// reuse it here).
///
/// # Parameters
/// - `data`: pointer to the input bytes (typically a 33-byte
///   compressed secp256k1 pubkey, but any length works).
/// - `data_len`: byte count for `data`.
/// - `out_hash`: pointer to a 20-byte buffer the caller has already
///   allocated. The function writes the resulting hash there on
///   success.
///
/// Returns 0 on success, -1 on null pointer or zero length input.
///
/// # Safety
/// - `data` must be a valid `[u8; data_len]` buffer for the duration of
///   the call.
/// - `out_hash` must be a valid `[u8; 20]` writable buffer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_hash160(
    data: *const u8,
    data_len: usize,
    out_hash: *mut u8,
) -> i32 {
    if data.is_null() || data_len == 0 || out_hash.is_null() {
        return -1;
    }
    use dashcore::hashes::Hash;
    let bytes = std::slice::from_raw_parts(data, data_len);
    let hash = dashcore::hashes::hash160::Hash::hash(bytes);
    let h: [u8; 20] = hash.to_byte_array();
    std::ptr::copy_nonoverlapping(h.as_ptr(), out_hash, 20);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash160_matches_known_vector() {
        use dashcore::hashes::Hash;
        let input = [0u8; 33];
        let mut out = [0u8; 20];
        let rc = unsafe { platform_wallet_hash160(input.as_ptr(), input.len(), out.as_mut_ptr()) };
        assert_eq!(rc, 0);
        let expected = dashcore::hashes::hash160::Hash::hash(&input);
        let expected_bytes: [u8; 20] = expected.to_byte_array();
        assert_eq!(out, expected_bytes);
    }

    #[test]
    fn test_hash160_rejects_null_input() {
        let mut out = [0u8; 20];
        let rc = unsafe { platform_wallet_hash160(std::ptr::null(), 33, out.as_mut_ptr()) };
        assert_eq!(rc, -1);
    }

    #[test]
    fn test_hash160_rejects_zero_len() {
        let input = [0u8; 33];
        let mut out = [0u8; 20];
        let rc = unsafe { platform_wallet_hash160(input.as_ptr(), 0, out.as_mut_ptr()) };
        assert_eq!(rc, -1);
    }

    #[test]
    fn test_hash160_rejects_null_out() {
        let input = [0u8; 33];
        let rc =
            unsafe { platform_wallet_hash160(input.as_ptr(), input.len(), std::ptr::null_mut()) };
        assert_eq!(rc, -1);
    }

    #[test]
    fn test_generate_random_identifier() {
        unsafe {
            let mut id = [0u8; 32];
            let result = platform_wallet_generate_random_identifier(id.as_mut_ptr());
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_ne!(id, [0u8; 32]);
        }
    }
}
