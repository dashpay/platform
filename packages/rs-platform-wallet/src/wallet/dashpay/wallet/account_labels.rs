//! DIP-15 account-label encryption and decryption.

use platform_encryption::CryptoError;

use super::*;
use crate::broadcaster::TransactionBroadcaster;
use crate::error::PlatformWalletError;

// ---------------------------------------------------------------------------
// Account label encryption / decryption (DIP-15)
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> DashPayWallet<B> {
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

        let encrypted = platform_encryption::encrypt_account_label(shared_key, &iv, label);

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
        platform_encryption::decrypt_account_label(shared_key, encrypted).map_err(|e| match e {
            CryptoError::DecryptionFailed => {
                PlatformWalletError::InvalidIdentityData("Account label decryption failed".into())
            }
            CryptoError::InvalidUtf8 => PlatformWalletError::InvalidIdentityData(
                "Decrypted account label is not valid UTF-8".into(),
            ),
            CryptoError::InvalidCiphertextLength => PlatformWalletError::InvalidIdentityData(
                "Invalid encrypted account label length".into(),
            ),
        })
    }
}
