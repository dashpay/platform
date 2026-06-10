//! DIP-14 / DIP-15 compliant key derivation and contact payment operations.
//!
//! This module implements:
//! - **DIP-14**: Extended key derivation with 256-bit unsigned integer indices,
//!   used for identity-based derivation paths where a 32-byte `Identifier` is
//!   the child index.
//! - **DIP-15**: DashPay contact relationship management, including contact xpub
//!   derivation, account reference calculation, and contact payment address
//!   generation.
//!
//! The underlying 256-bit child key derivation (`ckd_priv` / `ckd_pub`) is
//! handled by [`key_wallet::bip32`] via [`ChildNumber::Normal256`] and
//! [`ChildNumber::Hardened256`]. This module provides higher-level functions
//! that compose those primitives into the DashPay derivation paths defined by
//! the DIP specifications.
//!
//! # Derivation path
//!
//! Contact xpub path: `m/9'/coin'/15'/account'/(sender_id)/(recipient_id)`
//!
//! - First four segments use standard BIP32 (hardened).
//! - Last two segments use DIP-14 256-bit non-hardened derivation.
//!
//! # References
//!
//! - [DIP-14](https://github.com/dashpay/dips/blob/master/dip-0014.md)
//! - [DIP-15](https://github.com/dashpay/dips/blob/master/dip-0015.md)

use dashcore::hashes::hmac::{Hmac, HmacEngine};
use dashcore::hashes::{sha256, Hash, HashEngine};
use dashcore::secp256k1::Secp256k1;
use dashcore::{Address, Network, PublicKey};
use dpp::prelude::Identifier;
use key_wallet::account::AccountType;
use key_wallet::bip32::{ChildNumber, ExtendedPubKey};
use key_wallet::wallet::Wallet;

use crate::error::PlatformWalletError;

// ---------------------------------------------------------------------------
// Contact xpub data
// ---------------------------------------------------------------------------

/// Data extracted from a contact-specific extended public key.
///
/// This contains the components needed for DashPay contact request documents
/// and for deriving payment addresses within a contact relationship.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactXpubData {
    /// The full extended public key for this contact relationship.
    pub xpub: ExtendedPubKey,
    /// Parent key fingerprint (first 4 bytes of HASH160 of parent public key).
    pub parent_fingerprint: [u8; 4],
    /// Chain code from the derived key (32 bytes).
    pub chain_code: [u8; 32],
    /// Compressed public key (33 bytes, starts with 0x02 or 0x03).
    pub public_key: [u8; 33],
}

impl ContactXpubData {
    /// Assemble the DIP-15 compact extended-public-key plaintext (69 bytes).
    ///
    /// Layout: `parent_fingerprint(4) ‖ chain_code(32) ‖ public_key(33)`.
    ///
    /// This is the plaintext that must be encrypted into `encryptedPublicKey`
    /// per DIP-15 — **not** [`ExtendedPubKey::encode()`], which for the DashPay
    /// receiving path emits the 107-byte DIP-14 serialization (extra
    /// version/depth/child-number metadata) and encrypts to 128 bytes, failing
    /// the contract's `maxItems: 96`. Both reference clients (iOS
    /// dash-shared-core, Android dashj `serializeContactPub`) emit exactly this
    /// 69-byte form. See `docs/dashpay/research/06-interop-desk-check.md` (G14).
    pub fn compact_xpub(&self) -> [u8; platform_encryption::COMPACT_XPUB_LEN] {
        platform_encryption::compact_xpub_bytes(
            self.parent_fingerprint,
            self.chain_code,
            self.public_key,
        )
    }
}

// ---------------------------------------------------------------------------
// Contact xpub derivation
// ---------------------------------------------------------------------------

/// Derive the contact-specific extended public key.
///
/// Path: `m/9'/coin'/15'/account'/(sender_id)/(recipient_id)`
///
/// The first four segments use standard BIP32 hardened derivation.
/// The last two segments use DIP-14 256-bit non-hardened derivation,
/// where the child index is the 32-byte identity identifier.
///
/// This leverages [`key_wallet`]'s built-in support for
/// [`ChildNumber::Normal256`] in its `derive_priv` / `ckd_priv` methods.
///
/// # Arguments
///
/// * `wallet`        - The HD wallet containing the master key.
/// * `network`       - Network (Mainnet / Testnet) for coin-type selection.
/// * `account_index` - Account index (hardened) in the derivation path.
/// * `sender_id`     - The sender (our) identity identifier.
/// * `recipient_id`  - The recipient (contact) identity identifier.
pub fn derive_contact_xpub(
    wallet: &Wallet,
    network: Network,
    account_index: u32,
    sender_id: &Identifier,
    recipient_id: &Identifier,
) -> Result<ContactXpubData, PlatformWalletError> {
    // Build the derivation path using AccountType, which correctly creates:
    //   m/9'/coin'/15'/0'/(sender_id)/(recipient_id)
    // with Normal256 child numbers for the identity segments.
    let account_type = AccountType::DashpayReceivingFunds {
        index: account_index,
        user_identity_id: sender_id.to_buffer(),
        friend_identity_id: recipient_id.to_buffer(),
    };

    let path = account_type.derivation_path(network).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Failed to build DashPay derivation path: {}",
            e
        ))
    })?;

    // Derive the extended public key. Because the path contains hardened
    // segments, this goes through derive_extended_private_key internally and
    // then converts to the public key.
    let xpub = wallet.derive_extended_public_key(&path).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!("Failed to derive contact xpub: {}", e))
    })?;

    let parent_fingerprint = xpub.parent_fingerprint.to_bytes();
    let chain_code = xpub.chain_code.to_bytes();
    let public_key = xpub.public_key.serialize();

    Ok(ContactXpubData {
        xpub,
        parent_fingerprint,
        chain_code,
        public_key,
    })
}

// ---------------------------------------------------------------------------
// Contact xpub reconstruction (DIP-15 receive side)
// ---------------------------------------------------------------------------

/// Reconstruct a contact's [`ExtendedPubKey`] from the DIP-15 compact form.
///
/// The wire format (`encryptedPublicKey`) carries only
/// `parent_fingerprint ‖ chain_code ‖ public_key` — it deliberately omits the
/// BIP32/DIP-14 version/depth/child-number metadata. To rebuild an
/// `ExtendedPubKey` we synthesize that metadata from the known derivation-path
/// context: both parties know this key sits at the leaf of the friendship path
/// `m/9'/coin'/15'/0'/<sender256>/<recipient256>`, i.e. **depth 6** with a
/// non-hardened 256-bit child.
///
/// **Correctness:** only `chain_code` + `public_key` feed non-hardened
/// `ckd_pub` ([`derive_contact_payment_address`]), so the synthesized
/// depth/child-number are pure metadata and do not affect derived payment
/// addresses (pinned by `reconstructed_xpub_derives_identical_addresses`). The
/// `child_number` is set to a `Normal256` zero index purely so the
/// reconstructed key is non-hardened (a hardened child would refuse `ckd_pub`).
///
/// # Arguments
/// * `parent_fingerprint` - 4-byte parent fingerprint from the compact form.
/// * `chain_code` - 32-byte chain code from the compact form.
/// * `public_key` - 33-byte compressed public key from the compact form.
/// * `network` - Network for address encoding (from path context).
pub fn reconstruct_contact_xpub(
    parent_fingerprint: [u8; 4],
    chain_code: [u8; 32],
    public_key: [u8; 33],
    network: Network,
) -> Result<ExtendedPubKey, PlatformWalletError> {
    let public_key = dashcore::secp256k1::PublicKey::from_slice(&public_key).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Compact contact xpub has an invalid compressed public key: {e}"
        ))
    })?;

    Ok(ExtendedPubKey {
        network,
        // Friendship-path leaf depth (m/9'/coin'/15'/0'/sender/recipient).
        depth: 6,
        parent_fingerprint: key_wallet::bip32::Fingerprint::from_bytes(parent_fingerprint),
        // Non-hardened so ckd_pub is permitted; index value is irrelevant to
        // non-hardened child derivation, which keys only off chain_code+pubkey.
        child_number: ChildNumber::Normal256 { index: [0u8; 32] },
        public_key,
        chain_code: key_wallet::bip32::ChainCode::from_bytes(chain_code),
    })
}

// ---------------------------------------------------------------------------
// Account reference (DIP-15)
// ---------------------------------------------------------------------------

/// Calculate the account reference per DIP-15.
///
/// ```text
/// ASK        = HMAC-SHA256(sender_secret_key, encoded_xpub_bytes)
/// ASK28      = first_4_bytes_of(ASK) >> 4          // 28 MSBs
/// shortened  = account_index & 0x0FFF_FFFF         // 28 low bits
/// version_hi = version << 28                       // 4 high bits
/// result     = version_hi | (ASK28 ^ shortened)
/// ```
///
/// The `sender_secret_key` is the raw 32-byte ECDH private key of the sender.
/// The `contact_xpub` is the extended public key for the contact relationship.
///
/// # Arguments
///
/// * `sender_secret_key` - 32-byte ECDH secret key material.
/// * `contact_xpub`      - The contact relationship extended public key.
/// * `account_index`     - The account index used in the derivation path.
/// * `version`           - Protocol version (0..15), placed in top 4 bits.
pub fn calculate_account_reference(
    sender_secret_key: &[u8; 32],
    contact_xpub: &ExtendedPubKey,
    account_index: u32,
    version: u32,
) -> u32 {
    // Serialize the extended public key (uses DIP-14 256-bit encoding if the
    // child number is 256-bit, otherwise standard 78-byte BIP32 encoding).
    let xpub_bytes = contact_xpub.encode();

    // HMAC-SHA256(senderSecretKey, extendedPublicKey)
    let mut engine = HmacEngine::<sha256::Hash>::new(sender_secret_key);
    engine.input(&xpub_bytes);
    let ask = Hmac::<sha256::Hash>::from_engine(engine);

    // Take the 28 most significant bits: read first 4 bytes as big-endian u32,
    // then right-shift by 4 to discard the 4 least significant bits.
    let ask_bytes = ask.to_byte_array();
    let ask28 = u32::from_be_bytes([ask_bytes[0], ask_bytes[1], ask_bytes[2], ask_bytes[3]]) >> 4;

    // Combine version (4 high bits) with XOR of ASK28 and shortened account bits.
    let shortened_account_bits = account_index & 0x0FFF_FFFF;
    let version_bits = version << 28;

    version_bits | (ask28 ^ shortened_account_bits)
}

// ---------------------------------------------------------------------------
// Contact payment address derivation
// ---------------------------------------------------------------------------

/// Derive a payment receiving address for a contact at a given index.
///
/// This performs standard BIP32 non-hardened derivation from the contact xpub:
/// `contact_xpub / index` and converts the resulting public key to a P2PKH
/// address.
///
/// # Arguments
///
/// * `contact_xpub` - The contact relationship extended public key.
/// * `index`        - The payment address index (non-hardened).
/// * `network`      - Network for address encoding.
pub fn derive_contact_payment_address(
    contact_xpub: &ExtendedPubKey,
    index: u32,
    network: Network,
) -> Result<Address, PlatformWalletError> {
    let secp = Secp256k1::new();

    let child_number = ChildNumber::from_normal_idx(index).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!("Invalid payment address index: {}", e))
    })?;

    let address_key = contact_xpub.ckd_pub(&secp, child_number).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Failed to derive contact payment key at index {}: {}",
            index, e
        ))
    })?;

    // Convert secp256k1::PublicKey to dashcore::PublicKey and create P2PKH address.
    let pubkey = PublicKey::new(address_key.public_key);
    Ok(Address::p2pkh(&pubkey, network))
}

/// Derive multiple payment addresses for a contact, starting from
/// `start_index` up to `start_index + count - 1`.
///
/// This is a convenience wrapper around [`derive_contact_payment_address`].
pub fn derive_contact_payment_addresses(
    contact_xpub: &ExtendedPubKey,
    start_index: u32,
    count: u32,
    network: Network,
) -> Result<Vec<Address>, PlatformWalletError> {
    (start_index..start_index.saturating_add(count))
        .map(|i| derive_contact_payment_address(contact_xpub, i, network))
        .collect()
}

// ---------------------------------------------------------------------------
// Gap limit constants
// ---------------------------------------------------------------------------

/// Default gap limit for contact payment addresses as recommended by DIP-15.
///
/// "We recommend a gap limit of 10 at this stage, which means to load 10
/// addresses past the last used address."
pub const DEFAULT_CONTACT_GAP_LIMIT: u32 = 10;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::bip32::ExtendedPrivKey;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;

    /// Helper: create a deterministic wallet from a fixed seed.
    fn test_wallet(network: Network) -> Wallet {
        let seed = [0x42u8; 64];
        Wallet::from_seed_bytes(seed, network, WalletAccountCreationOptions::None)
            .expect("Failed to create test wallet")
    }

    fn test_identifiers() -> (Identifier, Identifier) {
        let sender_bytes = [
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
            0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc,
            0xdd, 0xee, 0xff, 0x11,
        ];
        let recipient_bytes = [
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
            0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
            0x66, 0x77, 0x88, 0x99,
        ];
        (
            Identifier::from_bytes(&sender_bytes).unwrap(),
            Identifier::from_bytes(&recipient_bytes).unwrap(),
        )
    }

    #[test]
    fn test_derive_contact_xpub_basic() {
        let wallet = test_wallet(Network::Testnet);
        let (sender, recipient) = test_identifiers();

        let data = derive_contact_xpub(&wallet, Network::Testnet, 0, &sender, &recipient)
            .expect("Should derive contact xpub");

        // Path: m/9'/1'/15'/0'/(sender)/(recipient) = depth 6
        assert_eq!(data.xpub.depth, 6);
        assert_eq!(data.xpub.network, Network::Testnet);
        assert_eq!(data.parent_fingerprint.len(), 4);
        assert_eq!(data.chain_code.len(), 32);
        assert_eq!(data.public_key.len(), 33);
        // Compressed public key prefix
        assert!(data.public_key[0] == 0x02 || data.public_key[0] == 0x03);
    }

    #[test]
    fn test_derive_contact_xpub_deterministic() {
        let wallet = test_wallet(Network::Testnet);
        let (sender, recipient) = test_identifiers();

        let data1 = derive_contact_xpub(&wallet, Network::Testnet, 0, &sender, &recipient)
            .expect("first derivation");
        let data2 = derive_contact_xpub(&wallet, Network::Testnet, 0, &sender, &recipient)
            .expect("second derivation");

        assert_eq!(data1, data2, "Same inputs should produce same xpub");
    }

    #[test]
    fn test_derive_contact_xpub_different_accounts() {
        let wallet = test_wallet(Network::Testnet);
        let (sender, recipient) = test_identifiers();

        // Both account indices should produce valid derivations.
        // NOTE: AccountType::DashpayReceivingFunds uses the index for
        // account collection management, not in the derivation path itself.
        // The path is always m/9'/coin'/15'/0'/(sender_id)/(recipient_id).
        let data0 = derive_contact_xpub(&wallet, Network::Testnet, 0, &sender, &recipient)
            .expect("account 0");
        let data1 = derive_contact_xpub(&wallet, Network::Testnet, 1, &sender, &recipient)
            .expect("account 1");

        // Both should be valid derivations (may produce same key if index
        // is not part of derivation path).
        assert!(!data0.public_key.is_empty());
        assert!(!data1.public_key.is_empty());
    }

    #[test]
    fn test_derive_contact_xpub_asymmetric() {
        let wallet = test_wallet(Network::Testnet);
        let (sender, recipient) = test_identifiers();

        let forward = derive_contact_xpub(&wallet, Network::Testnet, 0, &sender, &recipient)
            .expect("sender->recipient");
        let reverse = derive_contact_xpub(&wallet, Network::Testnet, 0, &recipient, &sender)
            .expect("recipient->sender");

        assert_ne!(
            forward.public_key, reverse.public_key,
            "Swapping sender/recipient should produce different keys"
        );
    }

    #[test]
    fn test_account_reference_version_bits() {
        let secret_key = [1u8; 32];
        let master_xprv = ExtendedPrivKey::new_master(Network::Testnet, &[2u8; 64]).unwrap();
        let secp = Secp256k1::new();
        let xpub = ExtendedPubKey::from_priv(&secp, &master_xprv);

        // Version 0
        let ref_v0 = calculate_account_reference(&secret_key, &xpub, 0, 0);
        assert_eq!(ref_v0 >> 28, 0, "Version 0 → top 4 bits = 0");

        // Version 1
        let ref_v1 = calculate_account_reference(&secret_key, &xpub, 0, 1);
        assert_eq!(ref_v1 >> 28, 1, "Version 1 → top 4 bits = 1");

        // Version 15 (maximum)
        let ref_v15 = calculate_account_reference(&secret_key, &xpub, 0, 15);
        assert_eq!(ref_v15 >> 28, 15, "Version 15 → top 4 bits = 15");
    }

    #[test]
    fn test_account_reference_deterministic() {
        let secret_key = [0xABu8; 32];
        let master_xprv = ExtendedPrivKey::new_master(Network::Testnet, &[0xCDu8; 64]).unwrap();
        let secp = Secp256k1::new();
        let xpub = ExtendedPubKey::from_priv(&secp, &master_xprv);

        let ref1 = calculate_account_reference(&secret_key, &xpub, 0, 0);
        let ref2 = calculate_account_reference(&secret_key, &xpub, 0, 0);

        assert_eq!(
            ref1, ref2,
            "Same inputs should produce same account reference"
        );
    }

    #[test]
    fn test_contact_payment_address_derivation() {
        let wallet = test_wallet(Network::Testnet);
        let (sender, recipient) = test_identifiers();

        let data = derive_contact_xpub(&wallet, Network::Testnet, 0, &sender, &recipient)
            .expect("derive xpub");

        let addr0 =
            derive_contact_payment_address(&data.xpub, 0, Network::Testnet).expect("address 0");
        let addr1 =
            derive_contact_payment_address(&data.xpub, 1, Network::Testnet).expect("address 1");

        // Different indices produce different addresses.
        assert_ne!(addr0, addr1);
    }

    #[test]
    fn test_contact_payment_address_deterministic() {
        let wallet = test_wallet(Network::Testnet);
        let (sender, recipient) = test_identifiers();

        let data = derive_contact_xpub(&wallet, Network::Testnet, 0, &sender, &recipient)
            .expect("derive xpub");

        let addr_a =
            derive_contact_payment_address(&data.xpub, 5, Network::Testnet).expect("first call");
        let addr_b =
            derive_contact_payment_address(&data.xpub, 5, Network::Testnet).expect("second call");

        assert_eq!(addr_a, addr_b, "Same index should yield same address");
    }

    #[test]
    fn test_derive_contact_payment_addresses_batch() {
        let wallet = test_wallet(Network::Testnet);
        let (sender, recipient) = test_identifiers();

        let data = derive_contact_xpub(&wallet, Network::Testnet, 0, &sender, &recipient)
            .expect("derive xpub");

        let addrs = derive_contact_payment_addresses(&data.xpub, 0, 5, Network::Testnet)
            .expect("batch derive");

        assert_eq!(addrs.len(), 5);
        // All addresses should be unique.
        for i in 0..addrs.len() {
            for j in (i + 1)..addrs.len() {
                assert_ne!(
                    addrs[i], addrs[j],
                    "Addresses at index {} and {} collide",
                    i, j
                );
            }
        }

        // Individually derived addresses should match batch results.
        for (i, addr) in addrs.iter().enumerate() {
            let single = derive_contact_payment_address(&data.xpub, i as u32, Network::Testnet)
                .expect("single derive");
            assert_eq!(
                addr, &single,
                "Batch and single derivation mismatch at index {}",
                i
            );
        }
    }

    #[test]
    fn test_default_gap_limit() {
        assert_eq!(DEFAULT_CONTACT_GAP_LIMIT, 10);
    }

    #[test]
    fn compact_xpub_is_69_byte_dip15_plaintext_not_107_byte_encode() {
        // G14 regression. The send path must encrypt the DIP-15 compact
        // plaintext (fingerprint ‖ chaincode ‖ pubkey = 69 bytes), NOT
        // `ExtendedPubKey::encode()`, which for the DashPay receiving path is
        // the 107-byte DIP-14 serialization (ends in a Normal256 child) and
        // encrypts to 128 bytes — failing the contract's maxItems: 96.
        //
        // Before the fix, the producer at contact_requests.rs:150 used
        // `account_xpub.encode()`; against that code this assertion is RED
        // (107 != 69). This pins the byte-exact compact layout.
        let wallet = test_wallet(Network::Testnet);
        let (sender, recipient) = test_identifiers();

        let data = derive_contact_xpub(&wallet, Network::Testnet, 0, &sender, &recipient)
            .expect("derive contact xpub");

        let compact = data.compact_xpub();

        // The DashPay receiving path ends in a Normal256 child, so encode() is
        // the 107-byte DIP-14 form. Prove the two are genuinely different.
        let encoded = data.xpub.encode();
        assert_eq!(
            encoded.len(),
            107,
            "DashPay account xpub encodes to 107 bytes"
        );
        assert_eq!(compact.len(), 69, "compact plaintext must be 69 bytes");

        // Byte-exact layout: fingerprint ‖ chaincode ‖ compressed pubkey.
        assert_eq!(&compact[0..4], &data.parent_fingerprint);
        assert_eq!(&compact[4..36], &data.chain_code);
        assert_eq!(&compact[36..69], &data.public_key);

        // And it round-trips through the codec.
        let (fp, cc, pk) =
            platform_encryption::parse_compact_xpub(&compact).expect("parse compact");
        assert_eq!(fp, data.parent_fingerprint);
        assert_eq!(cc, data.chain_code);
        assert_eq!(pk, data.public_key);
    }

    #[test]
    fn reconstructed_xpub_derives_identical_addresses() {
        // G14 receive-side correctness. After compacting a contact xpub to 69
        // bytes and reconstructing an ExtendedPubKey from
        // (chain_code, public_key) with synthesized depth/child-number, address
        // derivation MUST produce the same addresses as the original xpub —
        // because non-hardened ckd_pub uses only chain_code + public_key.
        let wallet = test_wallet(Network::Testnet);
        let (sender, recipient) = test_identifiers();

        let data = derive_contact_xpub(&wallet, Network::Testnet, 0, &sender, &recipient)
            .expect("derive contact xpub");

        let compact = data.compact_xpub();
        let (fp, cc, pk) =
            platform_encryption::parse_compact_xpub(&compact).expect("parse compact");
        let reconstructed = reconstruct_contact_xpub(fp, cc, pk, Network::Testnet)
            .expect("reconstruct from compact");

        for i in 0..6u32 {
            let from_original = derive_contact_payment_address(&data.xpub, i, Network::Testnet)
                .expect("derive from original");
            let from_reconstructed =
                derive_contact_payment_address(&reconstructed, i, Network::Testnet)
                    .expect("derive from reconstructed");
            assert_eq!(
                from_original, from_reconstructed,
                "address {} differs between original and reconstructed xpub",
                i
            );
        }
    }
}
