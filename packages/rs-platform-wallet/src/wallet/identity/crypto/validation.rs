//! Pre-send validation for DashPay contact requests.
//!
//! Validates that the sender and recipient identities have the correct key
//! types and purposes before a contact request is submitted to the platform.

use dash_sdk::platform::dashpay::recipient_key_purpose_is_valid;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, KeyType, Purpose};

/// Result of validating a contact request before it is sent.
#[derive(Debug, Clone)]
pub struct ContactRequestValidation {
    /// Whether the contact request is valid and safe to send.
    pub is_valid: bool,
    /// Hard errors that prevent the request from being sent.
    pub errors: Vec<String>,
    /// Non-fatal warnings the caller may want to surface.
    pub warnings: Vec<String>,
    /// `true` when a key-PURPOSE mismatch was seen (e.g. a legacy 2024 doc
    /// referencing an AUTHENTICATION key).
    ///
    /// This classification is load-bearing for the sync sweep / accept paths:
    /// a purpose mismatch must NOT mark the payment channel
    /// **permanently** broken — on-chain history demonstrably contains
    /// nonconforming-but-honest documents, and our acceptance policy (not the
    /// immutable request) is what might change. A purpose-only failure is a
    /// non-permanent skip (log + retry next sweep); a key-TYPE / missing-key /
    /// disabled-key failure stays permanent.
    ///
    /// **Read [`is_purpose_only`](Self::is_purpose_only), not this field, to
    /// decide skip-vs-break.** This flag alone is `true` even when a hard
    /// (non-purpose) error is *also* present; downgrading to a skip in that
    /// case would mask a genuinely permanent failure (a disabled / wrong-type
    /// key) into a retry-forever loop.
    pub purpose_mismatch: bool,
    /// `true` when at least one *non-purpose* hard error was recorded (missing
    /// key, wrong key type, disabled key). Distinguishes "purpose mismatch is
    /// the sole cause" (downgradable to a skip) from "purpose mismatch plus a
    /// genuinely permanent fault" (must stay permanent). See
    /// [`is_purpose_only`](Self::is_purpose_only).
    pub hard_error: bool,
}

impl Default for ContactRequestValidation {
    fn default() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            purpose_mismatch: false,
            hard_error: false,
        }
    }
}

impl ContactRequestValidation {
    /// Create a new, initially-valid validation result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a hard (non-purpose) error: sets `is_valid = false` AND flags
    /// `hard_error` so a co-occurring purpose mismatch can't downgrade this
    /// genuinely-permanent fault to a skip.
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
        self.hard_error = true;
    }

    /// Add a key-PURPOSE error: sets `is_valid = false` AND flags
    /// `purpose_mismatch` so callers can downgrade a *purpose-only* failure
    /// to a non-permanent skip rather than a permanent broken-channel mark.
    /// Does NOT set `hard_error`.
    pub fn add_purpose_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
        self.purpose_mismatch = true;
    }

    /// Add a non-fatal warning.
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Whether the *sole* cause of invalidity is a key-purpose mismatch —
    /// the only case that may be downgraded to a non-permanent skip.
    /// A purpose mismatch that co-occurs with a hard error (disabled /
    /// missing / wrong-type key) is NOT purpose-only and must stay permanent.
    pub fn is_purpose_only(&self) -> bool {
        self.purpose_mismatch && !self.hard_error
    }

    /// Merge another validation result into this one.
    pub fn merge(&mut self, other: ContactRequestValidation) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        if !other.is_valid {
            self.is_valid = false;
        }
        if other.purpose_mismatch {
            self.purpose_mismatch = true;
        }
        if other.hard_error {
            self.hard_error = true;
        }
    }
}

/// Validate a contact request against the verified on-chain envelope.
///
/// The empirical testnet census (368 docs) shows two live
/// honest cohorts: the dominant mobile population references an **unbound
/// ENCRYPTION key for BOTH indices** (mobile identities carry no DECRYPTION
/// key), and the newest cohort uses bound **ENCRYPTION(sender) /
/// DECRYPTION(recipient)** — our original convention. Consensus enforces
/// neither purpose nor boundedness on these integer fields. This validator is
/// therefore *liberal on receive*: it accepts the purposes mobile actually
/// uses while keeping the ECDSA key-*type* gate (every observed key is
/// ECDSA_SECP256K1) and the disabled-key check.
///
/// # Checks performed
///
/// **Sender key:**
/// - Key at `sender_key_index` exists on the sender identity.
/// - Key type is `ECDSA_SECP256K1` (required for ECDH).
/// - Key purpose is `ENCRYPTION` (bound or unbound) — a non-ENCRYPTION
///   purpose is flagged as a `purpose_mismatch` (non-permanent).
/// - Key is not disabled.
///
/// **Recipient key (our key):**
/// - Key at `recipient_key_index` exists on the recipient identity.
/// - Key type is compatible (`ECDSA_SECP256K1` or `ECDSA_HASH160`).
/// - Key purpose is `ENCRYPTION` **or** `DECRYPTION` — anything else
///   (AUTHENTICATION/MASTER/TRANSFER) is flagged as a `purpose_mismatch`.
/// - Key is not disabled.
///
/// A failure whose *only* cause is a purpose mismatch sets
/// [`ContactRequestValidation::purpose_mismatch`], signalling callers to skip
/// (and retry) rather than permanently break the channel.
pub fn validate_contact_request(
    sender_identity: &Identity,
    sender_key_index: u32,
    recipient_identity: &Identity,
    recipient_key_index: u32,
) -> ContactRequestValidation {
    let mut validation = ContactRequestValidation::new();

    // -----------------------------------------------------------------------
    // Sender key validation
    // -----------------------------------------------------------------------
    match sender_identity.get_public_key_by_id(sender_key_index) {
        Some(key) => {
            // Must be ECDSA_SECP256K1 for ECDH.
            if key.key_type() != KeyType::ECDSA_SECP256K1 {
                validation.add_error(format!(
                    "Sender key {} has type {:?}, but ECDSA_SECP256K1 is required for ECDH",
                    sender_key_index,
                    key.key_type(),
                ));
            }

            // Must have ENCRYPTION purpose (bound or unbound — both live
            // cohorts use ENCRYPTION for the sender). A non-ENCRYPTION
            // purpose is a non-permanent purpose mismatch.
            if key.purpose() != Purpose::ENCRYPTION {
                validation.add_purpose_error(format!(
                    "Sender key {} has purpose {:?}, but ENCRYPTION is required for contact requests",
                    sender_key_index,
                    key.purpose(),
                ));
            }

            // Must not be disabled.
            if let Some(disabled_at) = key.disabled_at() {
                validation.add_error(format!(
                    "Sender key {} is disabled (at timestamp {})",
                    sender_key_index, disabled_at,
                ));
            }
        }
        None => {
            validation.add_error(format!(
                "Sender key index {} not found on identity {}",
                sender_key_index,
                sender_identity.id(),
            ));
        }
    }

    // -----------------------------------------------------------------------
    // Recipient key validation
    // -----------------------------------------------------------------------
    match recipient_identity.get_public_key_by_id(recipient_key_index) {
        Some(key) => {
            // Must be an ECDSA variant for ECDH compatibility.
            match key.key_type() {
                KeyType::ECDSA_SECP256K1 => {
                    // Ideal type for contact requests.
                }
                KeyType::ECDSA_HASH160 => {
                    validation.add_warning(format!(
                        "Recipient key {} is ECDSA_HASH160; full public key is needed for ECDH — \
                         ensure the full key is available",
                        recipient_key_index,
                    ));
                }
                other => {
                    validation.add_error(format!(
                        "Recipient key {} has type {:?}, which is not compatible with ECDH",
                        recipient_key_index, other,
                    ));
                }
            }

            // Purpose must be ENCRYPTION or DECRYPTION: the mobile
            // cohort's recipientKeyIndex points at an ENCRYPTION key, the
            // newest cohort's at a DECRYPTION key — both honest. Anything
            // else (AUTHENTICATION/MASTER/TRANSFER) is a non-permanent purpose
            // mismatch: legacy 2024 docs reference AUTHENTICATION keys, so we
            // skip-and-retry rather than permanently break the channel. The
            // accepted cohort is owned by the shared SDK predicate so this
            // validator and the recipient-key selector cannot disagree.
            if !recipient_key_purpose_is_valid(key.purpose()) {
                validation.add_purpose_error(format!(
                    "Recipient key {} has purpose {:?}, but ENCRYPTION or DECRYPTION is \
                     required for contact requests",
                    recipient_key_index,
                    key.purpose(),
                ));
            }

            // Must not be disabled.
            if let Some(disabled_at) = key.disabled_at() {
                validation.add_error(format!(
                    "Recipient key {} is disabled (at timestamp {})",
                    recipient_key_index, disabled_at,
                ));
            }
        }
        None => {
            validation.add_error(format!(
                "Recipient key index {} not found on identity {}",
                recipient_key_index,
                recipient_identity.id(),
            ));
        }
    }

    validation
}

/// Decide whether a derived compressed secp256k1 public key binds to the
/// caller's known on-chain key data — the sign-time / verify-time
/// public-key-binding policy, shared by every ECDSA key path so it cannot
/// drift from the discovery-time ownership decision.
///
/// `derived_pubkey` is the 33-byte compressed pubkey re-derived at a
/// breadcrumb path (`ExtendedPubKey::from_priv(..).public_key.serialize()`).
/// `expected_key_data` is the on-chain key's `data`, discriminated by length:
///
/// - **33 bytes** → the on-chain key is an `ECDSA_SECP256K1` key whose `data`
///   is the compressed pubkey; binds iff the two byte strings are equal.
/// - **20 bytes** → the on-chain key is an `ECDSA_HASH160` key whose `data` is
///   `ripemd160_sha256` of the compressed pubkey; binds iff that hash equals
///   the expected bytes.
/// - **any other length** → fails closed (`false`), never binds.
///
/// This is byte-for-byte the same decision
/// `IdentityPublicKey::validate_private_key_bytes` makes from the secret
/// scalar: for `ECDSA_SECP256K1` it compares `data` to the compressed pubkey,
/// and for `ECDSA_HASH160` it compares `data` to `ripemd160_sha256` of that
/// same compressed pubkey (`identity_public_key/v0/methods/mod.rs`). Length is
/// the wire discriminator here because the caller (the FFI resolver-signing
/// binding, `sign_with_mnemonic_resolver.rs`) holds raw expected bytes rather
/// than a typed `IdentityPublicKey`; the 33/20 split is exactly the ECDSA
/// arms' two representations, so the policies stay aligned. The
/// `pubkey_reproduces` / `validate_private_key_bytes` equivalence is pinned in
/// `discovery.rs::pubkey_verify_matches_scalar_verify_for_every_key`.
pub fn pubkey_binds_expected_key_data(derived_pubkey: &[u8; 33], expected_key_data: &[u8]) -> bool {
    use dpp::util::hash::ripemd160_sha256;
    match expected_key_data.len() {
        33 => derived_pubkey.as_slice() == expected_key_data,
        20 => ripemd160_sha256(derived_pubkey.as_slice()).as_slice() == expected_key_data,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityPublicKey, IdentityV0, KeyType, Purpose, SecurityLevel};
    use dpp::prelude::Identifier;
    use std::collections::BTreeMap;

    fn make_key(id: u32, key_type: KeyType, purpose: Purpose) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            key_type,
            purpose,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: None,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(vec![0x02; 33]),
            disabled_at: None,
        })
    }

    fn make_identity(keys: Vec<IdentityPublicKey>) -> Identity {
        let mut key_map = BTreeMap::new();
        for k in keys {
            key_map.insert(k.id(), k);
        }
        Identity::V0(IdentityV0 {
            id: Identifier::from([0xAA; 32]),
            public_keys: key_map,
            balance: 0,
            revision: 0,
        })
    }

    #[test]
    fn test_valid_request() {
        let sender = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::ENCRYPTION,
        )]);
        let recipient = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::DECRYPTION,
        )]);

        let result = validate_contact_request(&sender, 0, &recipient, 0);
        assert!(result.is_valid, "errors: {:?}", result.errors);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_sender_key_missing() {
        let sender = make_identity(vec![]);
        let recipient = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::DECRYPTION,
        )]);

        let result = validate_contact_request(&sender, 0, &recipient, 0);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("not found")));
    }

    #[test]
    fn test_sender_wrong_key_type() {
        let sender = make_identity(vec![make_key(0, KeyType::BLS12_381, Purpose::ENCRYPTION)]);
        let recipient = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::DECRYPTION,
        )]);

        let result = validate_contact_request(&sender, 0, &recipient, 0);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("ECDSA_SECP256K1")));
    }

    #[test]
    fn test_sender_wrong_purpose() {
        let sender = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::AUTHENTICATION,
        )]);
        let recipient = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::DECRYPTION,
        )]);

        let result = validate_contact_request(&sender, 0, &recipient, 0);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("ENCRYPTION")));
    }

    #[test]
    fn test_recipient_key_missing() {
        let sender = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::ENCRYPTION,
        )]);
        let recipient = make_identity(vec![]);

        let result = validate_contact_request(&sender, 0, &recipient, 5);
        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("Recipient key index 5")));
    }

    #[test]
    fn test_recipient_incompatible_key_type() {
        let sender = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::ENCRYPTION,
        )]);
        let recipient = make_identity(vec![make_key(0, KeyType::BLS12_381, Purpose::DECRYPTION)]);

        let result = validate_contact_request(&sender, 0, &recipient, 0);
        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("not compatible with ECDH")));
    }

    #[test]
    fn test_disabled_sender_key() {
        let mut key = make_key(0, KeyType::ECDSA_SECP256K1, Purpose::ENCRYPTION);
        let IdentityPublicKey::V0(ref mut k) = key;
        k.disabled_at = Some(12345);
        let sender = make_identity(vec![key]);
        let recipient = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::DECRYPTION,
        )]);

        let result = validate_contact_request(&sender, 0, &recipient, 0);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("disabled")));
    }

    #[test]
    fn test_merge() {
        let mut a = ContactRequestValidation::new();
        a.add_warning("warn1".to_string());

        let mut b = ContactRequestValidation::new();
        b.add_error("err1".to_string());

        a.merge(b);
        assert!(!a.is_valid);
        assert_eq!(a.errors.len(), 1);
        assert_eq!(a.warnings.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Key-purpose alignment. The verified testnet reality
    // (368 on-chain docs): the dominant mobile cohort
    // references an UNBOUND ENCRYPTION key for BOTH senderKeyIndex and
    // recipientKeyIndex (mobile identities carry no DECRYPTION key); the
    // newest cohort uses bound ENCRYPTION(sender)/DECRYPTION(recipient).
    // Consensus enforces neither purpose nor boundedness. So the validator
    // must accept ENCRYPTION for the sender and ENCRYPTION-or-DECRYPTION for
    // the recipient, keep the ECDSA type gate, and reject AUTHENTICATION.
    // -----------------------------------------------------------------------

    /// Mobile-cohort shape: sender references an ENCRYPTION key, recipient
    /// (our key) is ALSO an ENCRYPTION key (mobile identities have no
    /// DECRYPTION key). This must pass. The companion AUTHENTICATION test
    /// below pins the recipient-purpose gate: without that gate an
    /// AUTHENTICATION recipient key is silently accepted (it "passes" for
    /// the wrong reason).
    #[test]
    fn mobile_cohort_recipient_encryption_key_is_accepted() {
        let sender = make_identity(vec![make_key(
            2,
            KeyType::ECDSA_SECP256K1,
            Purpose::ENCRYPTION,
        )]);
        let recipient = make_identity(vec![make_key(
            2,
            KeyType::ECDSA_SECP256K1,
            Purpose::ENCRYPTION,
        )]);

        let result = validate_contact_request(&sender, 2, &recipient, 2);
        assert!(
            result.is_valid,
            "mobile-cohort ENC/ENC request must validate, errors: {:?}",
            result.errors
        );
        assert!(!result.purpose_mismatch);
    }

    /// A recipient key of purpose AUTHENTICATION must FAIL validation (legacy
    /// 2024 cohort / test-noise shape). Without the recipient-purpose gate an
    /// AUTHENTICATION recipient key is silently accepted and a wrong shared
    /// secret could be derived.
    #[test]
    fn recipient_authentication_key_is_rejected_as_purpose_mismatch() {
        let sender = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::ENCRYPTION,
        )]);
        let recipient = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::AUTHENTICATION,
        )]);

        let result = validate_contact_request(&sender, 0, &recipient, 0);
        assert!(
            !result.is_valid,
            "an AUTHENTICATION recipient key must be rejected"
        );
        assert!(
            result.purpose_mismatch,
            "an AUTHENTICATION recipient is a PURPOSE mismatch (non-permanent skip), not a hard/permanent failure"
        );
        assert!(result.errors.iter().any(|e| e.contains("ENCRYPTION")
            || e.contains("DECRYPTION")
            || e.contains("purpose")));
    }

    /// Sender ENCRYPTION + recipient DECRYPTION (our existing convention,
    /// the newest 2026 cohort) still validates and is not a purpose mismatch.
    #[test]
    fn bound_convention_enc_dec_still_validates() {
        let sender = make_identity(vec![make_key(
            4,
            KeyType::ECDSA_SECP256K1,
            Purpose::ENCRYPTION,
        )]);
        let recipient = make_identity(vec![make_key(
            5,
            KeyType::ECDSA_SECP256K1,
            Purpose::DECRYPTION,
        )]);

        let result = validate_contact_request(&sender, 4, &recipient, 5);
        assert!(result.is_valid, "errors: {:?}", result.errors);
        assert!(!result.purpose_mismatch);
    }

    /// A sender key of purpose AUTHENTICATION is a purpose mismatch (the
    /// classification flag must be set so the sweep/accept paths skip rather
    /// than permanently break the channel).
    #[test]
    fn sender_authentication_key_is_a_purpose_mismatch() {
        let sender = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::AUTHENTICATION,
        )]);
        let recipient = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::DECRYPTION,
        )]);

        let result = validate_contact_request(&sender, 0, &recipient, 0);
        assert!(!result.is_valid);
        assert!(
            result.purpose_mismatch,
            "a sender purpose mismatch must be flagged so the channel is not permanently broken"
        );
    }

    /// A NON-purpose failure (wrong key type) must NOT set `purpose_mismatch`
    /// — it stays a hard/permanent failure that breaks the channel.
    #[test]
    fn wrong_key_type_is_not_a_purpose_mismatch() {
        let sender = make_identity(vec![make_key(0, KeyType::BLS12_381, Purpose::ENCRYPTION)]);
        let recipient = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::DECRYPTION,
        )]);

        let result = validate_contact_request(&sender, 0, &recipient, 0);
        assert!(!result.is_valid);
        assert!(
            !result.purpose_mismatch,
            "a key-TYPE failure is permanent, not a purpose mismatch"
        );
    }

    /// **#5 — a purpose mismatch that co-occurs with a hard error must NOT be
    /// downgraded to a skip.** `add_purpose_error` flags `purpose_mismatch`
    /// even when a genuinely-permanent hard error (disabled / missing /
    /// wrong-type key) is also present; reading the bare flag to decide
    /// skip-vs-break would mask that permanent fault into a retry-forever
    /// loop. `is_purpose_only()` is the correct gate.
    #[test]
    fn purpose_mismatch_with_hard_error_is_not_purpose_only() {
        let mut v = ContactRequestValidation::new();
        v.add_purpose_error("recipient key purpose is AUTHENTICATION".into());
        v.add_error("sender key is disabled".into());

        assert!(!v.is_valid);
        assert!(v.purpose_mismatch, "the purpose flag is still raised");
        assert!(
            !v.is_purpose_only(),
            "a purpose mismatch alongside a hard error is NOT purpose-only — must stay permanent"
        );
    }

    /// A lone purpose mismatch IS purpose-only → skippable.
    #[test]
    fn lone_purpose_mismatch_is_purpose_only() {
        let mut v = ContactRequestValidation::new();
        v.add_purpose_error("recipient key purpose is AUTHENTICATION".into());
        assert!(v.is_purpose_only());
    }

    /// A lone hard error is never purpose-only.
    #[test]
    fn lone_hard_error_is_not_purpose_only() {
        let mut v = ContactRequestValidation::new();
        v.add_error("sender key is disabled".into());
        assert!(!v.is_purpose_only());
    }

    /// `merge` must carry the `hard_error` flag so a hard fault in a merged
    /// sub-result can't be lost (which would re-open the masking bug).
    #[test]
    fn merge_propagates_hard_error() {
        let mut a = ContactRequestValidation::new();
        a.add_purpose_error("purpose".into());
        let mut b = ContactRequestValidation::new();
        b.add_error("hard".into());
        a.merge(b);
        assert!(a.purpose_mismatch);
        assert!(a.hard_error);
        assert!(!a.is_purpose_only());
    }

    // -----------------------------------------------------------------------
    // Pubkey-binding policy (`pubkey_binds_expected_key_data`). The 33/20 split
    // is the sign-time / verify-time binding shared by the FFI resolver path;
    // these pin that it matches AND fails closed on the wrong bytes, and that
    // it is byte-for-byte identical to `validate_private_key_bytes`.
    // -----------------------------------------------------------------------

    /// Derive the compressed secp256k1 pubkey (`[u8; 33]`) for a fixed
    /// in-range scalar — the shape a breadcrumb re-derivation produces.
    fn fixed_scalar_and_compressed_pubkey() -> ([u8; 32], [u8; 33]) {
        use dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
        let mut scalar = [0u8; 32];
        scalar[31] = 7;
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&scalar).expect("in-range scalar");
        let pubkey = PublicKey::from_secret_key(&secp, &sk).serialize();
        (scalar, pubkey)
    }

    #[test]
    fn binds_matching_33_byte_pubkey() {
        let (_scalar, pubkey) = fixed_scalar_and_compressed_pubkey();
        assert!(pubkey_binds_expected_key_data(&pubkey, &pubkey));
    }

    #[test]
    fn rejects_wrong_33_byte_pubkey() {
        let (_scalar, pubkey) = fixed_scalar_and_compressed_pubkey();
        // A syntactically valid compressed-pubkey prefix, wrong key.
        let wrong = [0x02u8; 33];
        assert!(!pubkey_binds_expected_key_data(&pubkey, &wrong));
    }

    #[test]
    fn binds_matching_20_byte_hash() {
        use dpp::util::hash::ripemd160_sha256;
        let (_scalar, pubkey) = fixed_scalar_and_compressed_pubkey();
        let hash = ripemd160_sha256(&pubkey);
        assert!(pubkey_binds_expected_key_data(&pubkey, &hash));
    }

    #[test]
    fn rejects_wrong_20_byte_hash() {
        use dpp::util::hash::ripemd160_sha256;
        let (_scalar, pubkey) = fixed_scalar_and_compressed_pubkey();
        // ripemd160_sha256 of an unrelated pubkey — valid-shaped, wrong hash.
        let wrong = ripemd160_sha256(&[0x03u8; 33]);
        assert!(!pubkey_binds_expected_key_data(&pubkey, &wrong));
    }

    /// An expected length that is neither 33 nor 20 must fail closed — never
    /// silently bind (guards a caller passing a 32-byte scalar or a 65-byte
    /// uncompressed key by mistake).
    #[test]
    fn malformed_expected_length_fails_closed() {
        let (_scalar, pubkey) = fixed_scalar_and_compressed_pubkey();
        assert!(!pubkey_binds_expected_key_data(&pubkey, &[0x02u8; 32]));
        assert!(!pubkey_binds_expected_key_data(&pubkey, &[0x02u8; 65]));
        assert!(!pubkey_binds_expected_key_data(&pubkey, &[]));
    }

    /// The pubkey-only binding decision is byte-for-byte identical to
    /// `IdentityPublicKey::validate_private_key_bytes` (which decides from the
    /// secret scalar) for both ECDSA representations — the guarantee that the
    /// FFI sign-time binding cannot drift from the discovery-time ownership
    /// decision. Mirrors `discovery.rs::pubkey_verify_matches_scalar_verify_*`.
    #[test]
    fn binding_matches_validate_private_key_bytes_for_both_ecdsa_types() {
        use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
        use dpp::util::hash::ripemd160_sha256;

        let network = dashcore::Network::Testnet;
        let (scalar, pubkey) = fixed_scalar_and_compressed_pubkey();

        // ECDSA_SECP256K1: on-chain data = the 33-byte compressed pubkey.
        let secp_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(pubkey.to_vec()),
            disabled_at: None,
        });
        // ECDSA_HASH160: on-chain data = ripemd160_sha256 of the pubkey.
        let hash160_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: false,
            data: dpp::platform_value::BinaryData::new(ripemd160_sha256(&pubkey).to_vec()),
            disabled_at: None,
        });

        for key in [&secp_key, &hash160_key] {
            let expected = key.data().as_slice();
            let scalar_decision = key
                .validate_private_key_bytes(&scalar, network)
                .unwrap_or(false);
            let pubkey_decision = pubkey_binds_expected_key_data(&pubkey, expected);
            assert_eq!(
                scalar_decision,
                pubkey_decision,
                "pubkey-binding diverged from validate_private_key_bytes for {:?}",
                key.key_type()
            );
            assert!(pubkey_decision, "the correct key must bind");
        }
    }
}
