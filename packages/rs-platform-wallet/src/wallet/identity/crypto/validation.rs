//! Key validation for DashPay contact requests.
//!
//! [`validate_contact_request`] judges a request already on chain: liberal,
//! because the document is immutable. `validate_contact_request_to_send`
//! adds what DIP-15 asks of a request we are about to mint: both keys valid,
//! i.e. not disabled.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, KeyType};

/// Result of validating a contact request before it is sent.
#[derive(Debug, Clone)]
pub struct ContactRequestValidation {
    /// Whether the contact request is valid and safe to send.
    pub is_valid: bool,
    /// Hard errors that prevent the request from being sent.
    pub errors: Vec<String>,
    /// Non-fatal warnings the caller may want to surface.
    pub warnings: Vec<String>,
    /// `true` when at least one permanent error was recorded (a wrong key
    /// type, or a disabled key on the send path). See
    /// [`is_permanent`](Self::is_permanent).
    pub hard_error: bool,
}

impl Default for ContactRequestValidation {
    fn default() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            hard_error: false,
        }
    }
}

impl ContactRequestValidation {
    /// Create a new, initially-valid validation result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a hard (permanent) error: sets `is_valid = false` AND flags
    /// `hard_error`.
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
        self.hard_error = true;
    }

    /// Add an ABSENT-KEY error: the referenced key id does not exist on the
    /// identity *today*. Sets `is_valid = false` but NOT `hard_error`, because
    /// identities gain keys — see [`is_permanent`](Self::is_permanent).
    pub fn add_absent_key_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
    }

    /// Add a non-fatal warning.
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Whether this failure can never resolve on its own — the only kind that
    /// may permanently break a contact's payment channel.
    ///
    /// The distinction is not "did validation fail" but "can the world change
    /// such that it stops failing". A `contactRequest` clears consensus without
    /// consensus checking anything about the keys it names, so a document can
    /// reference a key id our identity does not have *yet*: identities gain
    /// keys (that is what the DashPay enablement flow does, and what
    /// dashwallet-ios#981 exists to notice when it happened on another device).
    /// Recording that as permanent turns a temporary gap into a relationship
    /// the user cannot repair — only a fresh request from the CONTACT clears
    /// the flag.
    ///
    /// So an absent key is retryable. What stays
    /// permanent is what immutable facts make impossible: a key whose *type*
    /// cannot do ECDH. A disabled key is not among them: DIP-11 keeps disabled
    /// keys usable for verification, and the request named the key while it
    /// was valid, so the receive validators only warn about it.
    pub fn is_permanent(&self) -> bool {
        self.hard_error
    }

    /// Merge another validation result into this one.
    pub fn merge(&mut self, other: ContactRequestValidation) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        if !other.is_valid {
            self.is_valid = false;
        }
        if other.hard_error {
            self.hard_error = true;
        }
    }
}

/// Validate a contact request against the verified on-chain envelope.
///
/// Consensus enforces neither purpose nor boundedness on `senderKeyIndex` /
/// `recipientKeyIndex`, and a `contactRequest` document is immutable — so this
/// validator is *liberal on receive* by necessity. Rejecting a document is not
/// a retry, it is a permanent sentence on a contact relationship the user
/// cannot renegotiate. What it keeps strict is what actually carries weight:
/// the ECDSA key-*type* gate (ECDH needs the full secp256k1 key). A key
/// disabled since the request was sent is only a warning: DIP-11 keeps
/// disabled keys usable for verification, and DIP-15's "valid key" rule
/// binds the sender when minting, which
/// `validate_contact_request_to_send` enforces for our own requests.
///
/// Three live cohorts are known:
/// - **Newest** — bound `ENCRYPTION(sender)` / `DECRYPTION(recipient)`, our
///   original convention.
/// - **Mobile** — an unbound `ENCRYPTION` key for BOTH indices; these
///   identities carry no DECRYPTION key at all. (Both of the above come from
///   a 368-document *testnet* census.)
/// - **Legacy Android/dashj** — references the recipient's `AUTHENTICATION`
///   (key ids 0-2) or `TRANSFER` (key id 3) key, sometimes pairing it with an
///   `AUTHENTICATION` sender key. Absent from the testnet census and
///   discovered only from **mainnet** device logs (2026-08): 27 of one
///   wallet's 29 contacts, every one of them established before the iOS
///   client existed. Under the previous, testnet-calibrated policy all 27
///   were permanently unpayable.
///
/// # Checks performed
///
/// **Sender key:**
/// - Key at `sender_key_index` exists on the sender identity.
/// - Key type is `ECDSA_SECP256K1` (required for ECDH).
/// - A disabled key is a warning.
///
/// **Recipient key (our key):**
/// - Key at `recipient_key_index` exists on the recipient identity.
/// - Key type is compatible (`ECDSA_SECP256K1` or `ECDSA_HASH160`).
/// - A disabled key is a warning.
///
/// Key purpose is not checked: DIP-15 constrains these keys by type only, and
/// ECDH is defined over the secp256k1 keypair whatever its purpose. Purpose
/// policy belongs to the requests we *mint*
/// (`dash_sdk::platform::dashpay::recipient_key_purpose_is_valid` and the
/// own-key selector), where key separation can still be practiced.
pub fn validate_contact_request(
    sender_identity: &Identity,
    sender_key_index: u32,
    recipient_identity: &Identity,
    recipient_key_index: u32,
) -> ContactRequestValidation {
    let mut validation = validate_sender_key(sender_identity, sender_key_index);
    validation.merge(validate_recipient_key(
        recipient_identity,
        recipient_key_index,
    ));
    validation
}

/// [`validate_contact_request`] for a request we are about to mint, plus
/// DIP-15's requirement that both key indices name a *valid* key: a disabled
/// key on either side is a hard error. Encrypting to a revoked key would hand
/// the payment xpub to whoever holds its private half.
pub(crate) fn validate_contact_request_to_send(
    sender_identity: &Identity,
    sender_key_index: u32,
    recipient_identity: &Identity,
    recipient_key_index: u32,
) -> ContactRequestValidation {
    let mut validation = validate_contact_request(
        sender_identity,
        sender_key_index,
        recipient_identity,
        recipient_key_index,
    );
    for (side, identity, key_index) in [
        ("Sender", sender_identity, sender_key_index),
        ("Recipient", recipient_identity, recipient_key_index),
    ] {
        if let Some(disabled_at) = identity
            .get_public_key_by_id(key_index)
            .and_then(|key| key.disabled_at())
        {
            validation.add_error(format!(
                "{side} key {key_index} is disabled (at timestamp {disabled_at}); a new \
                 contact request needs a valid key",
            ));
        }
    }
    validation
}

/// The sender half of [`validate_contact_request`] — the checks that need the
/// **counterparty's** identity.
///
/// Crate-private: external callers go through the complete
/// [`validate_contact_request`] contract. Split out so the deferred-crypto
/// drain can run the recipient half first — see [`validate_recipient_key`] for
/// why that ordering matters, and what it changes for a mixed failure.
pub(crate) fn validate_sender_key(
    sender_identity: &Identity,
    sender_key_index: u32,
) -> ContactRequestValidation {
    let mut validation = ContactRequestValidation::new();

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

            // Disabled since the request was sent: still usable (DIP-11).
            if let Some(disabled_at) = key.disabled_at() {
                validation.add_warning(format!(
                    "Sender key {} is disabled (at timestamp {}); accepted, as the request \
                     named it before",
                    sender_key_index, disabled_at,
                ));
            }
        }
        None => {
            validation.add_absent_key_error(format!(
                "Sender key index {} not found on identity {}",
                sender_key_index,
                sender_identity.id(),
            ));
        }
    }

    validation
}

/// The recipient half of [`validate_contact_request`] — the checks that need
/// only **our own** identity, which is always already resident.
///
/// Split out because the deferred-crypto drain would otherwise pay a Platform
/// round trip (`Identity::fetch` of the contact) before it could discover that
/// the request is unusable for a reason it could have known locally. An entry
/// naming a key our identity does not have yet stays queued by design — the
/// identity can gain it — so that fetch would repeat on every sweep. (Mainnet
/// logs from one wallet once showed 27 contacts and 396 such
/// fetch-then-reject cycles in a single session.) Running this half first
/// costs nothing and removes the network entirely from that loop.
///
/// # What this changes for a MIXED failure
///
/// Deciding on this half alone is a real policy change, not just a reordering.
/// When our key is absent AND the sender's key carries a hard fault (a key
/// type that cannot do ECDH), the composed [`validate_contact_request`] would
/// merge both, see `hard_error`, and mark the channel permanently broken.
/// Stopping here classifies it retryable and leaves it queued.
///
/// That is the intended outcome. The `hard_error` precedence exists to stop a
/// genuinely permanent fault from becoming a retry-forever loop — but the
/// forever-loop it guards against was expensive precisely because each retry
/// fetched. With the fetch gone, a retryable entry costs a map lookup
/// per sweep, while marking the channel broken is unappealable by the user:
/// only a fresh request from the CONTACT clears it. Deferring the broken mark
/// until the fault is one we can see locally trades a cheap retry for an
/// irreversible one. A sender-side hard fault still marks the channel broken
/// the moment our own key stops being the blocker.
pub(crate) fn validate_recipient_key(
    recipient_identity: &Identity,
    recipient_key_index: u32,
) -> ContactRequestValidation {
    let mut validation = ContactRequestValidation::new();

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

            // Disabled since the request was sent: we still derive its
            // scalar, so the request stays usable (DIP-11).
            if let Some(disabled_at) = key.disabled_at() {
                validation.add_warning(format!(
                    "Recipient key {} is disabled (at timestamp {}); accepted, as the request \
                     named it before",
                    recipient_key_index, disabled_at,
                ));
            }
        }
        None => {
            validation.add_absent_key_error(format!(
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
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
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

    fn disabled_key(id: u32, purpose: Purpose) -> IdentityPublicKey {
        let mut key = make_key(id, KeyType::ECDSA_SECP256K1, purpose);
        key.set_disabled_at(12345);
        key
    }

    /// DIP-11: disabled keys "can be used for signature verification". A
    /// received request is immutable history that named a key valid when it
    /// was sent; the sender disabling that key since must not lock us out of
    /// the channel. It is a warning, never a failure.
    #[test]
    fn should_accept_a_received_request_naming_a_since_disabled_sender_key() {
        let sender = make_identity(vec![disabled_key(0, Purpose::ENCRYPTION)]);
        let recipient = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::DECRYPTION,
        )]);

        let result = validate_contact_request(&sender, 0, &recipient, 0);
        assert!(result.is_valid, "errors: {:?}", result.errors);
        assert!(!result.is_permanent());
        assert!(result.warnings.iter().any(|w| w.contains("disabled")));

        let sender_half = validate_sender_key(&sender, 0);
        assert!(sender_half.is_valid, "errors: {:?}", sender_half.errors);
    }

    /// Our own key disabled after the request arrived: we still derive its
    /// scalar, so the request stays usable.
    #[test]
    fn should_accept_a_received_request_naming_our_since_disabled_recipient_key() {
        let sender = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::ENCRYPTION,
        )]);
        let recipient = make_identity(vec![disabled_key(0, Purpose::DECRYPTION)]);

        let result = validate_contact_request(&sender, 0, &recipient, 0);
        assert!(result.is_valid, "errors: {:?}", result.errors);
        assert!(result.warnings.iter().any(|w| w.contains("disabled")));

        let recipient_half = validate_recipient_key(&recipient, 0);
        assert!(
            recipient_half.is_valid,
            "errors: {:?}",
            recipient_half.errors
        );
        assert!(!recipient_half.is_permanent());
    }

    /// DIP-15 requires a valid key when SENDING: neither side may be disabled.
    #[test]
    fn should_refuse_to_send_with_a_disabled_key() {
        let enabled_sender = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::ENCRYPTION,
        )]);
        let enabled_recipient = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::DECRYPTION,
        )]);
        let disabled_sender = make_identity(vec![disabled_key(0, Purpose::ENCRYPTION)]);
        let disabled_recipient = make_identity(vec![disabled_key(0, Purpose::DECRYPTION)]);

        let valid = validate_contact_request_to_send(&enabled_sender, 0, &enabled_recipient, 0);
        assert!(valid.is_valid, "errors: {:?}", valid.errors);

        for (sender, recipient, side) in [
            (&disabled_sender, &enabled_recipient, "Sender"),
            (&enabled_sender, &disabled_recipient, "Recipient"),
        ] {
            let result = validate_contact_request_to_send(sender, 0, recipient, 0);
            assert!(
                !result.is_valid,
                "a disabled {side} key must refuse the send"
            );
            assert!(
                result
                    .errors
                    .iter()
                    .any(|e| e.starts_with(side) && e.contains("disabled")),
                "errors: {:?}",
                result.errors
            );
        }
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
    // Consensus enforces neither purpose nor boundedness, and mainnet adds a
    // third, legacy Android/dashj cohort referencing AUTHENTICATION/TRANSFER
    // keys. DIP-15 constrains these keys by type only, so the validator
    // accepts every purpose on receive and keeps the ECDSA type gate — the
    // check that actually protects the ECDH.
    // -----------------------------------------------------------------------

    /// Mobile-cohort shape: sender references an ENCRYPTION key, recipient
    /// (our key) is ALSO an ENCRYPTION key (mobile identities have no
    /// DECRYPTION key). This must pass.
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
    }

    /// Legacy Android/dashj shape: the inbound request references OUR
    /// AUTHENTICATION or TRANSFER key. Both must validate.
    ///
    /// This is the regression guard for the mainnet bug where 27 of a
    /// wallet's 29 contacts — every one established before the iOS client
    /// existed — were permanently unpayable. The deferred account build kept
    /// failing `key-purpose mismatch`, so `send_payment` found no
    /// `DashpayExternalAccount` and the user saw "call
    /// register_external_contact_account first" forever. The documents are
    /// immutable: no user action could have fixed it.
    #[test]
    fn legacy_dashj_recipient_key_purposes_are_accepted() {
        for purpose in [Purpose::AUTHENTICATION, Purpose::TRANSFER] {
            let sender = make_identity(vec![make_key(
                0,
                KeyType::ECDSA_SECP256K1,
                Purpose::ENCRYPTION,
            )]);
            let recipient = make_identity(vec![make_key(0, KeyType::ECDSA_SECP256K1, purpose)]);

            let result = validate_contact_request(&sender, 0, &recipient, 0);
            assert!(
                result.is_valid,
                "a {purpose:?} recipient key must be accepted from an immutable on-chain \
                 document, errors: {:?}",
                result.errors
            );
        }
    }

    /// The whole legacy pair — AUTHENTICATION sender against AUTHENTICATION
    /// recipient — is the exact shape 15 of the logged mainnet failures took.
    #[test]
    fn legacy_dashj_authentication_pair_is_accepted() {
        let sender = make_identity(vec![make_key(
            1,
            KeyType::ECDSA_SECP256K1,
            Purpose::AUTHENTICATION,
        )]);
        let recipient = make_identity(vec![make_key(
            1,
            KeyType::ECDSA_SECP256K1,
            Purpose::AUTHENTICATION,
        )]);

        let result = validate_contact_request(&sender, 1, &recipient, 1);
        assert!(result.is_valid, "errors: {:?}", result.errors);
    }

    /// A key id the identity does not have **yet** must not be permanent.
    ///
    /// Identities gain keys — that is what the DashPay enablement flow does,
    /// and dashwallet-ios#981 exists to notice it happening on another device.
    /// A `contactRequest` clears consensus without consensus checking anything
    /// about the keys it names, and it can never be re-minted, so recording
    /// "we have no key 5 today" as a permanent verdict ends a relationship over
    /// a gap that may close on its own — and only the CONTACT can clear the
    /// flag, so the user cannot appeal it.
    #[test]
    fn an_absent_key_is_not_a_permanent_fault() {
        let sender = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::ENCRYPTION,
        )]);
        let recipient = make_identity(vec![]);

        let result = validate_contact_request(&sender, 0, &recipient, 0);
        assert!(!result.is_valid, "an absent key still fails validation");
        assert!(
            !result.is_permanent(),
            "but it must be retryable: the identity can gain the key later"
        );
    }

    /// A key type that cannot do ECDH is permanent — a key's type is fixed for
    /// its lifetime, so no future state makes this request usable.
    #[test]
    fn a_non_ecdh_key_type_is_a_permanent_fault() {
        let sender = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::ENCRYPTION,
        )]);
        let recipient = make_identity(vec![make_key(0, KeyType::BLS12_381, Purpose::ENCRYPTION)]);

        let result = validate_contact_request(&sender, 0, &recipient, 0);
        assert!(!result.is_valid);
        assert!(
            result.is_permanent(),
            "a BLS key can never do secp256k1 ECDH, so this one may break the channel"
        );
    }

    const ALL_PURPOSES: [Purpose; 7] = [
        Purpose::AUTHENTICATION,
        Purpose::ENCRYPTION,
        Purpose::DECRYPTION,
        Purpose::TRANSFER,
        Purpose::SYSTEM,
        Purpose::VOTING,
        Purpose::OWNER,
    ];

    /// DIP-15 constrains the keys of a contact request by type only (ECDH
    /// needs secp256k1), never by purpose, so a received request must not be
    /// refused for the purpose of either key.
    #[test]
    fn should_accept_a_received_request_whatever_the_key_purposes() {
        for sender_purpose in ALL_PURPOSES {
            for recipient_purpose in ALL_PURPOSES {
                let sender =
                    make_identity(vec![make_key(0, KeyType::ECDSA_SECP256K1, sender_purpose)]);
                let recipient = make_identity(vec![make_key(
                    0,
                    KeyType::ECDSA_SECP256K1,
                    recipient_purpose,
                )]);

                let result = validate_contact_request(&sender, 0, &recipient, 0);
                assert!(
                    result.is_valid,
                    "{sender_purpose:?} sender / {recipient_purpose:?} recipient: {:?}",
                    result.errors
                );
            }
        }
    }

    /// The type gate is the genuine one and stays: a key that cannot do
    /// secp256k1 ECDH fails permanently, whatever its purpose.
    #[test]
    fn should_still_refuse_a_key_type_that_cannot_do_ecdh() {
        let enabled = make_identity(vec![make_key(
            0,
            KeyType::ECDSA_SECP256K1,
            Purpose::ENCRYPTION,
        )]);
        for purpose in ALL_PURPOSES {
            let bls = make_identity(vec![make_key(0, KeyType::BLS12_381, purpose)]);
            for result in [
                validate_contact_request(&bls, 0, &enabled, 0),
                validate_contact_request(&enabled, 0, &bls, 0),
            ] {
                assert!(!result.is_valid, "{purpose:?}");
                assert!(result.is_permanent(), "{purpose:?}");
            }
        }
    }

    /// `merge` must carry the `hard_error` flag so a hard fault in a merged
    /// sub-result can't be lost.
    #[test]
    fn should_propagate_a_hard_error_through_merge() {
        let mut a = ContactRequestValidation::new();
        a.add_absent_key_error("absent".into());
        let mut b = ContactRequestValidation::new();
        b.add_error("hard".into());
        a.merge(b);
        assert!(!a.is_valid);
        assert!(a.is_permanent());
    }

    /// Sender ENCRYPTION + recipient DECRYPTION (our existing convention,
    /// the newest 2026 cohort) still validates.
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
        use dpp::secp256k1::{PublicKey, SecretKey};
        let mut scalar = [0u8; 32];
        scalar[31] = 7;
        let sk = SecretKey::from_secret_bytes(scalar).expect("in-range scalar");
        let pubkey = PublicKey::from_secret_key(&sk).serialize();
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
