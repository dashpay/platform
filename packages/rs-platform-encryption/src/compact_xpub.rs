//! DIP-15 compact extended public key (`encryptedPublicKey`) — the 69-byte
//! compact plaintext, its parse/serialize, and its AES-256-CBC encryption.

use crate::aes::{decrypt_aes_256_cbc, encrypt_aes_256_cbc};
use crate::error::CryptoError;

/// Length of the DIP-15 compact extended-public-key plaintext, in bytes.
///
/// `parentFingerprint(4) ‖ chainCode(32) ‖ publicKey(33)` = 69 bytes. This is
/// the plaintext layout DIP-15 specifies for `encryptedPublicKey` and the form
/// both reference clients (iOS dash-shared-core, Android dashj) emit and
/// hard-check on receive. Encrypting exactly 69 bytes yields a 96-byte
/// ciphertext (16-byte IV + 80-byte AES-256-CBC/PKCS7 block), matching the
/// deployed contract's `minItems/maxItems: 96`.
pub const COMPACT_XPUB_LEN: usize = 69;

/// Encrypt an extended public key for DashPay contact requests (DIP-15)
///
/// # Arguments
/// * `shared_key` - 32-byte shared secret from ECDH
/// * `iv` - 16-byte initialization vector (must be randomly generated)
/// * `xpub` - Extended public key bytes to encrypt
///
/// # Returns
/// Encrypted extended public key with IV prepended (96 bytes: 16-byte IV + 80-byte encrypted data)
pub fn encrypt_extended_public_key(shared_key: &[u8; 32], iv: &[u8; 16], xpub: &[u8]) -> Vec<u8> {
    let encrypted_data = encrypt_aes_256_cbc(shared_key, iv, xpub);

    // Prepend IV to encrypted data as per DIP-15
    let mut result = Vec::with_capacity(16 + encrypted_data.len());
    result.extend_from_slice(iv);
    result.extend_from_slice(&encrypted_data);
    result
}

/// Decrypt an extended public key from DashPay contact requests (DIP-15)
///
/// # Arguments
/// * `shared_key` - 32-byte shared secret from ECDH
/// * `encrypted_data` - Encrypted extended public key with IV prepended (96 bytes total)
///
/// # Returns
/// Decrypted extended public key bytes
pub fn decrypt_extended_public_key(
    shared_key: &[u8; 32],
    encrypted_data: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if encrypted_data.len() < 16 {
        return Err(CryptoError::InvalidCiphertextLength);
    }

    // Extract IV from first 16 bytes
    let iv: [u8; 16] = encrypted_data[..16].try_into().unwrap();
    let ciphertext = &encrypted_data[16..];

    decrypt_aes_256_cbc(shared_key, &iv, ciphertext)
}

/// The three components of a DIP-15 compact extended public key.
///
/// `parent_fingerprint ‖ chain_code ‖ public_key` is the 69-byte compact
/// form DIP-15 defines for `encryptedPublicKey`. A named struct (rather
/// than a `([u8; 4], [u8; 32], [u8; 33])` tuple) keeps the component
/// meaning explicit at every call site — the three byte arrays are
/// otherwise easy to mis-read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactXpub {
    /// 4-byte fingerprint of the parent key.
    pub parent_fingerprint: [u8; 4],
    /// 32-byte chain code of the shared (account) key.
    pub chain_code: [u8; 32],
    /// 33-byte compressed secp256k1 public key.
    pub public_key: [u8; 33],
}

impl CompactXpub {
    /// Serialize to the 69-byte DIP-15 compact plaintext
    /// (`parent_fingerprint ‖ chain_code ‖ public_key`). This is the
    /// plaintext fed to [`encrypt_extended_public_key`] — *not* a
    /// BIP32/DIP-14 serialization, which carries extra
    /// version/depth/child-number metadata the wire format omits.
    pub fn to_bytes(&self) -> [u8; COMPACT_XPUB_LEN] {
        let mut out = [0u8; COMPACT_XPUB_LEN];
        out[0..4].copy_from_slice(&self.parent_fingerprint);
        out[4..36].copy_from_slice(&self.chain_code);
        out[36..69].copy_from_slice(&self.public_key);
        out
    }
}

/// Assemble the DIP-15 compact extended-public-key plaintext from its
/// three components. Thin wrapper over [`CompactXpub::to_bytes`] kept for
/// call sites that have the components loose rather than in a struct.
pub fn compact_xpub_bytes(
    parent_fingerprint: [u8; 4],
    chain_code: [u8; 32],
    public_key: [u8; 33],
) -> [u8; COMPACT_XPUB_LEN] {
    CompactXpub {
        parent_fingerprint,
        chain_code,
        public_key,
    }
    .to_bytes()
}

/// Parse a DIP-15 compact extended-public-key plaintext into a
/// [`CompactXpub`].
///
/// Inverse of [`CompactXpub::to_bytes`] / [`compact_xpub_bytes`]. Rejects
/// any input whose length is not exactly [`COMPACT_XPUB_LEN`] (69) bytes —
/// the reference clients hard-check this on receive, so a non-69-byte
/// payload is not a valid DIP-15 compact xpub and must be handled
/// separately (e.g. a legacy 78/107-byte BIP32/DIP-14 serialization) by
/// the caller.
///
/// # Errors
/// [`CryptoError::InvalidCompactXpubLength`] if `bytes.len() != 69`.
pub fn parse_compact_xpub(bytes: &[u8]) -> Result<CompactXpub, CryptoError> {
    if bytes.len() != COMPACT_XPUB_LEN {
        return Err(CryptoError::InvalidCompactXpubLength(bytes.len()));
    }

    let mut parent_fingerprint = [0u8; 4];
    let mut chain_code = [0u8; 32];
    let mut public_key = [0u8; 33];
    parent_fingerprint.copy_from_slice(&bytes[0..4]);
    chain_code.copy_from_slice(&bytes[4..36]);
    public_key.copy_from_slice(&bytes[36..69]);

    Ok(CompactXpub {
        parent_fingerprint,
        chain_code,
        public_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecdh::derive_shared_key_ecdh;
    use secp256k1::rand::{thread_rng, RngCore};
    use secp256k1::Secp256k1;

    #[test]
    fn test_extended_public_key_encryption() {
        let secp = Secp256k1::new();
        let (secret1, _public1) = secp.generate_keypair(&mut thread_rng());
        let (_secret2, public2) = secp.generate_keypair(&mut thread_rng());

        // Derive shared key
        let shared_key = derive_shared_key_ecdh(&secret1, &public2);

        // Generate random IV
        let mut iv = [0u8; 16];
        thread_rng().fill_bytes(&mut iv);

        // DIP-15 compact xpub plaintext (69 bytes). 69 → PKCS7 → 80, + 16-byte
        // IV = exactly 96 bytes, matching the contract's minItems/maxItems: 96.
        let xpub_data = vec![0x04; COMPACT_XPUB_LEN];

        // Encrypt and decrypt
        let encrypted = encrypt_extended_public_key(&shared_key, &iv, &xpub_data);

        // Verify size: 16 bytes (IV) + 80 bytes (encrypted data) = 96 bytes
        assert_eq!(encrypted.len(), 96, "Encrypted xpub should be 96 bytes");

        let decrypted = decrypt_extended_public_key(&shared_key, &encrypted).unwrap();

        assert_eq!(xpub_data, decrypted);
    }

    #[test]
    fn test_compact_xpub_round_trip() {
        // Distinct byte patterns per field so a mis-sliced offset is caught.
        let parent_fingerprint = [0x11u8, 0x22, 0x33, 0x44];
        let chain_code = [0xAAu8; 32];
        let mut pubkey = [0xBBu8; 33];
        pubkey[0] = 0x02; // compressed-pubkey prefix

        let compact = compact_xpub_bytes(parent_fingerprint, chain_code, pubkey);
        assert_eq!(compact.len(), COMPACT_XPUB_LEN);
        assert_eq!(compact.len(), 69);

        // Byte-exact layout: fingerprint ‖ chaincode ‖ pubkey.
        assert_eq!(&compact[0..4], &parent_fingerprint);
        assert_eq!(&compact[4..36], &chain_code);
        assert_eq!(&compact[36..69], &pubkey);

        let parsed = parse_compact_xpub(&compact).expect("parse 69-byte compact");
        assert_eq!(parsed.parent_fingerprint, parent_fingerprint);
        assert_eq!(parsed.chain_code, chain_code);
        assert_eq!(parsed.public_key, pubkey);
        // Struct round-trips back to the same bytes.
        assert_eq!(parsed.to_bytes(), compact);
    }

    #[test]
    fn test_encrypt_compact_xpub_is_exactly_96_bytes() {
        // The whole point of the 69-byte compact form: it encrypts to exactly
        // 96 bytes (16-byte IV + 80-byte AES-256-CBC/PKCS7), which is what the
        // deployed contract enforces. A 107-byte DIP-14 serialization would
        // yield 128 bytes and fail the contract's maxItems: 96.
        let shared_key = [0x07u8; 32];
        let iv = [0x09u8; 16];
        let plaintext = [0xCDu8; COMPACT_XPUB_LEN];

        let encrypted = encrypt_extended_public_key(&shared_key, &iv, &plaintext);
        assert_eq!(
            encrypted.len(),
            96,
            "69-byte compact plaintext must encrypt to exactly 96 bytes"
        );

        let decrypted = decrypt_extended_public_key(&shared_key, &encrypted).unwrap();
        assert_eq!(&decrypted[..], &plaintext[..]);
    }

    #[test]
    fn test_parse_compact_xpub_rejects_wrong_length() {
        // Lengths that are NOT 69 must be rejected — including the legacy 78/107
        // BIP32/DIP-14 serializations and the empty case.
        for bad_len in [0usize, 36, 68, 70, 78, 107, 128] {
            let bytes = vec![0u8; bad_len];
            let err = parse_compact_xpub(&bytes).expect_err("non-69-byte input must be rejected");
            assert!(
                matches!(err, CryptoError::InvalidCompactXpubLength(n) if n == bad_len),
                "expected InvalidCompactXpubLength({}), got {:?}",
                bad_len,
                err
            );
        }
    }
}
