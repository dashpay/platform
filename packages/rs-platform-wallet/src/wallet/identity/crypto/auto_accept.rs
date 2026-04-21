//! Auto-accept proof generation and verification for DashPay QR-based contact
//! requests (DIP-15).
//!
//! An auto-accept proof allows the recipient of a QR code to automatically
//! send and accept a contact request without requiring manual approval from the
//! QR creator.
//!
//! # Proof format
//!
//! ```text
//! key_type  (1 byte)  — 0x00 for ECDSA_SECP256K1
//! timestamp (4 bytes) — big-endian u32, derivation index / expiry
//! sig_size  (1 byte)  — 0x40 (64 bytes for compact ECDSA)
//! signature (64 bytes) — compact ECDSA signature
//! ```
//!
//! # Signed message
//!
//! ```text
//! SHA256(sender_id(32B) || recipient_id(32B) || account_ref(4B LE))
//! ```
//!
//! # Derivation path
//!
//! `m/9'/coin'/16'/timestamp'` (all segments hardened)

use dashcore::hashes::{sha256, Hash, HashEngine};
use dashcore::secp256k1::{ecdsa::Signature, Message, Secp256k1, SecretKey};
use dpp::prelude::Identifier;
use key_wallet::bip32::{ChildNumber, DerivationPath};
use key_wallet::wallet::Wallet;
use key_wallet::Network;

use crate::error::PlatformWalletError;

/// DashPay auto-accept feature index per DIP-15.
const DASHPAY_AUTO_ACCEPT_FEATURE: u32 = 16;

// TODO: Where and how we use these helpers?

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build the SHA-256 message that is signed / verified.
///
/// `SHA256(sender_id(32B) || recipient_id(32B) || account_reference(4B LE))`
fn build_message_hash(
    sender_id: &Identifier,
    recipient_id: &Identifier,
    account_reference: u32,
) -> [u8; 32] {
    let mut engine = sha256::Hash::engine();
    engine.input(&sender_id.to_buffer());
    engine.input(&recipient_id.to_buffer());
    engine.input(&account_reference.to_le_bytes());
    sha256::Hash::from_engine(engine).to_byte_array()
}

/// Derive the auto-accept private key at `m/9'/coin'/16'/timestamp'`.
pub fn derive_auto_accept_private_key(
    wallet: &Wallet,
    network: Network,
    timestamp: u32,
) -> Result<SecretKey, PlatformWalletError> {
    let coin_type: u32 = match network {
        Network::Mainnet => 5,
        _ => 1,
    };

    let path = DerivationPath::from(vec![
        ChildNumber::from_hardened_idx(9).expect("valid"),
        ChildNumber::from_hardened_idx(coin_type).expect("valid"),
        ChildNumber::from_hardened_idx(DASHPAY_AUTO_ACCEPT_FEATURE).expect("valid"),
        ChildNumber::from_hardened_idx(timestamp).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid timestamp index: {}", e))
        })?,
    ]);

    let ext_priv = wallet.derive_extended_private_key(&path).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!("Failed to derive auto-accept key: {}", e))
    })?;

    let secret_bytes = zeroize::Zeroizing::new(ext_priv.private_key.secret_bytes());

    SecretKey::from_slice(&*secret_bytes).map_err(|e| {
        PlatformWalletError::InvalidIdentityData(format!(
            "Invalid derived auto-accept private key: {}",
            e
        ))
    })
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Generate an auto-accept proof.
///
/// Derives the ephemeral key at `m/9'/coin'/16'/timestamp'`, then signs
/// `SHA256(sender_id || recipient_id || account_reference)` using compact
/// ECDSA.
///
/// # Arguments
///
/// * `wallet`            - The HD wallet containing the master key.
/// * `network`           - Network for coin-type selection.
/// * `sender_id`         - The identity creating the QR (proof creator).
/// * `recipient_id`      - The identity that will consume the QR.
/// * `account_reference` - Account reference to bind in the proof.
/// * `timestamp`         - Derivation index (typically an expiry timestamp).
///
/// # Returns
///
/// A 70-byte proof: `key_type(1) + timestamp(4 BE) + sig_size(1) + signature(64)`.
pub fn generate_auto_accept_proof(
    wallet: &Wallet,
    network: Network,
    sender_id: &Identifier,
    recipient_id: &Identifier,
    account_reference: u32,
    timestamp: u32,
) -> Result<Vec<u8>, PlatformWalletError> {
    let secret_key = derive_auto_accept_private_key(wallet, network, timestamp)?;

    let msg_hash = build_message_hash(sender_id, recipient_id, account_reference);
    let message = Message::from_digest(msg_hash);

    let secp = Secp256k1::new();
    let signature = secp.sign_ecdsa(&message, &secret_key);
    let sig_bytes = signature.serialize_compact();

    // Build proof bytes.
    let mut proof = Vec::with_capacity(70);
    proof.push(0x00); // key_type: ECDSA_SECP256K1
    proof.extend_from_slice(&timestamp.to_be_bytes()); // 4 bytes BE
    proof.push(0x40); // sig_size: 64
    proof.extend_from_slice(&sig_bytes); // 64 bytes compact ECDSA

    Ok(proof)
}

/// Verify an auto-accept proof.
///
/// Parses the proof bytes, reconstructs the expected public key by deriving
/// from the wallet at `m/9'/coin'/16'/timestamp'`, and checks the ECDSA
/// signature.
///
/// # Note
///
/// This verification requires access to the same wallet that generated the
/// proof, because the public key is derived from the wallet seed. If you only
/// have the proof and a standalone public key, use
/// [`verify_auto_accept_proof_with_pubkey`] instead (if available).
///
/// For a standalone (no-wallet) verification, the caller must derive or know
/// the public key externally. This function performs the full derivation.
pub fn verify_auto_accept_proof(
    wallet: &Wallet,
    network: Network,
    proof_bytes: &[u8],
    sender_id: &Identifier,
    recipient_id: &Identifier,
    account_reference: u32,
) -> Result<bool, PlatformWalletError> {
    // Parse proof header.
    if proof_bytes.len() < 6 {
        return Ok(false);
    }

    let _key_type = proof_bytes[0];
    let timestamp = u32::from_be_bytes([
        proof_bytes[1],
        proof_bytes[2],
        proof_bytes[3],
        proof_bytes[4],
    ]);
    let sig_len = proof_bytes[5] as usize;

    if sig_len != 64 || proof_bytes.len() < 6 + sig_len {
        return Ok(false);
    }

    let signature_bytes = &proof_bytes[6..6 + sig_len];

    // Derive the expected public key from the wallet.
    let secret_key = derive_auto_accept_private_key(wallet, network, timestamp)?;
    let secp = Secp256k1::new();
    let pubkey = dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

    // Reconstruct the message.
    let msg_hash = build_message_hash(sender_id, recipient_id, account_reference);
    let message = Message::from_digest(msg_hash);

    // Parse the signature.
    let signature = match Signature::from_compact(signature_bytes) {
        Ok(s) => s,
        Err(_) => return Ok(false),
    };

    // Verify.
    match secp.verify_ecdsa(&message, &signature, &pubkey) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;

    fn test_wallet() -> Wallet {
        let seed = [0x42u8; 64];
        Wallet::from_seed_bytes(seed, Network::Testnet, WalletAccountCreationOptions::None)
            .expect("Failed to create test wallet")
    }

    fn test_ids() -> (Identifier, Identifier) {
        (
            Identifier::from([0x11u8; 32]),
            Identifier::from([0x22u8; 32]),
        )
    }

    #[test]
    fn test_generate_proof_format() {
        let wallet = test_wallet();
        let (sender, recipient) = test_ids();

        let proof = generate_auto_accept_proof(
            &wallet,
            Network::Testnet,
            &sender,
            &recipient,
            0,
            1700000000,
        )
        .expect("should generate proof");

        // Total: 1 + 4 + 1 + 64 = 70 bytes
        assert_eq!(proof.len(), 70);
        assert_eq!(proof[0], 0x00); // key_type
        assert_eq!(proof[5], 0x40); // sig_size = 64
    }

    #[test]
    fn test_roundtrip_verify() {
        let wallet = test_wallet();
        let (sender, recipient) = test_ids();
        let timestamp = 1700000000u32;
        let account_ref = 42u32;

        let proof = generate_auto_accept_proof(
            &wallet,
            Network::Testnet,
            &sender,
            &recipient,
            account_ref,
            timestamp,
        )
        .expect("generate");

        let valid = verify_auto_accept_proof(
            &wallet,
            Network::Testnet,
            &proof,
            &sender,
            &recipient,
            account_ref,
        )
        .expect("verify");

        assert!(valid, "proof should verify with correct params");
    }

    #[test]
    fn test_wrong_account_reference_fails() {
        let wallet = test_wallet();
        let (sender, recipient) = test_ids();

        let proof = generate_auto_accept_proof(
            &wallet,
            Network::Testnet,
            &sender,
            &recipient,
            0,
            1700000000,
        )
        .expect("generate");

        let valid = verify_auto_accept_proof(
            &wallet,
            Network::Testnet,
            &proof,
            &sender,
            &recipient,
            999, // wrong account reference
        )
        .expect("verify");

        assert!(!valid, "proof should not verify with wrong account ref");
    }

    #[test]
    fn test_wrong_ids_fail() {
        let wallet = test_wallet();
        let (sender, recipient) = test_ids();
        let other = Identifier::from([0x33u8; 32]);

        let proof = generate_auto_accept_proof(
            &wallet,
            Network::Testnet,
            &sender,
            &recipient,
            0,
            1700000000,
        )
        .expect("generate");

        // Wrong sender
        let valid =
            verify_auto_accept_proof(&wallet, Network::Testnet, &proof, &other, &recipient, 0)
                .expect("verify");
        assert!(!valid);

        // Wrong recipient
        let valid = verify_auto_accept_proof(&wallet, Network::Testnet, &proof, &sender, &other, 0)
            .expect("verify");
        assert!(!valid);
    }

    #[test]
    fn test_truncated_proof_returns_false() {
        let wallet = test_wallet();
        let (sender, recipient) = test_ids();

        let valid =
            verify_auto_accept_proof(&wallet, Network::Testnet, &[0u8; 3], &sender, &recipient, 0)
                .expect("verify");
        assert!(!valid);
    }

    #[test]
    fn test_different_timestamps_produce_different_proofs() {
        let wallet = test_wallet();
        let (sender, recipient) = test_ids();

        let proof1 = generate_auto_accept_proof(
            &wallet,
            Network::Testnet,
            &sender,
            &recipient,
            0,
            1700000000,
        )
        .expect("generate 1");

        let proof2 = generate_auto_accept_proof(
            &wallet,
            Network::Testnet,
            &sender,
            &recipient,
            0,
            1700000001,
        )
        .expect("generate 2");

        assert_ne!(proof1, proof2);
    }
}
