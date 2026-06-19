//! DIP-15 account-label encryption and decryption.

use platform_encryption::CryptoError;

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;

/// kotlin/dashj pad the label to at least 16 characters with trailing spaces
/// before encrypting, and always emit it. This keeps the AES-256-CBC
/// ciphertext (IV + blocks) ≥ 48 bytes — the contract's `encryptedAccountLabel`
/// floor — even for a short or empty label (empty → 16 spaces). Matching it is
/// required for cross-client interop (a label shorter than 16 chars would
/// otherwise produce a 32-byte blob the contract rejects).
const ACCOUNT_LABEL_MIN_CHARS: usize = 16;

/// Pad `label` to at least [`ACCOUNT_LABEL_MIN_CHARS`] chars with spaces
/// (no-op when it is already ≥ that). Mirrors kotlin's `padEnd(16, ' ')`.
fn pad_account_label(label: &str) -> String {
    let chars = label.chars().count();
    if chars >= ACCOUNT_LABEL_MIN_CHARS {
        label.to_string()
    } else {
        let mut s = String::with_capacity(label.len() + (ACCOUNT_LABEL_MIN_CHARS - chars));
        s.push_str(label);
        s.extend(std::iter::repeat_n(' ', ACCOUNT_LABEL_MIN_CHARS - chars));
        s
    }
}

// ---------------------------------------------------------------------------
// Account label encryption / decryption (DIP-15)
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> IdentityWallet<B> {
    /// Encrypt an account label using CBC-AES-256 with a shared ECDH key.
    ///
    /// Uses the `platform_encryption` crate which prepends a random 16-byte IV
    /// to the ciphertext.
    ///
    /// # Arguments
    ///
    /// * `label`      - The account label to encrypt.
    /// * `shared_key` - 32-byte shared secret derived via ECDH.
    ///
    /// # Returns
    ///
    /// Encrypted label bytes (48-80 bytes: 16-byte IV + 32-64 byte ciphertext).
    pub fn encrypt_account_label(
        label: &str,
        shared_key: &[u8; 32],
    ) -> Result<Vec<u8>, PlatformWalletError> {
        use dashcore::secp256k1::rand::thread_rng;
        use dashcore::secp256k1::rand::RngCore;
        let mut iv = [0u8; 16];
        thread_rng().fill_bytes(&mut iv);

        // Pad to ≥16 chars (kotlin/dashj convention) so the ciphertext clears
        // the contract's 48-byte `encryptedAccountLabel` floor and interops.
        let padded = pad_account_label(label);
        let encrypted = platform_encryption::encrypt_account_label(shared_key, &iv, &padded);

        Ok(encrypted)
    }

    /// Decrypt an account label using CBC-AES-256 with a shared ECDH key.
    ///
    /// The first 16 bytes of `encrypted` are taken as the IV.
    ///
    /// # Arguments
    ///
    /// * `encrypted`  - Encrypted label bytes (48-80 bytes).
    /// * `shared_key` - 32-byte shared secret derived via ECDH.
    ///
    /// # Returns
    ///
    /// The decrypted label string.
    pub fn decrypt_account_label(
        encrypted: &[u8],
        shared_key: &[u8; 32],
    ) -> Result<String, PlatformWalletError> {
        let label =
            platform_encryption::decrypt_account_label(shared_key, encrypted).map_err(|e| {
                match e {
                    CryptoError::DecryptionFailed => PlatformWalletError::InvalidIdentityData(
                        "Account label decryption failed".into(),
                    ),
                    CryptoError::InvalidUtf8 => PlatformWalletError::InvalidIdentityData(
                        "Decrypted account label is not valid UTF-8".into(),
                    ),
                    CryptoError::InvalidCiphertextLength => {
                        PlatformWalletError::InvalidIdentityData(
                            "Invalid encrypted account label length".into(),
                        )
                    }
                    // Not reachable from account-label decryption (that path never
                    // parses a compact xpub), but the match must stay exhaustive.
                    CryptoError::InvalidCompactXpubLength(len) => {
                        PlatformWalletError::InvalidIdentityData(format!(
                            "Unexpected compact-xpub length error during label decryption: {len}"
                        ))
                    }
                }
            })?;
        // Strip the trailing space padding added on encrypt (kotlin convention).
        Ok(label.trim_end_matches(' ').to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_account_label_matches_kotlin_pad_end_16() {
        assert_eq!(pad_account_label("hi"), "hi              "); // 2 + 14 spaces = 16
        assert_eq!(pad_account_label("").len(), 16); // empty → 16 spaces
        assert_eq!(pad_account_label("exactly-16-chars"), "exactly-16-chars"); // ≥16: untouched
        assert_eq!(
            pad_account_label("a longer label than sixteen"),
            "a longer label than sixteen"
        );
    }

    /// A short label encrypts to ≥48 bytes (clearing the contract floor) and
    /// round-trips back to the original (padding stripped) — the bug was that
    /// labels < 16 chars produced a 32-byte blob the contract rejects.
    #[test]
    fn short_and_empty_labels_clear_the_48_byte_floor_and_round_trip() {
        use platform_encryption::decrypt_account_label as dec;
        let key = [0x42u8; 32];
        let iv = [0x11u8; 16];
        for label in ["", "hi", "lunch fund"] {
            let blob =
                platform_encryption::encrypt_account_label(&key, &iv, &pad_account_label(label));
            assert!(
                (48..=80).contains(&blob.len()),
                "label {label:?} blob len {} not in 48..=80",
                blob.len()
            );
            let decrypted = dec(&key, &blob).expect("decrypt");
            assert_eq!(decrypted.trim_end_matches(' '), label);
        }
    }
}
