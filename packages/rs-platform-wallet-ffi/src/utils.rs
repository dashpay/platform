use crate::error::*;
use crate::{check_ptr, unwrap_result_or_return};
use std::os::raw::{c_char, c_uchar};

/// RAII guard that scrubs a `secp256k1::SecretKey`'s scalar on drop. `from_slice`
/// allocates a 32-byte scalar copy of the caller's private key, and `SecretKey`
/// has no `Drop` wipe of its own — so without this the copy would survive on the
/// stack after the call returns. Mirrors `WipingSecretKey` in
/// `rs-sdk-ffi/src/mnemonic_resolver_core_signer.rs`.
struct WipingSecretKey(dashcore::secp256k1::SecretKey);

impl Drop for WipingSecretKey {
    fn drop(&mut self) {
        self.0.non_secure_erase();
    }
}

/// Serialize any object to JSON bytes
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_serialize_to_json_bytes(
    json_string: *const c_char,
    out_bytes: *mut *mut c_uchar,
    out_len: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(json_string);
    check_ptr!(out_bytes);
    check_ptr!(out_len);
    // Sentinel first: the UTF-8 check below is fallible, and
    // `platform_wallet_bytes_free` reconstructs a `Vec` from any non-null
    // pointer / non-zero length pair — a cleanup-on-error caller must never
    // see stack garbage here.
    unsafe {
        *out_bytes = std::ptr::null_mut();
        *out_len = 0;
    }

    let json_str =
        unwrap_result_or_return!(unsafe { std::ffi::CStr::from_ptr(json_string).to_str() });

    let bytes = json_str.as_bytes().to_vec();
    let len = bytes.len();
    let ptr = bytes.as_ptr() as *mut c_uchar;
    std::mem::forget(bytes);

    unsafe {
        *out_bytes = ptr;
        *out_len = len;
    }

    PlatformWalletFFIResult::ok()
}

/// Deserialize JSON bytes to string
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_deserialize_from_json_bytes(
    bytes: *const c_uchar,
    len: usize,
    out_json_string: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(bytes);
    check_ptr!(out_json_string);
    // Null the out-pointer before the fallible UTF-8 / NUL checks below so
    // an error return never leaves it holding stack garbage for a
    // cleanup-on-error caller to `platform_wallet_string_free`.
    unsafe { *out_json_string = std::ptr::null_mut() };

    let data = unsafe { std::slice::from_raw_parts(bytes, len) };
    let s = unwrap_result_or_return!(std::str::from_utf8(data));
    let c_str = unwrap_result_or_return!(std::ffi::CString::new(s));
    unsafe { *out_json_string = c_str.into_raw() };
    PlatformWalletFFIResult::ok()
}

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

/// Convert identifier (32 bytes pointed to by `id`) to base58 hex
/// string.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identifier_to_hex(
    id: *const u8,
    out_hex: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(id);
    check_ptr!(out_hex);

    let identifier = unwrap_result_or_return!(unsafe { crate::types::read_identifier(id) });

    let hex = identifier.to_string(dpp::platform_value::string_encoding::Encoding::Base58);
    let c_str = unwrap_result_or_return!(std::ffi::CString::new(hex));
    unsafe { *out_hex = c_str.into_raw() };
    PlatformWalletFFIResult::ok()
}

/// Convert base58 hex string to identifier (writes 32 bytes into
/// `out_id`).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identifier_from_hex(
    hex: *const c_char,
    out_id: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(hex);
    check_ptr!(out_id);

    let hex_str = unwrap_result_or_return!(unsafe { std::ffi::CStr::from_ptr(hex).to_str() });

    let identifier = unwrap_result_or_return!(dpp::prelude::Identifier::from_string(
        hex_str,
        dpp::platform_value::string_encoding::Encoding::Base58,
    ));
    unsafe { crate::types::write_identifier(out_id, &identifier) };
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

/// Compute the hash160 of the **compressed** secp256k1 public key for a
/// 32-byte ECDSA private scalar — i.e. the on-chain `ECDSA_SECP256K1`
/// public-key hash that scalar would own.
///
/// Lets the Swift Keychain layer cheaply re-verify, after re-deriving an
/// identity key's private scalar, that the scalar actually reproduces the
/// stored on-chain key's hash before persisting it — a cross-FFI guard
/// against the Rust-side derivation path and the Swift-side re-derivation
/// ever drifting (compute it here rather than pull a secp256k1 + RIPEMD-160
/// stack into Swift). Compression is network-independent, so no network
/// parameter is needed.
///
/// # Parameters
/// - `private_key`: pointer to the 32-byte ECDSA secret scalar.
/// - `out_hash`: caller-allocated 20-byte buffer; the hash160 of the
///   compressed pubkey is written here on success.
///
/// Returns 0 on success, -1 on null pointer or an invalid (out-of-range)
/// secret scalar.
///
/// # Safety
/// - `private_key` must be a valid `[u8; 32]` buffer for the call.
/// - `out_hash` must be a valid `[u8; 20]` writable buffer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_pubkey_hash_from_private_key(
    private_key: *const u8,
    out_hash: *mut u8,
) -> i32 {
    if private_key.is_null() || out_hash.is_null() {
        return -1;
    }
    use dashcore::hashes::Hash;
    use dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};

    let sk_bytes = std::slice::from_raw_parts(private_key, 32);
    let secp = Secp256k1::new();
    // `WipingSecretKey` scrubs the `from_slice`-allocated scalar copy on every
    // exit path (the success return below, the `Err` early return, and any
    // panic) — the caller's `private_key` bytes are theirs to manage, but this
    // copy must not linger.
    let secret_key = match SecretKey::from_slice(sk_bytes) {
        Ok(sk) => WipingSecretKey(sk),
        Err(_) => return -1,
    };
    let pubkey = PublicKey::from_secret_key(&secp, &secret_key.0).serialize();
    let hash = dashcore::hashes::hash160::Hash::hash(&pubkey);
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
    fn test_pubkey_hash_from_private_key_matches_canonical_derivation() {
        use dashcore::hashes::Hash;
        use dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};

        // A fixed, in-range scalar.
        let mut scalar = [0u8; 32];
        scalar[31] = 1;

        let mut out = [0u8; 20];
        let rc = unsafe {
            platform_wallet_pubkey_hash_from_private_key(scalar.as_ptr(), out.as_mut_ptr())
        };
        assert_eq!(rc, 0);

        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&scalar).expect("in-range scalar");
        let pubkey = PublicKey::from_secret_key(&secp, &sk).serialize();
        let expected: [u8; 20] = dashcore::hashes::hash160::Hash::hash(&pubkey).to_byte_array();
        assert_eq!(out, expected);
    }

    #[test]
    fn test_pubkey_hash_from_private_key_rejects_null() {
        let mut out = [0u8; 20];
        let scalar = [1u8; 32];
        assert_eq!(
            unsafe {
                platform_wallet_pubkey_hash_from_private_key(std::ptr::null(), out.as_mut_ptr())
            },
            -1
        );
        assert_eq!(
            unsafe {
                platform_wallet_pubkey_hash_from_private_key(scalar.as_ptr(), std::ptr::null_mut())
            },
            -1
        );
    }

    #[test]
    fn test_pubkey_hash_from_private_key_rejects_invalid_scalar() {
        // All-zero scalar is out of secp256k1's valid range.
        let scalar = [0u8; 32];
        let mut out = [0u8; 20];
        let rc = unsafe {
            platform_wallet_pubkey_hash_from_private_key(scalar.as_ptr(), out.as_mut_ptr())
        };
        assert_eq!(rc, -1);
    }

    #[test]
    fn test_serialize_deserialize_json_bytes() {
        unsafe {
            let json = std::ffi::CString::new(r#"{"test":"value"}"#).unwrap();
            let mut bytes: *mut c_uchar = std::ptr::null_mut();
            let mut len: usize = 0;

            let result =
                platform_wallet_serialize_to_json_bytes(json.as_ptr(), &mut bytes, &mut len);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert!(!bytes.is_null());
            assert!(len > 0);

            let mut json_out: *mut c_char = std::ptr::null_mut();
            let result = platform_wallet_deserialize_from_json_bytes(bytes, len, &mut json_out);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert!(!json_out.is_null());

            let json_str = std::ffi::CStr::from_ptr(json_out).to_str().unwrap();
            assert_eq!(json_str, r#"{"test":"value"}"#);

            platform_wallet_bytes_free(bytes, len);
            crate::platform_wallet_string_free(json_out);
        }
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

    #[test]
    fn test_identifier_to_from_hex() {
        unsafe {
            let mut id = [0u8; 32];
            platform_wallet_generate_random_identifier(id.as_mut_ptr());

            let mut hex: *mut c_char = std::ptr::null_mut();
            let result = platform_wallet_identifier_to_hex(id.as_ptr(), &mut hex);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert!(!hex.is_null());

            let mut id2 = [0u8; 32];
            let result = platform_wallet_identifier_from_hex(hex, id2.as_mut_ptr());
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

            assert_eq!(id, id2);

            crate::platform_wallet_string_free(hex);
        }
    }
}
