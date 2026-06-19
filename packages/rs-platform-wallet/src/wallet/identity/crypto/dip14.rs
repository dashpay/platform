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
    /// The DIP-15 compact components (`parent_fingerprint ‖ chain_code ‖
    /// public_key`) of `xpub` — the wire form fed into `encryptedPublicKey`.
    /// Reuses [`platform_encryption::CompactXpub`] rather than duplicating
    /// the three byte arrays.
    pub compact: platform_encryption::CompactXpub,
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
    /// 69-byte form. See `docs/dashpay/research/06-interop-desk-check.md`.
    pub fn compact_xpub(&self) -> [u8; platform_encryption::COMPACT_XPUB_LEN] {
        self.compact.to_bytes()
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

    let compact = platform_encryption::CompactXpub {
        parent_fingerprint: xpub.parent_fingerprint.to_bytes(),
        chain_code: xpub.chain_code.to_bytes(),
        public_key: xpub.public_key.serialize(),
    };

    Ok(ContactXpubData { xpub, compact })
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
/// * `compact` - the parsed [`CompactXpub`] components from the wire form.
/// * `network` - Network for address encoding (from path context).
pub fn reconstruct_contact_xpub(
    compact: platform_encryption::CompactXpub,
    network: Network,
) -> Result<ExtendedPubKey, PlatformWalletError> {
    let public_key =
        dashcore::secp256k1::PublicKey::from_slice(&compact.public_key).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "Compact contact xpub has an invalid compressed public key: {e}"
            ))
        })?;

    Ok(ExtendedPubKey {
        network,
        // Friendship-path leaf depth (m/9'/coin'/15'/0'/sender/recipient).
        depth: 6,
        parent_fingerprint: key_wallet::bip32::Fingerprint::from_bytes(compact.parent_fingerprint),
        // Non-hardened so ckd_pub is permitted; index value is irrelevant to
        // non-hardened child derivation, which keys only off chain_code+pubkey.
        child_number: ChildNumber::Normal256 { index: [0u8; 32] },
        public_key,
        chain_code: key_wallet::bip32::ChainCode::from_bytes(compact.chain_code),
    })
}

// ---------------------------------------------------------------------------
// Account reference (DIP-15)
// ---------------------------------------------------------------------------

/// The 28-bit account-secret-key mask shared by
/// [`calculate_account_reference`] / [`unmask_account_reference`].
///
/// ```text
/// ASK   = HMAC-SHA256(sender_secret_key, compact_xpub_69_bytes)
/// ASK28 = u32_be(ASK[28..32]) >> 4
/// ```
///
/// Two interop-critical conventions, pinned against the reference
/// clients (research/06 §3 — the desk-check that found our previous
/// helper diverged on both axes):
///
/// - **HMAC input is the 69-byte DIP-15 compact form**
///   (`fingerprint ‖ chain_code ‖ pubkey`), the same plaintext that
///   goes inside `encryptedPublicKey`. Both DashWallet iOS
///   (dash-shared-core `keys.rs`) and Android (dashj
///   `serializeContactPub`) agree on this; our old helper hashed the
///   107-byte DIP-14 `encode()`.
/// - **ASK28 byte order matches iOS dash-shared-core.** DIP-15 leaves the
///   extraction ambiguous ("28 most significant bits of ASK", no byte order),
///   and FOUR conventions exist in the wild (verified 2026-06 against the live
///   sources):
///     - ours / iOS dash-shared-core: `be(ASK[28..32]) >> 4` — iOS reverses the
///       digest then reads LE first-4, which is *algebraically identical* to
///       our BE last-4;
///     - Android (kotlin-platform): `le(ASK[0..4]) >> 4`;
///     - dash-evo-tool AND the literal DIP reading (the digest as a 256-bit
///       big-endian integer): `be(ASK[0..4]) >> 4`.
///   They give different 28-bit values — but DIP-15 defines `accountReference`
///   as a one-time-pad obfuscation that **recipients MUST ignore**; only the
///   original sender ever un-masks it (to read the rotation version on
///   re-send). So every convention round-trips for its own sender and there is
///   **no on-chain interop failure** to observe and no canonical value to
///   match. We match iOS (the most-deployed DashPay wallet) so our sent
///   requests are bit-identical to the incumbent's — note this is *not*
///   "matching the DIP literal" (that reading equals dash-evo-tool).
fn account_secret_key_28(sender_secret_key: &[u8; 32], compact_xpub: &[u8]) -> u32 {
    let mut engine = HmacEngine::<sha256::Hash>::new(sender_secret_key);
    engine.input(compact_xpub);
    let ask = Hmac::<sha256::Hash>::from_engine(engine);
    extract_ask28(&ask.to_byte_array())
}

/// Extract `ASK28` from the 32-byte HMAC digest using the iOS dash-shared-core
/// convention: `be(ASK[28..32]) >> 4`. See [`account_secret_key_28`] for the
/// four-way convention split and why this choice is interop-neutral.
fn extract_ask28(ask_bytes: &[u8; 32]) -> u32 {
    u32::from_be_bytes([ask_bytes[28], ask_bytes[29], ask_bytes[30], ask_bytes[31]]) >> 4
}

/// Calculate the masked `accountReference` per DIP-15.
///
/// ```text
/// result = (version << 28) | (ASK28 ^ (account_index & 0x0FFF_FFFF))
/// ```
///
/// The top 4 bits carry the rotation `version` (bumped each time the
/// sender re-keys the friendship after an identity-key rotation); the
/// low 28 bits are the account index masked by a PRF of the contact
/// xpub so observers can't correlate accounts across requests. The
/// contract's unique index `($ownerId, toUserId, accountReference)`
/// means the version bump is what makes a superseding request
/// broadcastable at all.
///
/// # Arguments
///
/// * `sender_secret_key` - 32-byte ECDH private key of the sender (the
///   same key that encrypts the xpub).
/// * `compact_xpub`      - The 69-byte DIP-15 compact contact xpub
///   ([`ContactXpubData::compact_xpub`]).
/// * `account_index`     - Account index used in the derivation path
///   (only the low 28 bits are representable).
/// * `version`           - Rotation version (0..=15), placed in the top
///   4 bits.
pub fn calculate_account_reference(
    sender_secret_key: &[u8; 32],
    compact_xpub: &[u8],
    account_index: u32,
    version: u32,
) -> u32 {
    let ask28 = account_secret_key_28(sender_secret_key, compact_xpub);
    let shortened_account_bits = account_index & 0x0FFF_FFFF;
    let version_bits = version << 28;
    version_bits | (ask28 ^ shortened_account_bits)
}

/// Recover `(version, account_index)` from a masked `accountReference`.
///
/// Inverse of [`calculate_account_reference`] for the same
/// `(sender_secret_key, compact_xpub)` pair — only the original sender
/// can un-mask (the PRF key is their ECDH private key). Used on
/// re-send to read the previous rotation version so the superseding
/// request bumps it.
pub fn unmask_account_reference(
    account_reference: u32,
    sender_secret_key: &[u8; 32],
    compact_xpub: &[u8],
) -> (u32, u32) {
    let ask28 = account_secret_key_28(sender_secret_key, compact_xpub);
    let version = account_reference >> 28;
    let account_index = (account_reference & 0x0FFF_FFFF) ^ ask28;
    (version, account_index)
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
        assert_eq!(data.compact.parent_fingerprint.len(), 4);
        assert_eq!(data.compact.chain_code.len(), 32);
        assert_eq!(data.compact.public_key.len(), 33);
        // Compressed public key prefix
        assert!(data.compact.public_key[0] == 0x02 || data.compact.public_key[0] == 0x03);
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
        assert!(!data0.compact.public_key.is_empty());
        assert!(!data1.compact.public_key.is_empty());
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
            forward.compact.public_key, reverse.compact.public_key,
            "Swapping sender/recipient should produce different keys"
        );
    }

    /// Deterministic 69-byte compact xpub fixture for the
    /// account-reference tests (the helper only HMACs the bytes, so a
    /// synthetic buffer with the right length is sufficient and keeps
    /// the vectors stable).
    fn test_compact_xpub() -> [u8; 69] {
        let mut buf = [0u8; 69];
        for (i, b) in buf.iter_mut().enumerate() {
            *b = i as u8;
        }
        buf
    }

    #[test]
    fn test_account_reference_version_bits() {
        let secret_key = [1u8; 32];
        let compact = test_compact_xpub();

        // Version 0
        let ref_v0 = calculate_account_reference(&secret_key, &compact, 0, 0);
        assert_eq!(ref_v0 >> 28, 0, "Version 0 → top 4 bits = 0");

        // Version 1
        let ref_v1 = calculate_account_reference(&secret_key, &compact, 0, 1);
        assert_eq!(ref_v1 >> 28, 1, "Version 1 → top 4 bits = 1");

        // Version 15 (maximum)
        let ref_v15 = calculate_account_reference(&secret_key, &compact, 0, 15);
        assert_eq!(ref_v15 >> 28, 15, "Version 15 → top 4 bits = 15");
    }

    #[test]
    fn test_account_reference_deterministic() {
        let secret_key = [0xABu8; 32];
        let compact = test_compact_xpub();

        let ref1 = calculate_account_reference(&secret_key, &compact, 0, 0);
        let ref2 = calculate_account_reference(&secret_key, &compact, 0, 0);

        assert_eq!(
            ref1, ref2,
            "Same inputs should produce same account reference"
        );
    }

    /// Pin the ASK28 extraction convention (research/06 §3): the mask
    /// must come from HMAC digest bytes `[28..32]` big-endian `>> 4` —
    /// the iOS dash-shared-core reading — NOT bytes `[0..4]` (our old
    /// helper) or little-endian (Android). The expectation recomputes
    /// the HMAC with the same primitive but extracts the window
    /// explicitly, so any byte-order regression in the helper flips
    /// this test.
    #[test]
    fn account_reference_ask28_uses_digest_tail_big_endian() {
        let secret_key = [0x42u8; 32];
        let compact = test_compact_xpub();

        let mut engine = HmacEngine::<sha256::Hash>::new(&secret_key);
        engine.input(&compact);
        let digest = Hmac::<sha256::Hash>::from_engine(engine).to_byte_array();
        let expected_ask28 =
            u32::from_be_bytes([digest[28], digest[29], digest[30], digest[31]]) >> 4;

        // account_index = 0 → the low 28 bits ARE the mask.
        let reference = calculate_account_reference(&secret_key, &compact, 0, 0);
        assert_eq!(
            reference & 0x0FFF_FFFF,
            expected_ask28,
            "ASK28 must be digest bytes [28..32] big-endian >> 4 (iOS dash-shared-core)"
        );

        // And the head-of-digest reading (the old bug) must NOT match —
        // guards against an accidental revert.
        let old_ask28 = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) >> 4;
        assert_ne!(
            reference & 0x0FFF_FFFF,
            old_ask28,
            "head-of-digest extraction is the old bug"
        );
    }

    /// Mask → unmask round-trips `(version, account_index)` for the
    /// sender across the representable ranges.
    #[test]
    fn account_reference_round_trips_version_and_account() {
        let secret_key = [0x07u8; 32];
        let compact = test_compact_xpub();

        for version in [0u32, 1, 7, 15] {
            for account in [0u32, 1, 5, 0x0FFF_FFFF] {
                let reference =
                    calculate_account_reference(&secret_key, &compact, account, version);
                let (got_version, got_account) =
                    unmask_account_reference(reference, &secret_key, &compact);
                assert_eq!(got_version, version, "version round-trip");
                assert_eq!(got_account, account, "account round-trip");
            }
        }

        // A different secret can't recover the account (PRF property —
        // sanity, not security proof).
        let reference = calculate_account_reference(&secret_key, &compact, 5, 0);
        let (_, wrong) = unmask_account_reference(reference, &[0x08u8; 32], &compact);
        assert_ne!(wrong, 5, "different PRF key must not unmask the account");
    }

    /// Known-answer test pinning the ASK28 extraction to the iOS
    /// dash-shared-core convention (`be(ASK[28..32]) >> 4`), and documenting
    /// the other deployed conventions' values for the same digest so a future
    /// reader sees exactly how they diverge. `accountReference` is a one-time
    /// pad recipients ignore, so this is sender-private — but the byte order
    /// must stay locked to keep our sent requests bit-identical to iOS.
    #[test]
    fn ask28_extraction_matches_ios_and_diverges_from_others() {
        let ask: [u8; 32] = std::array::from_fn(|i| i as u8); // ask[i] = i

        // Ours == iOS dash-shared-core (reversed digest → be[28..32]>>4).
        assert_eq!(extract_ask28(&ask), 0x01c1_d1e1);

        // The other live conventions, for the record (all differ):
        let android = u32::from_le_bytes([ask[0], ask[1], ask[2], ask[3]]) >> 4;
        let dip_literal = u32::from_be_bytes([ask[0], ask[1], ask[2], ask[3]]) >> 4;
        assert_eq!(android, 0x0030_2010, "kotlin-platform: le(ASK[0..4])>>4");
        assert_eq!(
            dip_literal, 0x0000_1020,
            "dash-evo-tool / DIP literal: be(ASK[0..4])>>4"
        );
        assert_ne!(extract_ask28(&ask), android);
        assert_ne!(extract_ask28(&ask), dip_literal);
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
        // The send path must encrypt the DIP-15 compact
        // plaintext (fingerprint ‖ chaincode ‖ pubkey = 69 bytes), NOT
        // `ExtendedPubKey::encode()`, which for the DashPay receiving path is
        // the 107-byte DIP-14 serialization (ends in a Normal256 child) and
        // encrypts to 128 bytes — failing the contract's maxItems: 96.
        //
        // The earlier `account_xpub.encode()` producer emitted the 107-byte
        // form (107 != 69); this assertion pins the byte-exact compact layout
        // so a revert to `encode()` is caught.
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
        assert_eq!(&compact[0..4], &data.compact.parent_fingerprint);
        assert_eq!(&compact[4..36], &data.compact.chain_code);
        assert_eq!(&compact[36..69], &data.compact.public_key);

        // And it round-trips through the codec.
        let parsed = platform_encryption::parse_compact_xpub(&compact).expect("parse compact");
        assert_eq!(parsed.parent_fingerprint, data.compact.parent_fingerprint);
        assert_eq!(parsed.chain_code, data.compact.chain_code);
        assert_eq!(parsed.public_key, data.compact.public_key);
    }

    #[test]
    fn reconstructed_xpub_derives_identical_addresses() {
        // Receive-side correctness. After compacting a contact xpub to 69
        // bytes and reconstructing an ExtendedPubKey from
        // (chain_code, public_key) with synthesized depth/child-number, address
        // derivation MUST produce the same addresses as the original xpub —
        // because non-hardened ckd_pub uses only chain_code + public_key.
        let wallet = test_wallet(Network::Testnet);
        let (sender, recipient) = test_identifiers();

        let data = derive_contact_xpub(&wallet, Network::Testnet, 0, &sender, &recipient)
            .expect("derive contact xpub");

        let compact = data.compact_xpub();
        let parsed = platform_encryption::parse_compact_xpub(&compact).expect("parse compact");
        let reconstructed =
            reconstruct_contact_xpub(parsed, Network::Testnet).expect("reconstruct from compact");

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
