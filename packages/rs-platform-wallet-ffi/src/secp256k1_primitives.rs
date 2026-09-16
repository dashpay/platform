//! Standalone secp256k1 primitives used by DashConnect.

use std::slice;

use dashcore::secp256k1::ecdh::shared_secret_point;
use dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
use zeroize::Zeroizing;

use crate::error::*;
use crate::{check_ptr, unwrap_result_or_return};

struct WipingSecretKey(SecretKey);

impl Drop for WipingSecretKey {
    fn drop(&mut self) {
        self.0.non_secure_erase();
    }
}

fn invalid_parameter(message: impl Into<String>) -> PlatformWalletFFIResult {
    PlatformWalletFFIResult::err(
        PlatformWalletFFIResultCode::ErrorInvalidParameter,
        message.into(),
    )
}

fn parse_secret_key(
    private_key: *const u8,
    private_key_len: usize,
) -> Result<WipingSecretKey, PlatformWalletFFIResult> {
    if private_key_len != 32 {
        return Err(invalid_parameter(format!(
            "secp256k1 private keys must be exactly 32 bytes, got {private_key_len}"
        )));
    }

    // `SecretKey::from_slice` converts the slice into an unguarded local
    // `[u8; 32]` on its way to the returned key, so parsing straight from the
    // caller's slice would leave a second, unscrubbed copy of the scalar on
    // the stack. Staging it here keeps every copy this side owns wiped.
    let mut staged = Zeroizing::new([0u8; 32]);
    staged.copy_from_slice(unsafe { slice::from_raw_parts(private_key, private_key_len) });

    SecretKey::from_byte_array(&staged)
        .map(WipingSecretKey)
        .map_err(|error| invalid_parameter(format!("Invalid secp256k1 private key: {error}")))
}

fn parse_compressed_public_key(
    public_key: *const u8,
    public_key_len: usize,
) -> Result<PublicKey, PlatformWalletFFIResult> {
    if public_key_len != 33 {
        return Err(invalid_parameter(format!(
            "Compressed secp256k1 public keys must be exactly 33 bytes, got {public_key_len}"
        )));
    }

    let bytes = unsafe { slice::from_raw_parts(public_key, public_key_len) };
    if !matches!(bytes.first().copied(), Some(0x02) | Some(0x03)) {
        return Err(invalid_parameter(
            "Compressed secp256k1 public keys must start with 0x02 or 0x03".to_string(),
        ));
    }

    PublicKey::from_slice(bytes)
        .map_err(|error| invalid_parameter(format!("Invalid compressed secp256k1 point: {error}")))
}

/// Returns `1` when `pubkey` is a valid compressed secp256k1 point, `0` otherwise.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_secp256k1_verify_compressed_point(
    pubkey: *const u8,
    pubkey_len: usize,
) -> i32 {
    if pubkey.is_null() || pubkey_len != 33 {
        return 0;
    }

    match parse_compressed_public_key(pubkey, pubkey_len) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

/// Derive the 33-byte compressed public key for a 32-byte secp256k1 scalar.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_secp256k1_compressed_public_key(
    seckey: *const u8,
    seckey_len: usize,
    out_pubkey: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(seckey);
    check_ptr!(out_pubkey);

    let secret_key = unwrap_result_or_return!(parse_secret_key(seckey, seckey_len));
    let compressed = PublicKey::from_secret_key(&Secp256k1::new(), &secret_key.0).serialize();
    std::ptr::copy_nonoverlapping(compressed.as_ptr(), out_pubkey, compressed.len());
    PlatformWalletFFIResult::ok()
}

/// Derive the raw affine X coordinate of `seckey * pubkey` (unhashed ECDH output).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_secp256k1_ecdh_shared_x(
    seckey: *const u8,
    seckey_len: usize,
    pubkey: *const u8,
    pubkey_len: usize,
    out_shared_x: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(seckey);
    check_ptr!(pubkey);
    check_ptr!(out_shared_x);

    let secret_key = unwrap_result_or_return!(parse_secret_key(seckey, seckey_len));
    let public_key = unwrap_result_or_return!(parse_compressed_public_key(pubkey, pubkey_len));

    // `shared_secret_point` uses libsecp256k1's constant-time ECDH path:
    // the scalar here is the wallet's private ephemeral key and the point
    // comes from the peer, so the multiplication must not branch on the
    // secret. Returns `x || y`; DashConnect wants the raw, unhashed X.
    let shared_point = Zeroizing::new(shared_secret_point(&public_key, &secret_key.0));
    std::ptr::copy_nonoverlapping(shared_point.as_ptr(), out_shared_x, 32);
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIVATE_KEY_A: [u8; 32] = [0x01; 32];
    const PRIVATE_KEY_B: [u8; 32] = [0x02; 32];
    const PUBLIC_KEY_A: [u8; 33] = [
        0x03, 0x1b, 0x84, 0xc5, 0x56, 0x7b, 0x12, 0x64, 0x40, 0x99, 0x5d, 0x3e, 0xd5, 0xaa, 0xba,
        0x05, 0x65, 0xd7, 0x1e, 0x18, 0x34, 0x60, 0x48, 0x19, 0xff, 0x9c, 0x17, 0xf5, 0xe9, 0xd5,
        0xdd, 0x07, 0x8f,
    ];
    const PUBLIC_KEY_B: [u8; 33] = [
        0x02, 0x4d, 0x4b, 0x6c, 0xd1, 0x36, 0x10, 0x32, 0xca, 0x9b, 0xd2, 0xae, 0xb9, 0xd9, 0x00,
        0xaa, 0x4d, 0x45, 0xd9, 0xea, 0xd8, 0x0a, 0xc9, 0x42, 0x33, 0x74, 0xc4, 0x51, 0xa7, 0x25,
        0x4d, 0x07, 0x66,
    ];
    const SHARED_X_AB: [u8; 32] = [
        0xd0, 0x15, 0x8a, 0x38, 0xfa, 0xf6, 0x11, 0x8a, 0xf1, 0x33, 0xaf, 0x12, 0xd9, 0xbf, 0xa3,
        0x88, 0xea, 0xb4, 0xa0, 0x8d, 0x1a, 0x20, 0x88, 0xea, 0x6e, 0x6e, 0xc1, 0x26, 0x9e, 0x03,
        0x56, 0x7f,
    ];

    #[test]
    fn compressed_public_key_matches_known_vector() {
        let mut out = [0u8; 33];
        let result = unsafe {
            platform_wallet_secp256k1_compressed_public_key(
                PRIVATE_KEY_A.as_ptr(),
                PRIVATE_KEY_A.len(),
                out.as_mut_ptr(),
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out, PUBLIC_KEY_A);
    }

    #[test]
    fn ecdh_shared_x_matches_known_vector() {
        let mut out = [0u8; 32];
        let result = unsafe {
            platform_wallet_secp256k1_ecdh_shared_x(
                PRIVATE_KEY_A.as_ptr(),
                PRIVATE_KEY_A.len(),
                PUBLIC_KEY_B.as_ptr(),
                PUBLIC_KEY_B.len(),
                out.as_mut_ptr(),
            )
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out, SHARED_X_AB);
    }

    #[test]
    fn verify_compressed_point_rejects_invalid_point() {
        let mut invalid = [0u8; 33];
        invalid[0] = 0x02;
        let result = unsafe {
            platform_wallet_secp256k1_verify_compressed_point(invalid.as_ptr(), invalid.len())
        };

        assert_eq!(result, 0);
    }

    #[test]
    fn verify_compressed_point_accepts_known_vector() {
        let result = unsafe {
            platform_wallet_secp256k1_verify_compressed_point(
                PUBLIC_KEY_A.as_ptr(),
                PUBLIC_KEY_A.len(),
            )
        };

        assert_eq!(result, 1);
    }

    #[test]
    fn compressed_public_key_rejects_out_of_range_scalar() {
        let invalid_scalar = [0xffu8; 32];
        let mut out = [0u8; 33];
        let result = unsafe {
            platform_wallet_secp256k1_compressed_public_key(
                invalid_scalar.as_ptr(),
                invalid_scalar.len(),
                out.as_mut_ptr(),
            )
        };

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
    }

    #[test]
    fn ecdh_rejects_wrong_length_inputs() {
        let mut out = [0u8; 32];
        let short_private = [0x01u8; 31];
        let short_public = [0x02u8; 32];

        let private_error = unsafe {
            platform_wallet_secp256k1_ecdh_shared_x(
                short_private.as_ptr(),
                short_private.len(),
                PUBLIC_KEY_A.as_ptr(),
                PUBLIC_KEY_A.len(),
                out.as_mut_ptr(),
            )
        };
        assert_eq!(
            private_error.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );

        let public_error = unsafe {
            platform_wallet_secp256k1_ecdh_shared_x(
                PRIVATE_KEY_B.as_ptr(),
                PRIVATE_KEY_B.len(),
                short_public.as_ptr(),
                short_public.len(),
                out.as_mut_ptr(),
            )
        };
        assert_eq!(
            public_error.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
    }
}
