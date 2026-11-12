//! The per-contact shielded payment address of a DashPay contact request: the
//! diversifier index a wallet derives it at, and the field that carries it.
//!
//! A DashPay identity gives every contact a different shielded payment address, each
//! a diversified address of the identity's shielded account, so a payment that
//! arrives on one is attributed to the contact it was given to. The address is never
//! allocated or stored. Its diversifier index is a keyed hash of the contact request's
//! unique key (`$ownerId`, `toUserId`, `accountReference`), under a key derived from
//! the account's full viewing key. Two devices restored from one seed derive the same
//! address for the same request, with no state to keep in step or to lose. Two
//! requests get the same address only if their indices collide in the 87 bits the
//! hash leaves free, which is negligible, and the sender checks for it before sending
//! (see [`shielded_contact_index`]). A counter cannot give that: two devices would
//! hand the same index to different contacts as a matter of course.
//!
//! This module holds what needs no shielded pool types: the index and the encoding of
//! `encryptedShieldedAddress`. The wallet turns an index into an address with the
//! account's full viewing key, and checks that a decrypted value is an address.

use crate::aes::{decrypt_aes_256_cbc, encrypt_aes_256_cbc};
use crate::error::CryptoError;
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// A raw shielded payment address: an 11-byte diversifier followed by a 32-byte
/// diversified transmission key.
pub const SHIELDED_ADDRESS_LEN: usize = 43;

/// `encryptedShieldedAddress` as this module writes it: a 16-byte IV followed by the
/// three AES blocks a 43-byte address pads to. The contract also admits 80 bytes,
/// kept for a later encoding, which [`decrypt_shielded_address`] refuses.
pub const ENCRYPTED_SHIELDED_ADDRESS_LEN: usize = 64;

/// A raw full viewing key of a shielded account: `ak || nk || rivk`.
pub const SHIELDED_FULL_VIEWING_KEY_LEN: usize = 96;

/// A diversifier index: an 88-bit little-endian integer.
pub const SHIELDED_DIVERSIFIER_INDEX_LEN: usize = 11;

/// The HMAC key that turns a full viewing key into a contact index key.
const CONTACT_INDEX_KEY_LABEL: &[u8] = b"DashPay/shielded/contact-index-key/v1";

/// What a contact index key hashes ahead of the contact request's unique key.
const CONTACT_INDEX_LABEL: &[u8] = b"DashPay/shielded/contact-index/v1";

/// Bit 87 of a diversifier index, in its last byte. Every per-contact index has it
/// set, and no profile or wallet-local index does.
const CONTACT_INDEX_HIGH_BIT: u8 = 0x80;

fn hmac_sha256(key: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut mac =
        <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC takes a key of any length");
    for part in parts {
        mac.update(part);
    }
    mac.finalize().into_bytes().into()
}

/// The key a shielded account derives its per-contact indices under:
/// `HMAC-SHA256(key = "DashPay/shielded/contact-index-key/v1", msg = full viewing key)`.
///
/// It needs the full viewing key, so a holder of the account's incoming viewing key
/// alone can see which index a payment arrived on but not which contact it belongs to.
pub fn shielded_contact_index_key(
    full_viewing_key: &[u8; SHIELDED_FULL_VIEWING_KEY_LEN],
) -> [u8; 32] {
    hmac_sha256(CONTACT_INDEX_KEY_LABEL, &[full_viewing_key])
}

/// The diversifier index of the shielded address a contact request carries, from the
/// request's unique key:
///
/// ```text
/// t = HMAC-SHA256(key = index_key,
///                 msg = "DashPay/shielded/contact-index/v1" || owner_id || to_user_id
///                       || account_reference as u64 little-endian)
/// j = t[0..11], with bit 87 set
/// ```
///
/// `owner_id` is the identity that sends the request and owns the address, `to_user_id`
/// the contact, and `account_reference` the value the request publishes, never one
/// recomputed locally. The index always lies in the per-contact range, from 2^87.
///
/// Before sending, the caller compares the index with those it has given to other
/// contacts and, on a match, sends the request under another `account_reference`.
pub fn shielded_contact_index(
    index_key: &[u8; 32],
    owner_id: &[u8; 32],
    to_user_id: &[u8; 32],
    account_reference: u64,
) -> [u8; SHIELDED_DIVERSIFIER_INDEX_LEN] {
    let t = hmac_sha256(
        index_key,
        &[
            CONTACT_INDEX_LABEL,
            owner_id,
            to_user_id,
            &account_reference.to_le_bytes(),
        ],
    );
    let mut index = [0u8; SHIELDED_DIVERSIFIER_INDEX_LEN];
    index.copy_from_slice(&t[..SHIELDED_DIVERSIFIER_INDEX_LEN]);
    index[SHIELDED_DIVERSIFIER_INDEX_LEN - 1] |= CONTACT_INDEX_HIGH_BIT;
    index
}

/// Whether a diversifier index lies in the per-contact range (2^87 and above), where
/// only [`shielded_contact_index`] places addresses. An index below it is a profile
/// address or one the wallet uses for itself, never one given to a contact.
pub fn is_shielded_contact_index(index: &[u8; SHIELDED_DIVERSIFIER_INDEX_LEN]) -> bool {
    index[SHIELDED_DIVERSIFIER_INDEX_LEN - 1] & CONTACT_INDEX_HIGH_BIT != 0
}

/// Encrypt a shielded payment address as a contact request's
/// `encryptedShieldedAddress`: the IV, then the address under AES-256-CBC with the
/// request's DIP-15 shared key, the key that encrypts its `encryptedPublicKey`.
///
/// `iv` is drawn at random for this field alone: no two fields of a request share one.
pub fn encrypt_shielded_address(
    shared_key: &[u8; 32],
    iv: &[u8; 16],
    address: &[u8; SHIELDED_ADDRESS_LEN],
) -> [u8; ENCRYPTED_SHIELDED_ADDRESS_LEN] {
    let ciphertext = encrypt_aes_256_cbc(shared_key, iv, address);
    let mut field = [0u8; ENCRYPTED_SHIELDED_ADDRESS_LEN];
    field[..16].copy_from_slice(iv);
    field[16..].copy_from_slice(&ciphertext);
    field
}

/// Decrypt a contact request's `encryptedShieldedAddress` to the 43 bytes it carries.
///
/// A value of any length but 64 is refused, the 80 bytes the contract keeps for a later
/// encoding included, and so is one whose plaintext is not exactly 43 bytes. CBC does
/// not authenticate, so a value decrypted under the wrong key can still pass: the caller
/// uses the result only if the same key decrypts the request's `encryptedPublicKey` to a
/// well-formed key and the bytes are a valid address that is neither its own nor the
/// sender's profile address.
pub fn decrypt_shielded_address(
    shared_key: &[u8; 32],
    field: &[u8],
) -> Result<[u8; SHIELDED_ADDRESS_LEN], CryptoError> {
    let (iv, ciphertext) = field
        .split_first_chunk::<16>()
        .filter(|_| field.len() == ENCRYPTED_SHIELDED_ADDRESS_LEN)
        .ok_or(CryptoError::InvalidEncryptedShieldedAddressLength(
            field.len(),
        ))?;
    let plaintext = decrypt_aes_256_cbc(shared_key, iv, ciphertext)?;
    plaintext
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::InvalidShieldedAddressLength(plaintext.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex<const N: usize>(text: &str) -> [u8; N] {
        let bytes: Vec<u8> = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("a hex digit pair"))
            .collect();
        bytes.try_into().expect("the expected length")
    }

    // The vectors of the DashPay shielded payment addresses DIP: mnemonic "abandon ...
    // about", mainnet, the shielded account of identity index 0 (account 0x40000000).
    const FULL_VIEWING_KEY: &str =
        "c0120eb660f8dd2f769e199bcc540476fec169b60f3e7508ffd42a21bd274337\
         b2301310b489575eb68a8adacfed66b88fd07b961d1ca366c7e7e6e619b29a2b\
         fce2cf9cb05a6439997f6f7ad4d9eb4b3d5a0672e571036cff9c8a600439063d";
    const INDEX_KEY: &str = "7233ea974f99a772e1f7cfeef7d1626f9d24cba429b32f5ecdabed7b14867791";
    const OWNER: [u8; 32] = [0x11; 32];
    const BOB: [u8; 32] = [0x22; 32];
    const CAROL: [u8; 32] = [0x33; 32];
    const BOB_ADDRESS: &str =
        "25611685f1aa7c3b7a2f7a758ea2f98ca16e32e20e0d3b403a1d81421f1185d67f47054ae98e55e05e839c";

    #[test]
    fn should_derive_the_contact_index_key_from_the_full_viewing_key() {
        assert_eq!(
            shielded_contact_index_key(&hex(FULL_VIEWING_KEY)),
            hex::<32>(INDEX_KEY)
        );
    }

    #[test]
    fn should_derive_the_index_of_each_contact_request() {
        let key = hex(INDEX_KEY);
        for (to_user_id, account_reference, index) in [
            (BOB, 0x0123_4567, "f90d5349d8530cf76835d0"),
            // Bit 87 is clear in the hash here (its eleventh byte is 0x14) and is set.
            (CAROL, 0x0123_4567, "a5ca8646a33e3db572ca94"),
            // The same contact under the next DIP-15 version: another address.
            (BOB, 0x1123_4567, "4d2ff123168ee0a07ff5b2"),
        ] {
            let derived = shielded_contact_index(&key, &OWNER, &to_user_id, account_reference);
            assert_eq!(derived, hex::<11>(index));
            assert!(is_shielded_contact_index(&derived));
        }
    }

    #[test]
    fn should_give_two_contacts_and_two_senders_different_indices() {
        let key = hex(INDEX_KEY);
        let to_bob = shielded_contact_index(&key, &OWNER, &BOB, 7);
        assert_ne!(to_bob, shielded_contact_index(&key, &OWNER, &CAROL, 7));
        assert_ne!(to_bob, shielded_contact_index(&key, &CAROL, &BOB, 7));
        assert_ne!(to_bob, shielded_contact_index(&key, &OWNER, &BOB, 8));
        assert_ne!(to_bob, shielded_contact_index(&[9; 32], &OWNER, &BOB, 7));
        // The inputs alone decide it.
        assert_eq!(to_bob, shielded_contact_index(&key, &OWNER, &BOB, 7));
    }

    #[test]
    fn should_keep_profile_and_wallet_indices_out_of_the_contact_range() {
        assert!(!is_shielded_contact_index(&[0; 11]));
        // 2^32 - 1, the last wallet-local index, and 2^87 - 1, the last below the range.
        assert!(!is_shielded_contact_index(&hex("ffffffff00000000000000")));
        assert!(!is_shielded_contact_index(&hex("ffffffffffffffffffff7f")));
        // 2^87, the first per-contact index.
        assert!(is_shielded_contact_index(&hex("0000000000000000000080")));
    }

    #[test]
    fn should_encrypt_an_address_to_the_64_byte_field() {
        let shared_key: [u8; 32] = core::array::from_fn(|i| i as u8);
        let iv: [u8; 16] = core::array::from_fn(|i| 0xa0 + i as u8);
        let field = encrypt_shielded_address(&shared_key, &iv, &hex(BOB_ADDRESS));
        assert_eq!(
            field,
            hex::<64>(
                "a0a1a2a3a4a5a6a7a8a9aaabacadaeafb00fd785cffaf9bd43b25189363352d5\
                 4462f4fc18e0ca7108e1effc6d615026025a763f29384d39e5b0c29020d62e97"
            )
        );
        assert_eq!(
            decrypt_shielded_address(&shared_key, &field).expect("the field decrypts"),
            hex::<43>(BOB_ADDRESS)
        );
    }

    #[test]
    fn should_refuse_a_field_of_any_other_length() {
        let shared_key = [1u8; 32];
        let field = encrypt_shielded_address(&shared_key, &[2; 16], &[3; 43]);
        for length in [0, 15, 16, 48, 63, 65, 80] {
            let mut other = field.to_vec();
            other.resize(length, 0);
            assert!(
                matches!(
                    decrypt_shielded_address(&shared_key, &other),
                    Err(CryptoError::InvalidEncryptedShieldedAddressLength(l)) if l == length
                ),
                "a field of {length} bytes"
            );
        }
    }

    #[test]
    fn should_refuse_a_plaintext_that_is_not_43_bytes() {
        let shared_key = [1u8; 32];
        let iv = [2u8; 16];
        // 33 to 47 bytes all pad to three blocks, so the field length cannot tell them apart.
        for length in [33, 42, 44, 47] {
            let mut field = iv.to_vec();
            field.extend(encrypt_aes_256_cbc(&shared_key, &iv, &vec![3; length]));
            assert_eq!(field.len(), ENCRYPTED_SHIELDED_ADDRESS_LEN);
            assert!(
                matches!(
                    decrypt_shielded_address(&shared_key, &field),
                    Err(CryptoError::InvalidShieldedAddressLength(l)) if l == length
                ),
                "a plaintext of {length} bytes"
            );
        }
    }

    #[test]
    fn should_refuse_a_field_whose_padding_does_not_decrypt() {
        let field = encrypt_shielded_address(&[1; 32], &[2; 16], &[3; 43]);
        // Under another key the last block almost never ends in valid padding; this one
        // does not.
        assert!(matches!(
            decrypt_shielded_address(&[4; 32], &field),
            Err(CryptoError::DecryptionFailed)
        ));
    }
}
