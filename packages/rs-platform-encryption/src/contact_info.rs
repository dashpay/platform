//! DIP-15 `contactInfo` field encryption: `encToUserId` (AES-256-ECB) and
//! `privateData` (`IV ‖ AES-256-CBC`).

use aes::Aes256;

use crate::aes::encrypt_aes_256_cbc;
use crate::error::CryptoError;

/// Encrypt a 32-byte identity id with AES-256-ECB (DIP-15
/// `contactInfo.encToUserId`).
///
/// Exactly two raw AES blocks — **no IV, no padding**. ECB is sound
/// for this one field per DIP-15's own analysis: the plaintext is
/// itself a SHA-256 output (pseudorandom, no repeated-block structure)
/// and the key — a dedicated hardened child at
/// `rootEncryptionKey/2^16'/index'` — is never reused for any other
/// purpose. Do NOT use this for anything but `encToUserId`.
pub fn encrypt_enc_to_user_id(key: &[u8; 32], to_user_id: &[u8; 32]) -> [u8; 32] {
    use aes::cipher::{BlockEncrypt, KeyInit};

    let cipher = Aes256::new(key.into());
    let mut out = *to_user_id;
    let (block1, block2) = out.split_at_mut(16);
    cipher.encrypt_block(block1.into());
    cipher.encrypt_block(block2.into());
    out
}

/// Decrypt a 32-byte `contactInfo.encToUserId` ciphertext
/// (inverse of [`encrypt_enc_to_user_id`]).
pub fn decrypt_enc_to_user_id(key: &[u8; 32], ciphertext: &[u8; 32]) -> [u8; 32] {
    use aes::cipher::{BlockDecrypt, KeyInit};

    let cipher = Aes256::new(key.into());
    let mut out = *ciphertext;
    let (block1, block2) = out.split_at_mut(16);
    cipher.decrypt_block(block1.into());
    cipher.decrypt_block(block2.into());
    out
}

/// Encrypt a `contactInfo.privateData` plaintext (CBOR bytes) as
/// `IV(16) ‖ AES-256-CBC(plaintext)` — the same prepended-IV layout
/// `encryptedPublicKey` uses (DIP-15 doesn't pin the layout for this
/// field; we adopt the same convention).
pub fn encrypt_private_data(key: &[u8; 32], iv: &[u8; 16], plaintext: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + plaintext.len() + 16);
    out.extend_from_slice(iv);
    out.extend_from_slice(&encrypt_aes_256_cbc(key, iv, plaintext));
    out
}

/// Decrypt a `contactInfo.privateData` blob (inverse of
/// [`encrypt_private_data`]).
pub fn decrypt_private_data(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>, CryptoError> {
    use crate::aes::decrypt_aes_256_cbc;

    if blob.len() < 16 {
        return Err(CryptoError::InvalidCiphertextLength);
    }
    let iv: [u8; 16] = blob[..16].try_into().expect("length checked above");
    decrypt_aes_256_cbc(key, &iv, &blob[16..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enc_to_user_id_round_trips_and_is_two_independent_blocks() {
        let key = [0x11u8; 32];
        let mut id = [0u8; 32];
        for (i, b) in id.iter_mut().enumerate() {
            *b = i as u8;
        }

        let ct = encrypt_enc_to_user_id(&key, &id);
        assert_ne!(ct, id, "ciphertext must differ from plaintext");
        assert_eq!(decrypt_enc_to_user_id(&key, &ct), id, "round trip");

        // ECB property we rely on: equal plaintext blocks → equal
        // ciphertext blocks (sound here only because identity ids are
        // hash outputs). This pins that the implementation really is
        // ECB and not CBC-with-zero-IV.
        let same_blocks = [0xAAu8; 32];
        let ct2 = encrypt_enc_to_user_id(&key, &same_blocks);
        assert_eq!(
            ct2[..16],
            ct2[16..],
            "ECB: identical blocks encrypt identically"
        );

        // Wrong key must not round-trip.
        assert_ne!(decrypt_enc_to_user_id(&[0x22u8; 32], &ct), id);
    }

    #[test]
    fn private_data_round_trips_with_prepended_iv() {
        let key = [0x33u8; 32];
        let iv = [0x44u8; 16];
        // Minimal CBOR-ish payload; the schema floor is 48 bytes of
        // ciphertext which IV(16) + one padded block satisfies — the
        // length policy lives at the document-build layer, not here.
        let plaintext = b"[\"alias\",\"note\",false] stand-in cbor";

        let blob = encrypt_private_data(&key, &iv, plaintext);
        assert_eq!(&blob[..16], &iv, "IV must be prepended verbatim");
        assert!(blob.len() >= 48, "IV + padded CBC reaches the schema floor");

        let plain = decrypt_private_data(&key, &blob).expect("decrypt");
        assert_eq!(plain, plaintext);

        // Truncated blob → typed error, not a panic.
        assert!(matches!(
            decrypt_private_data(&key, &blob[..10]),
            Err(CryptoError::InvalidCiphertextLength)
        ));
    }
}
