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
use key_wallet::dip9::{
    DASH_COIN_TYPE, DASH_TESTNET_COIN_TYPE, FEATURE_PURPOSE, FEATURE_PURPOSE_DASHPAY_AUTO_ACCEPT,
};
use key_wallet::wallet::Wallet;
use key_wallet::Network;

use crate::error::PlatformWalletError;

/// Default lifetime of a generated auto-accept QR, in seconds (1 hour).
///
/// DIP-15 mandates only that the proof's 4-byte timestamp *is* an expiry (and the
/// hardened derivation index); it does not prescribe a value. We pick a short
/// default because the QR carries a usable (bearer) private key and auto-accept
/// is always-on (no off-switch) — see the security notes in
/// `docs/dashpay/QR_AUTO_ACCEPT_SPEC.md` §6.
pub const AUTO_ACCEPT_TTL_SECS: u32 = 3600;

/// `key type` byte for an ECDSA_SECP256K1 auto-accept key/proof (DIP-15).
const KEY_TYPE_ECDSA: u8 = 0x00;
/// `key size` byte for a 32-byte secp256k1 private key in the `dapk` blob.
const ECDSA_KEY_SIZE: u8 = 0x20;
/// `signature size` byte for a 64-byte compact ECDSA signature in the proof.
const ECDSA_SIG_SIZE: u8 = 0x40;
/// Length of the ECDSA `dapk` key blob: `type(1)+timestamp(4)+size(1)+key(32)`.
const KEY_BLOB_LEN: usize = 38;

// ---------------------------------------------------------------------------
// Helpers
//
// Role mapping (DIP-15): throughout this module `sender_id` is the contact
// request's `$ownerId` — i.e. the **scanner** who sends the request — and
// `recipient_id` is `toUserId`, the **QR owner** who auto-accepts. The scanner
// signs with the owner's handed-out key; the owner verifies against its own
// re-derived key. Inverting these silently breaks verification.
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

/// The DIP-15 auto-accept derivation path `m/9'/coin'/16'/expiry'` (all
/// hardened). The owner derives the key here (for the QR / verify); `expiry`
/// is the hardened leaf, so it must be ≤ 2^31−1 (rejected otherwise).
pub fn auto_accept_derivation_path(
    network: Network,
    expiry: u32,
) -> Result<DerivationPath, PlatformWalletError> {
    let coin_type: u32 = match network {
        Network::Mainnet => DASH_COIN_TYPE,
        _ => DASH_TESTNET_COIN_TYPE,
    };
    Ok(DerivationPath::from(vec![
        ChildNumber::from_hardened_idx(FEATURE_PURPOSE).expect("valid"),
        ChildNumber::from_hardened_idx(coin_type).expect("valid"),
        ChildNumber::from_hardened_idx(FEATURE_PURPOSE_DASHPAY_AUTO_ACCEPT).expect("valid"),
        ChildNumber::from_hardened_idx(expiry).map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!("Invalid expiry index: {}", e))
        })?,
    ]))
}

/// Derive the auto-accept private key at `m/9'/coin'/16'/timestamp'`.
pub fn derive_auto_accept_private_key(
    wallet: &Wallet,
    network: Network,
    timestamp: u32,
) -> Result<SecretKey, PlatformWalletError> {
    let path = auto_accept_derivation_path(network, timestamp)?;

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

/// Sign an auto-accept proof with the `secret_key` handed out in the QR.
///
/// This is the **scanner's** side: the scanner decodes the owner's auto-accept
/// private key from the `dapk` blob and signs
/// `SHA256(sender_id || recipient_id || account_reference)` (compact ECDSA),
/// binding the proof to *this* sender so a leaked key can't be replayed by a
/// different sender. `timestamp` is the expiry from the key blob, written into
/// the proof header so the owner can re-derive the key to verify.
///
/// # Returns
/// A 70-byte proof: `key_type(1) + timestamp(4 BE) + sig_size(1) + signature(64)`.
pub fn sign_auto_accept_proof(
    secret_key: &SecretKey,
    sender_id: &Identifier,
    recipient_id: &Identifier,
    account_reference: u32,
    timestamp: u32,
) -> Vec<u8> {
    let msg_hash = build_message_hash(sender_id, recipient_id, account_reference);
    let message = Message::from_digest(msg_hash);

    let secp = Secp256k1::new();
    let signature = secp.sign_ecdsa(&message, secret_key);
    let sig_bytes = signature.serialize_compact();

    let mut proof = Vec::with_capacity(1 + 4 + 1 + 64);
    proof.push(KEY_TYPE_ECDSA);
    proof.extend_from_slice(&timestamp.to_be_bytes()); // 4 bytes BE
    proof.push(ECDSA_SIG_SIZE);
    proof.extend_from_slice(&sig_bytes); // 64 bytes compact ECDSA
    proof
}

/// Generate an auto-accept proof by deriving the owner's key from `wallet` and
/// signing — i.e. `derive_auto_accept_private_key` + [`sign_auto_accept_proof`].
///
/// In the real QR flow the owner does *not* call this (it doesn't know the
/// scanner's id at QR-create time); it derives the key for the `dapk` blob and
/// the scanner signs. This helper is kept for owner-side tests / a self-check.
///
/// # Returns
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
    Ok(sign_auto_accept_proof(
        &secret_key,
        sender_id,
        recipient_id,
        account_reference,
        timestamp,
    ))
}

/// The expiry timestamp embedded in a proof header (the `key index` field), or
/// `None` if the proof is too short. This is the *same* value that selects the
/// derivation index used to verify, so a forged future expiry derives a
/// different key and fails the signature check — the expiry can't be lied about
/// independently of the signature.
pub fn auto_accept_proof_expiry(proof_bytes: &[u8]) -> Option<u32> {
    if proof_bytes.len() < 6 {
        return None;
    }
    Some(u32::from_be_bytes([
        proof_bytes[1],
        proof_bytes[2],
        proof_bytes[3],
        proof_bytes[4],
    ]))
}

/// Verify an auto-accept proof against a known auto-accept **public** key.
///
/// This is the seedless verify path: the owner (recipient) re-derives its own
/// auto-accept public key at `m/9'/coin'/16'/expiry'` — through the Keychain
/// resolver, never a resident seed — and passes it here. Pure: parses the proof,
/// reconstructs `SHA256(sender_id || recipient_id || account_reference)`, and
/// checks the compact ECDSA signature. Returns `false` (never errors) on any
/// malformed input.
///
/// Does **not** check expiry — callers acting on the proof MUST also compare
/// [`auto_accept_proof_expiry`] against the current time. (Keeping the crypto
/// check clock-free makes it deterministically testable; the acceptance path
/// pairs the two.)
pub fn verify_auto_accept_proof_with_pubkey(
    pubkey: &dashcore::secp256k1::PublicKey,
    proof_bytes: &[u8],
    sender_id: &Identifier,
    recipient_id: &Identifier,
    account_reference: u32,
) -> bool {
    if proof_bytes.len() < 6 {
        return false;
    }
    let sig_len = proof_bytes[5] as usize;
    if sig_len != 64 || proof_bytes.len() < 6 + sig_len {
        return false;
    }
    let signature_bytes = &proof_bytes[6..6 + sig_len];

    let msg_hash = build_message_hash(sender_id, recipient_id, account_reference);
    let message = Message::from_digest(msg_hash);

    let signature = match Signature::from_compact(signature_bytes) {
        Ok(s) => s,
        Err(_) => return false,
    };

    Secp256k1::new()
        .verify_ecdsa(&message, &signature, pubkey)
        .is_ok()
}

/// Verify an auto-accept proof by re-deriving the expected key from `wallet`.
///
/// Owner-side convenience that derives the auto-accept key at
/// `m/9'/coin'/16'/expiry'` from a resident `Wallet` and delegates to
/// [`verify_auto_accept_proof_with_pubkey`]. **Seedless wallets cannot use this**
/// (there is no resident `Wallet`); the drain derives the public key via the
/// `ContactCryptoProvider` and calls the pubkey variant directly. Kept for
/// owner-side tests. Does not check expiry.
pub fn verify_auto_accept_proof(
    wallet: &Wallet,
    network: Network,
    proof_bytes: &[u8],
    sender_id: &Identifier,
    recipient_id: &Identifier,
    account_reference: u32,
) -> Result<bool, PlatformWalletError> {
    let Some(timestamp) = auto_accept_proof_expiry(proof_bytes) else {
        return Ok(false);
    };
    let secret_key = derive_auto_accept_private_key(wallet, network, timestamp)?;
    let pubkey = dashcore::secp256k1::PublicKey::from_secret_key(&Secp256k1::new(), &secret_key);
    Ok(verify_auto_accept_proof_with_pubkey(
        &pubkey,
        proof_bytes,
        sender_id,
        recipient_id,
        account_reference,
    ))
}

// ---------------------------------------------------------------------------
// QR `dapk` key blob + `dash:?du=…&dapk=…` URI codecs
// ---------------------------------------------------------------------------

fn invalid(msg: impl Into<String>) -> PlatformWalletError {
    PlatformWalletError::InvalidIdentityData(msg.into())
}

/// Encode the DIP-15 `dapk` key blob:
/// `key_type(1) | expiry(4 BE) | key_size(1) | key(32)` (38 bytes for ECDSA).
///
/// The blob carries the auto-accept **private** key — a deliberate, expiry-
/// bounded bearer credential the owner shares in the QR so any scanner can
/// produce a per-sender-bound proof (DIP-15). Scoped to auto-accept only.
pub fn encode_auto_accept_key_blob(secret_key: &SecretKey, expiry: u32) -> Vec<u8> {
    let mut blob = Vec::with_capacity(KEY_BLOB_LEN);
    blob.push(KEY_TYPE_ECDSA);
    blob.extend_from_slice(&expiry.to_be_bytes());
    blob.push(ECDSA_KEY_SIZE);
    blob.extend_from_slice(&secret_key.secret_bytes());
    blob
}

/// Decode a DIP-15 ECDSA `dapk` key blob into `(private key, expiry)`.
///
/// # Errors
/// Rejects a blob that is not exactly 38 bytes, has a non-ECDSA key type,
/// a key size other than 32, or an invalid scalar.
pub fn decode_auto_accept_key_blob(blob: &[u8]) -> Result<(SecretKey, u32), PlatformWalletError> {
    if blob.len() != KEY_BLOB_LEN {
        return Err(invalid(format!(
            "auto-accept key blob must be {KEY_BLOB_LEN} bytes, got {}",
            blob.len()
        )));
    }
    if blob[0] != KEY_TYPE_ECDSA {
        return Err(invalid("unsupported auto-accept key type (expected ECDSA)"));
    }
    let expiry = u32::from_be_bytes([blob[1], blob[2], blob[3], blob[4]]);
    if blob[5] != ECDSA_KEY_SIZE {
        return Err(invalid("auto-accept key size must be 32"));
    }
    let secret_key = SecretKey::from_slice(&blob[6..KEY_BLOB_LEN])
        .map_err(|e| invalid(format!("invalid auto-accept private key: {e}")))?;
    Ok((secret_key, expiry))
}

/// Build a DIP-15 contact URI: `dash:?du=<username>&dapk=<base58(key_blob)>`.
pub fn encode_dashpay_contact_uri(username: &str, key_blob: &[u8]) -> String {
    format!(
        "dash:?du={}&dapk={}",
        username,
        bs58::encode(key_blob).into_string()
    )
}

/// Parse a DIP-15 contact URI into `(username, key_blob)`.
///
/// Accepts the contact-only form `dash:?du=…&dapk=…` (and tolerates a leading
/// address before the `?`, per the merchant variant — ignored here). Both
/// `du` and `dapk` are required; `dapk` is base58.
///
/// # Errors
/// Rejects a non-`dash:` scheme or a URI missing either parameter / with a
/// non-base58 `dapk`.
pub fn parse_dashpay_contact_uri(uri: &str) -> Result<(String, Vec<u8>), PlatformWalletError> {
    let rest = uri
        .strip_prefix("dash:")
        .ok_or_else(|| invalid("not a dash: URI"))?;
    // Query is everything after the first '?'; an address may precede it.
    let query = rest.split_once('?').map(|(_, q)| q).unwrap_or(rest);

    let mut username: Option<String> = None;
    let mut dapk: Option<String> = None;
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            match k {
                "du" => username = Some(v.to_string()),
                "dapk" => dapk = Some(v.to_string()),
                _ => {}
            }
        }
    }

    let username = username.ok_or_else(|| invalid("contact URI missing du (username)"))?;
    let dapk = dapk.ok_or_else(|| invalid("contact URI missing dapk (key)"))?;
    // Bound the base58 work an unauthenticated QR / deep link can force. A
    // valid `KEY_BLOB_LEN`-byte DIP-15 blob base58-encodes to ~52 chars, and
    // `decode_auto_accept_key_blob` rejects wrong-length blobs anyway — but
    // only after `bs58::decode` allocates ~0.73 × len bytes. Cap the input so
    // a hostile multi-megabyte `dapk` value can't force a large allocation
    // per scan/paste before any structural validation runs.
    const MAX_DAPK_BASE58_LEN: usize = 128;
    if dapk.len() > MAX_DAPK_BASE58_LEN {
        return Err(invalid(format!(
            "dapk too long ({} chars; max {MAX_DAPK_BASE58_LEN})",
            dapk.len()
        )));
    }
    let key_blob = bs58::decode(&dapk)
        .into_vec()
        .map_err(|e| invalid(format!("dapk is not valid base58: {e}")))?;
    Ok((username, key_blob))
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

    /// The real cross-actor flow: the owner derives the key + shares it in the
    /// blob; the **scanner** decodes the handed key and signs over its own id;
    /// the owner verifies against its **own re-derived public key** (the seedless
    /// drain path, which gets the pubkey via `provider.receiving_xpub`). Pins
    /// that the scanner-signs / owner-verifies-with-pubkey split round-trips and
    /// that the per-sender binding holds.
    #[test]
    fn cross_actor_sign_then_verify_with_pubkey() {
        let wallet = test_wallet(); // the owner's wallet
        let (scanner, owner) = test_ids();
        let expiry = 1_700_000_000u32;
        let account_ref = 7u32;

        let owner_key =
            derive_auto_accept_private_key(&wallet, Network::Testnet, expiry).expect("derive");
        let blob = encode_auto_accept_key_blob(&owner_key, expiry);
        let (handed_key, decoded_expiry) = decode_auto_accept_key_blob(&blob).expect("decode blob");
        assert_eq!(decoded_expiry, expiry);

        let proof = sign_auto_accept_proof(&handed_key, &scanner, &owner, account_ref, expiry);
        assert_eq!(proof.len(), 70);
        assert_eq!(auto_accept_proof_expiry(&proof), Some(expiry));

        let pubkey = dashcore::secp256k1::PublicKey::from_secret_key(&Secp256k1::new(), &owner_key);
        assert!(
            verify_auto_accept_proof_with_pubkey(&pubkey, &proof, &scanner, &owner, account_ref),
            "owner verifies the scanner's proof against its own re-derived pubkey"
        );

        // Per-sender / per-account binding: a different sender or account fails.
        let other = Identifier::from([0x33u8; 32]);
        assert!(!verify_auto_accept_proof_with_pubkey(
            &pubkey,
            &proof,
            &other,
            &owner,
            account_ref
        ));
        assert!(!verify_auto_accept_proof_with_pubkey(
            &pubkey, &proof, &scanner, &owner, 999
        ));
    }

    #[test]
    fn key_blob_round_trip_and_rejects_malformed() {
        let key = SecretKey::from_slice(&[0x07u8; 32]).unwrap();
        let blob = encode_auto_accept_key_blob(&key, 12345);
        assert_eq!(blob.len(), 38);
        assert_eq!(blob[0], 0x00); // key type
        assert_eq!(blob[5], 0x20); // key size

        let (k2, e2) = decode_auto_accept_key_blob(&blob).expect("decode");
        assert_eq!(k2.secret_bytes(), key.secret_bytes());
        assert_eq!(e2, 12345);

        assert!(decode_auto_accept_key_blob(&blob[..37]).is_err(), "short");
        let mut bad_type = blob.clone();
        bad_type[0] = 0x01;
        assert!(decode_auto_accept_key_blob(&bad_type).is_err(), "key type");
        let mut bad_size = blob.clone();
        bad_size[5] = 0x10;
        assert!(decode_auto_accept_key_blob(&bad_size).is_err(), "key size");
    }

    #[test]
    fn uri_round_trip_and_rejects_malformed() {
        let key = SecretKey::from_slice(&[0x09u8; 32]).unwrap();
        let blob = encode_auto_accept_key_blob(&key, 999);
        let uri = encode_dashpay_contact_uri("bobspizza", &blob);
        assert!(uri.starts_with("dash:?du=bobspizza&dapk="));

        let (u, b) = parse_dashpay_contact_uri(&uri).expect("parse");
        assert_eq!(u, "bobspizza");
        assert_eq!(b, blob);

        // Merchant variant: a leading address + extra params, du/dapk still parse.
        let merchant = format!(
            "dash:Xabc123?amount=0.1&du=bobspizza&dapk={}",
            bs58::encode(&blob).into_string()
        );
        let (u2, b2) = parse_dashpay_contact_uri(&merchant).expect("parse merchant");
        assert_eq!(u2, "bobspizza");
        assert_eq!(b2, blob);

        assert!(
            parse_dashpay_contact_uri("http:?du=x&dapk=y").is_err(),
            "scheme"
        );
        assert!(
            parse_dashpay_contact_uri("dash:?dapk=abc").is_err(),
            "no du"
        );
        assert!(parse_dashpay_contact_uri("dash:?du=x").is_err(), "no dapk");
        // '0','O','I','l' are not in the base58 alphabet → decode fails.
        assert!(
            parse_dashpay_contact_uri("dash:?du=x&dapk=0OIl").is_err(),
            "bad b58"
        );
    }

    #[test]
    fn parse_contact_uri_caps_oversized_dapk_before_decoding() {
        // A hostile QR / deep link with a huge base58 `dapk` must be rejected
        // up front rather than base58-decoded into a large allocation. A valid
        // blob encodes to ~52 chars; this is far over the 128-char cap.
        let huge = "z".repeat(5000);
        let uri = format!("dash:?du=alice&dapk={huge}");
        let err = parse_dashpay_contact_uri(&uri).expect_err("oversized dapk must be rejected");
        assert!(
            err.to_string().contains("too long"),
            "expected a length-cap rejection, got: {err}"
        );

        // The cap must not reject a normal-length (valid) dapk.
        let key = SecretKey::from_slice(&[0x09u8; 32]).unwrap();
        let blob = encode_auto_accept_key_blob(&key, 1);
        let uri = encode_dashpay_contact_uri("alice", &blob);
        assert!(
            parse_dashpay_contact_uri(&uri).is_ok(),
            "a valid-length dapk must still parse"
        );
    }

    #[test]
    fn verify_with_pubkey_rejects_truncated_and_no_expiry() {
        let key = SecretKey::from_slice(&[0x05u8; 32]).unwrap();
        let pubkey = dashcore::secp256k1::PublicKey::from_secret_key(&Secp256k1::new(), &key);
        let (s, r) = test_ids();
        assert!(!verify_auto_accept_proof_with_pubkey(
            &pubkey, &[0u8; 3], &s, &r, 0
        ));
        assert_eq!(auto_accept_proof_expiry(&[0u8; 3]), None);
    }
}
