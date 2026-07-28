//! AES-256-CBC primitives (PKCS7) shared by the DIP-15 encrypted fields.

use aes::cipher::{block_padding::Pkcs7, KeyIvInit};
use aes::Aes256;
use zeroize::Zeroizing;

use crate::error::CryptoError;

type Aes256CbcEnc = cbc::Encryptor<Aes256>;
type Aes256CbcDec = cbc::Decryptor<Aes256>;

/// Encrypt data using CBC-AES-256
///
/// # Arguments
/// * `key` - 32-byte encryption key
/// * `iv` - 16-byte initialization vector (must be randomly generated and unique)
/// * `data` - Data to encrypt
///
/// # Returns
/// Encrypted data with PKCS7 padding
pub fn encrypt_aes_256_cbc(key: &[u8; 32], iv: &[u8; 16], data: &[u8]) -> Vec<u8> {
    use aes::cipher::BlockEncryptMut;

    let cipher = Aes256CbcEnc::new(key.into(), iv.into());
    let mut buffer = Vec::new();
    buffer.extend_from_slice(data);

    // Add padding
    let padding_needed = 16 - (data.len() % 16);
    buffer.resize(data.len() + padding_needed, padding_needed as u8);

    cipher
        .encrypt_padded_mut::<Pkcs7>(&mut buffer, data.len())
        .expect("encryption failed")
        .to_vec()
}

/// Decrypt data using CBC-AES-256 into memory that is zeroized on drop.
///
/// # Arguments
/// * `key` - 32-byte encryption key
/// * `iv` - 16-byte initialization vector
/// * `ciphertext` - Encrypted data to decrypt
///
/// # Returns
/// Decrypted data with padding removed. The in-place working allocation is
/// returned directly, so no intermediate plaintext allocation is left behind.
pub fn decrypt_aes_256_cbc_zeroizing(
    key: &[u8; 32],
    iv: &[u8; 16],
    ciphertext: &[u8],
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    use aes::cipher::BlockDecryptMut;

    let cipher = Aes256CbcDec::new(key.into(), iv.into());
    let mut buffer = Zeroizing::new(ciphertext.to_vec());

    let plaintext_len = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|_| CryptoError::DecryptionFailed)?
        .len();

    buffer.truncate(plaintext_len);
    Ok(buffer)
}

/// Decrypt data using CBC-AES-256.
///
/// This compatibility wrapper keeps the original `Vec<u8>` return type. Its
/// in-place working allocation is still zeroized before drop; callers that
/// retain sensitive plaintext should prefer [`decrypt_aes_256_cbc_zeroizing`].
pub fn decrypt_aes_256_cbc(
    key: &[u8; 32],
    iv: &[u8; 16],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let decrypted = decrypt_aes_256_cbc_zeroizing(key, iv, ciphertext)?;

    Ok(decrypted.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::rand::{thread_rng, RngCore};

    #[test]
    fn test_aes_encryption_decryption() {
        let key = [0u8; 32];
        let mut iv = [0u8; 16];
        thread_rng().fill_bytes(&mut iv);

        let plaintext = b"Hello, DashPay!";

        let ciphertext = encrypt_aes_256_cbc(&key, &iv, plaintext);
        let decrypted = decrypt_aes_256_cbc(&key, &iv, &ciphertext).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn should_use_zeroizing_storage_for_decrypted_plaintext() {
        let key = [0x31; 32];
        let iv = [0x42; 16];
        let plaintext = b"txMetadata plaintext";
        let ciphertext = encrypt_aes_256_cbc(&key, &iv, plaintext);

        let decrypted = decrypt_aes_256_cbc_zeroizing(&key, &iv, &ciphertext).expect("decrypt");

        fn assert_zeroizing(_: &Zeroizing<Vec<u8>>) {}
        assert_zeroizing(&decrypted);
        assert_eq!(decrypted.as_slice(), plaintext);
    }

    #[test]
    fn should_fail_invalid_padding_after_in_place_decryption() {
        let key = [0x53; 32];
        let iv = [0x64; 16];
        let plaintext = [0x75; 15];
        let ciphertext = encrypt_aes_256_cbc(&key, &iv, &plaintext);
        let mut invalid_iv = iv;
        invalid_iv[15] ^= 1;

        assert!(matches!(
            decrypt_aes_256_cbc_zeroizing(&key, &invalid_iv, &ciphertext),
            Err(CryptoError::DecryptionFailed)
        ));
    }
}
