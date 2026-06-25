//! DIP-15 DashPay `encryptedAccountLabel` encryption.

use crate::aes::{decrypt_aes_256_cbc, encrypt_aes_256_cbc};
use crate::error::CryptoError;

/// Minimum label length in characters before encryption. DIP-15 fixes
/// `encryptedAccountLabel` at ≥48 bytes (16-byte IV + ≥32-byte AES-CBC
/// ciphertext); a plaintext shorter than 16 bytes encrypts to a single 16-byte
/// block (48 bytes total only at exactly 16). kotlin/dashj pad the label to
/// ≥16 chars with trailing spaces so even a short or empty label clears the
/// floor, and strip the padding on read. Matching this is required for a valid
/// document and for cross-client interop. (≥16 chars ⟹ ≥16 bytes, since each
/// char is ≥1 byte, so this guarantees the ciphertext clears 48 bytes.)
const ACCOUNT_LABEL_MIN_CHARS: usize = 16;

/// Maximum plaintext length in bytes after padding. DIP-15 caps
/// `encryptedAccountLabel` at 80 bytes (16-byte IV + ≤64-byte ciphertext); via
/// PKCS7, ≤64 ciphertext bytes means ≤63 plaintext bytes. A longer label is
/// truncated (on a char boundary) so no host-supplied string — however long —
/// can push the ciphertext past the contract's cap and error the broadcast.
/// kotlin/dashj likewise bound the label.
const ACCOUNT_LABEL_MAX_BYTES: usize = 63;

/// Normalize `label` to the DIP-15 plaintext bounds: pad to
/// ≥[`ACCOUNT_LABEL_MIN_CHARS`] chars with trailing spaces (the floor; ≥16
/// chars ⟹ ≥16 bytes), then truncate to ≤[`ACCOUNT_LABEL_MAX_BYTES`] bytes on a
/// char boundary (the ceiling). The result is always 16..=63 plaintext bytes,
/// so the encrypted field is always 48..=80 bytes for any input. Mirrors
/// kotlin's `padEnd(16, ' ')` plus a length cap.
fn fit_account_label(label: &str) -> String {
    // Floor: pad short labels to ≥16 chars.
    let deficit = ACCOUNT_LABEL_MIN_CHARS.saturating_sub(label.chars().count());
    let padded = if deficit == 0 {
        label.to_string()
    } else {
        let mut s = String::with_capacity(label.len() + deficit);
        s.push_str(label);
        s.extend(std::iter::repeat_n(' ', deficit));
        s
    };

    // Ceiling: truncate long labels to ≤63 bytes on a char boundary. (Padding
    // only ever reaches 16 chars ≤ 63 bytes, so truncation only fires on a
    // genuinely long input, and the truncated prefix is still ≥16 bytes.)
    if padded.len() <= ACCOUNT_LABEL_MAX_BYTES {
        return padded;
    }
    let mut end = ACCOUNT_LABEL_MAX_BYTES;
    while !padded.is_char_boundary(end) {
        end -= 1;
    }
    padded[..end].to_string()
}

/// Encrypt an account label for DashPay (DIP-15)
///
/// The label is normalized to the DIP-15 plaintext bounds before encryption
/// (see [`fit_account_label`]): short labels are space-padded so the output
/// clears the 48-byte floor, and over-long labels are truncated so it never
/// exceeds the 80-byte cap. The output is therefore always a valid
/// `encryptedAccountLabel` for any input. [`decrypt_account_label`] strips the
/// padding.
///
/// # Arguments
/// * `shared_key` - 32-byte shared secret from ECDH
/// * `iv` - 16-byte initialization vector (must be randomly generated, different from xpub IV)
/// * `label` - Account label string to encrypt
///
/// # Returns
/// Encrypted label with IV prepended (48-80 bytes: 16-byte IV + 32-64 byte encrypted data)
pub fn encrypt_account_label(shared_key: &[u8; 32], iv: &[u8; 16], label: &str) -> Vec<u8> {
    let fitted = fit_account_label(label);
    let encrypted_data = encrypt_aes_256_cbc(shared_key, iv, fitted.as_bytes());

    // Prepend IV to encrypted data as per DIP-15
    let mut result = Vec::with_capacity(16 + encrypted_data.len());
    result.extend_from_slice(iv);
    result.extend_from_slice(&encrypted_data);
    result
}

/// Decrypt an account label from DashPay (DIP-15)
///
/// Trailing spaces — the padding [`encrypt_account_label`] adds to clear the
/// 48-byte floor — are stripped, recovering the original label. (A label that
/// intentionally ended in spaces cannot round-trip those spaces; this is
/// inherent to the DIP-15 space-padding convention and matches kotlin/dashj.)
///
/// # Arguments
/// * `shared_key` - 32-byte shared secret from ECDH
/// * `encrypted_data` - Encrypted label with IV prepended (48-80 bytes total)
///
/// # Returns
/// Decrypted label string
pub fn decrypt_account_label(
    shared_key: &[u8; 32],
    encrypted_data: &[u8],
) -> Result<String, CryptoError> {
    if encrypted_data.len() < 16 {
        return Err(CryptoError::InvalidCiphertextLength);
    }

    // Extract IV from first 16 bytes
    let iv: [u8; 16] = encrypted_data[..16].try_into().unwrap();
    let ciphertext = &encrypted_data[16..];

    let decrypted = decrypt_aes_256_cbc(shared_key, &iv, ciphertext)?;
    let label = String::from_utf8(decrypted).map_err(|_| CryptoError::InvalidUtf8)?;
    Ok(label.trim_end_matches(' ').to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecdh::derive_shared_key_ecdh;
    use secp256k1::rand::{thread_rng, RngCore};
    use secp256k1::Secp256k1;

    #[test]
    fn test_account_label_encryption() {
        let secp = Secp256k1::new();
        let (secret1, _public1) = secp.generate_keypair(&mut thread_rng());
        let (_secret2, public2) = secp.generate_keypair(&mut thread_rng());

        // Derive shared key
        let shared_key = derive_shared_key_ecdh(&secret1, &public2);

        // Generate random IV
        let mut iv = [0u8; 16];
        thread_rng().fill_bytes(&mut iv);

        let label = "My DashPay Account";

        // Encrypt and decrypt
        let encrypted = encrypt_account_label(&shared_key, &iv, label);

        // Verify size is in valid range: 48-80 bytes (16-byte IV + 32-64 bytes encrypted)
        assert!(
            encrypted.len() >= 48 && encrypted.len() <= 80,
            "Encrypted label should be 48-80 bytes, got {}",
            encrypted.len()
        );

        let decrypted = decrypt_account_label(&shared_key, &encrypted).unwrap();

        assert_eq!(label, decrypted);
    }

    /// DIP-15 fixes `encryptedAccountLabel` at 48..=80 bytes, and the primitive
    /// must produce a valid field for ANY input. Short/empty/multi-byte labels
    /// are padded to clear the 48-byte floor and round-trip exactly (padding
    /// stripped on decrypt); the exactly-16-char case sits on the lower edge;
    /// over-long labels are truncated to stay under the 80-byte cap (and so do
    /// not round-trip in full — the documented trade-off). The original bug:
    /// a <16-char label produced a 32-byte blob and a ≥64-byte label a 96-byte
    /// blob, either of which the contract rejects and fails the broadcast.
    #[test]
    fn account_label_is_always_a_valid_48_to_80_byte_field() {
        let key = [0x42u8; 32];
        let iv = [0x11u8; 16];

        // Floor + exact round-trip for short / empty / multi-byte labels.
        for label in ["", "hi", "lunch fund", "café ☕", "🍕🍕🍕"] {
            let blob = encrypt_account_label(&key, &iv, label);
            assert!(
                (48..=80).contains(&blob.len()),
                "label {label:?} -> {} bytes, expected 48..=80",
                blob.len()
            );
            assert_eq!(decrypt_account_label(&key, &blob).unwrap(), label);
        }

        // Lower boundary: exactly 16 ASCII chars = 16 bytes -> 32 ciphertext + 16
        // IV = exactly 48.
        assert_eq!(
            encrypt_account_label(&key, &iv, "sixteen-chars-16").len(),
            48
        );

        // Ceiling: an over-long label (ASCII and multi-byte) must never exceed 80.
        assert!(encrypt_account_label(&key, &iv, &"x".repeat(500)).len() <= 80);
        assert!(encrypt_account_label(&key, &iv, &"🍕".repeat(50)).len() <= 80);
    }
}
