//! Pre-send validation for DashPay contact requests.
//!
//! Validates that the sender and recipient identities have the correct key
//! types and purposes before a contact request is submitted to the platform.

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
    /// `true` when the *only* reason the request is invalid is a key-PURPOSE
    /// mismatch (e.g. a legacy 2024 doc referencing an AUTHENTICATION key).
    ///
    /// This classification is load-bearing for the sync sweep / accept paths
    /// (G15): a purpose mismatch must NOT mark the payment channel
    /// **permanently** broken — on-chain history demonstrably contains
    /// nonconforming-but-honest documents, and our acceptance policy (not the
    /// immutable request) is what might change. A purpose-only failure is a
    /// non-permanent skip (log + retry next sweep); a key-TYPE / missing-key /
    /// disabled-key failure stays permanent.
    pub purpose_mismatch: bool,
}

impl Default for ContactRequestValidation {
    fn default() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            purpose_mismatch: false,
        }
    }
}

impl ContactRequestValidation {
    /// Create a new, initially-valid validation result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a hard error (sets `is_valid = false`).
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
    }

    /// Add a key-PURPOSE error: sets `is_valid = false` AND flags
    /// `purpose_mismatch` so callers can downgrade this to a non-permanent
    /// skip rather than a permanent broken-channel mark (G15).
    pub fn add_purpose_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
        self.purpose_mismatch = true;
    }

    /// Add a non-fatal warning.
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
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
    }
}

/// Validate a contact request against the verified on-chain envelope (G15).
///
/// The empirical testnet census (368 docs, research/06 §G15) shows two live
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
            // purpose is a non-permanent purpose mismatch (G15).
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

            // Purpose must be ENCRYPTION or DECRYPTION (G15): the mobile
            // cohort's recipientKeyIndex points at an ENCRYPTION key, the
            // newest cohort's at a DECRYPTION key — both honest. Anything
            // else (AUTHENTICATION/MASTER/TRANSFER) is a non-permanent purpose
            // mismatch: legacy 2024 docs reference AUTHENTICATION keys, so we
            // skip-and-retry rather than permanently break the channel.
            match key.purpose() {
                Purpose::ENCRYPTION | Purpose::DECRYPTION => {}
                other => {
                    validation.add_purpose_error(format!(
                        "Recipient key {} has purpose {:?}, but ENCRYPTION or DECRYPTION is \
                         required for contact requests",
                        recipient_key_index, other,
                    ));
                }
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
    // G15 key-purpose alignment (M1 task 9). The verified testnet reality
    // (368 on-chain docs, research/06 §G15): the dominant mobile cohort
    // references an UNBOUND ENCRYPTION key for BOTH senderKeyIndex and
    // recipientKeyIndex (mobile identities carry no DECRYPTION key); the
    // newest cohort uses bound ENCRYPTION(sender)/DECRYPTION(recipient).
    // Consensus enforces neither purpose nor boundedness. So the validator
    // must accept ENCRYPTION for the sender and ENCRYPTION-or-DECRYPTION for
    // the recipient, keep the ECDSA type gate, and reject AUTHENTICATION.
    // -----------------------------------------------------------------------

    /// Mobile-cohort shape: sender references an ENCRYPTION key, recipient
    /// (our key) is ALSO an ENCRYPTION key (mobile identities have no
    /// DECRYPTION key). This must pass — RED before task 9 because the
    /// recipient side had no purpose gate at all, so it "passed" for the
    /// wrong reason; the companion AUTHENTICATION test below is the one that
    /// proves the gate was previously missing.
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
    /// 2024 cohort / test-noise shape). RED before task 9: the recipient side
    /// had NO purpose check, so an AUTHENTICATION recipient key was silently
    /// accepted and a wrong shared secret could be derived.
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
}
