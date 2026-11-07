//! The standard private document envelope, version 1.
//!
//! A private document type (protocol version 15) stores fixed-size entries that Platform never
//! reads. Clients fill each one with this envelope: a payload encrypted to one recipient key,
//! which the recipient finds by trying to open every entry of the type, and which the sender can
//! recover later from its outgoing viewing key alone. An entry of `entry_size` bytes is
//!
//! ```text
//! cm (32) | epk (33) | body_ct (B + 16) | out_ct (81)        B = entry_size - 162
//! ```
//!
//! derived from a fresh 32-byte `esk_seed`, the recipient's secp256k1 key `R` and the sender's
//! outgoing viewing key `ovk` (`H` is SHA-256, `ser` a compressed point, `||` concatenation):
//!
//! ```text
//! esk     = H("Dash/PrivDoc/v1/esk" || esk_seed), refused if not a valid secret key
//! epk     = esk * G
//! key     = H("Dash/PrivDoc/v1/key" || ser(esk * R) || ser(epk) || ser(R))
//! cm      = H("Dash/PrivDoc/v1/cm" || key || contract_id || u32le(len(type)) || type || ser(R)
//!             || body)
//! body_ct = ChaCha20-Poly1305(key, nonce 0, aad cm, body)
//! ock     = H("Dash/PrivDoc/v1/ock" || ovk || ser(epk))
//! out_ct  = ChaCha20-Poly1305(ock, nonce 0, aad cm, ser(R) || esk_seed)
//! ```
//!
//! `cm` is keyed, so nobody without `key` can confirm a guess about an entry, and commits to
//! `R`, so a ciphertext that would also open under another key is refused there. It commits to
//! the contract and type too, so an entry copied into another type does not open. A zero nonce is
//! safe because each key encrypts once: `esk_seed` must be fresh for every entry built.
//!
//! The body, `B` bytes, is
//!
//! ```text
//! version (1) | auth mode (1) | sender identity id (32) | sender key id (u32le) | auth (65)
//!   | recipient identity id (32) | payload length (u16le) | payload | zero padding
//! ```
//!
//! and the authentication covers
//!
//! ```text
//! m = H("Dash/PrivDoc/v1/auth" || version || auth mode || contract_id || u32le(len(type))
//!       || type || sender identity id || sender key id || recipient identity id
//!       || payload length || payload)
//! ```
//!
//! - anonymous (0): sender fields and `auth` are zeros;
//! - deniable MAC (1): `auth` is
//!   `HMAC-SHA256(H("Dash/PrivDoc/v1/mac" || ser(sender key * R)), m)`, then zeros. The
//!   recipient learns who sent the entry but could have computed the MAC itself, so it cannot
//!   prove it to anyone;
//! - signature (2): `auth` is a recoverable ECDSA signature of `m` by the sender key, in the
//!   compact form (header byte `31 + recovery id`, then `r || s`), which anyone can check.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hmac::{Hmac, Mac};
use secp256k1::ecdh::shared_secret_point;
use secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use secp256k1::{Message, PublicKey, SecretKey};
use sha2::{Digest, Sha256};

/// Why an entry could not be built, opened, recovered or checked.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PrivateDocumentEnvelopeError {
    #[error("a private document payload of {payload_len} bytes does not fit an entry that holds {capacity}")]
    PayloadTooLarge { payload_len: usize, capacity: usize },

    #[error("a private document entry of {0} bytes is shorter than the envelope's 299")]
    InvalidEntryLength(usize),

    #[error("the ephemeral seed derives no valid secret key; draw another")]
    InvalidEphemeralSeed,

    #[error("a private document ciphertext does not decrypt")]
    DecryptionFailed,

    #[error("malformed private document entry: {0}")]
    Malformed(&'static str),

    #[error("private document envelope version {0} is not supported")]
    UnsupportedVersion(u8),

    #[error("the private document entry names another recipient identity")]
    WrongRecipient,

    #[error("the private document entry is anonymous: it names no sender to check")]
    AnonymousSender,

    #[error("the private document entry's sender authentication does not verify")]
    SenderNotAuthenticated,
}

/// The envelope version this module builds and opens.
pub const PRIVATE_DOCUMENT_ENVELOPE_VERSION: u8 = 1;

const COMMITMENT_LEN: usize = 32;
const EPHEMERAL_KEY_LEN: usize = 33;
const TAG_LEN: usize = 16;
const SENDER_RECOVERY_PLAINTEXT_LEN: usize = 33 + 32;
const SENDER_RECOVERY_LEN: usize = SENDER_RECOVERY_PLAINTEXT_LEN + TAG_LEN;
const AUTH_LEN: usize = 65;
const MAC_LEN: usize = 32;

/// The bytes around the encrypted body: the commitment, the ephemeral key, the body's tag and
/// the sender-recovery ciphertext.
pub const PRIVATE_DOCUMENT_ENVELOPE_WRAPPER_LEN: usize =
    COMMITMENT_LEN + EPHEMERAL_KEY_LEN + TAG_LEN + SENDER_RECOVERY_LEN;

/// The bytes of the body before the payload.
pub const PRIVATE_DOCUMENT_ENVELOPE_BODY_HEADER_LEN: usize = 1 + 1 + 32 + 4 + AUTH_LEN + 32 + 2;

/// Every byte of an entry besides its payload: 299. An entry holds a payload of at most its
/// size minus this.
pub const PRIVATE_DOCUMENT_ENVELOPE_OVERHEAD_LEN: usize =
    PRIVATE_DOCUMENT_ENVELOPE_WRAPPER_LEN + PRIVATE_DOCUMENT_ENVELOPE_BODY_HEADER_LEN;

const DOMAIN_ESK: &[u8] = b"Dash/PrivDoc/v1/esk";
const DOMAIN_KEY: &[u8] = b"Dash/PrivDoc/v1/key";
const DOMAIN_CM: &[u8] = b"Dash/PrivDoc/v1/cm";
const DOMAIN_OVK: &[u8] = b"Dash/PrivDoc/v1/ovk";
const DOMAIN_OCK: &[u8] = b"Dash/PrivDoc/v1/ock";
const DOMAIN_AUTH: &[u8] = b"Dash/PrivDoc/v1/auth";
const DOMAIN_MAC: &[u8] = b"Dash/PrivDoc/v1/mac";

/// The compact signature header of a compressed key: 27 + 4, plus the recovery id.
const COMPACT_SIGNATURE_HEADER_BASE: u8 = 31;

/// The private document type an entry belongs to. Both are bound into the commitment, so an
/// entry opens only for the type it was built for.
#[derive(Clone, Copy, Debug)]
pub struct PrivateDocumentTarget<'a> {
    pub contract_id: [u8; 32],
    pub document_type_name: &'a str,
}

/// Whom an entry is for: the identity, written in the body so that an entry cannot be replayed
/// to another identity holding the same key, and the key it is encrypted to. For DashPay, the
/// recipient's DashPay decryption key.
#[derive(Clone, Copy, Debug)]
pub struct PrivateDocumentRecipient<'a> {
    pub identity_id: [u8; 32],
    pub public_key: &'a PublicKey,
}

/// How an entry tells its recipient who wrote it.
#[derive(Clone, Copy, Debug)]
pub enum PrivateDocumentSender<'a> {
    /// Nobody: the recipient learns nothing of the writer.
    Anonymous,
    /// The sender's identity and key, with a MAC under the static ECDH of that key and the
    /// recipient's. The recipient learns who wrote the entry, but could have computed the MAC
    /// itself, so it cannot prove that to anyone. DashPay contact requests use this, with the
    /// sender's DashPay encryption key.
    DeniableMac {
        identity_id: [u8; 32],
        key_id: u32,
        secret_key: &'a SecretKey,
    },
    /// The sender's identity and key, with a signature by that key anyone can check.
    Signature {
        identity_id: [u8; 32],
        key_id: u32,
        secret_key: &'a SecretKey,
    },
}

/// The authentication an opened entry carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivateDocumentAuthMode {
    Anonymous = 0,
    DeniableMac = 1,
    Signature = 2,
}

impl PrivateDocumentAuthMode {
    fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Anonymous),
            1 => Some(Self::DeniableMac),
            2 => Some(Self::Signature),
            _ => None,
        }
    }
}

/// An entry its recipient opened, or its sender recovered.
///
/// The sender fields are what the entry claims; [`Self::verify_sender`] checks the claim
/// against the key the sender's identity holds under `sender_key_id`. The fields are read-only,
/// so what a caller reads is what was authenticated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedPrivateDocumentEntry {
    commitment: [u8; 32],
    recipient_key_index: Option<usize>,
    recipient_public_key: PublicKey,
    recipient_identity_id: [u8; 32],
    auth_mode: PrivateDocumentAuthMode,
    sender_identity_id: [u8; 32],
    sender_key_id: u32,
    payload: Vec<u8>,
    auth: [u8; AUTH_LEN],
    auth_message: [u8; 32],
}

impl OpenedPrivateDocumentEntry {
    /// The entry's commitment: two entries with the same one are copies.
    pub fn commitment(&self) -> &[u8; 32] {
        &self.commitment
    }

    /// Which of the keys given to [`open_private_document_entry`] opened it; `None` when the
    /// sender recovered it.
    pub fn recipient_key_index(&self) -> Option<usize> {
        self.recipient_key_index
    }

    /// The key the entry is encrypted to.
    pub fn recipient_public_key(&self) -> &PublicKey {
        &self.recipient_public_key
    }

    pub fn recipient_identity_id(&self) -> &[u8; 32] {
        &self.recipient_identity_id
    }

    pub fn auth_mode(&self) -> PrivateDocumentAuthMode {
        self.auth_mode
    }

    /// Zeros when the entry is anonymous.
    pub fn sender_identity_id(&self) -> &[u8; 32] {
        &self.sender_identity_id
    }

    /// Zero when the entry is anonymous.
    pub fn sender_key_id(&self) -> u32 {
        self.sender_key_id
    }

    /// The type's properties in the document property encoding, for the type's schema to
    /// decode and validate.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// The payload, taking the entry.
    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }

    /// Checks that the key the entry names wrote it: `sender_public_key` is the key the sender
    /// identity holds under `sender_key_id`, as it was when the entry was written (keys are
    /// disabled, never removed). A deniable MAC is checked with the recipient's own secret key,
    /// the one that opened the entry; a signature needs only the sender's key.
    pub fn verify_sender(
        &self,
        sender_public_key: &PublicKey,
        recipient_secret_key: &SecretKey,
    ) -> Result<(), PrivateDocumentEnvelopeError> {
        match self.auth_mode {
            PrivateDocumentAuthMode::Anonymous => {
                Err(PrivateDocumentEnvelopeError::AnonymousSender)
            }
            PrivateDocumentAuthMode::DeniableMac => {
                let mac_key = mac_key(&shared_point(recipient_secret_key, sender_public_key));
                let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&mac_key)
                    .expect("HMAC takes a key of any length");
                mac.update(&self.auth_message);
                mac.verify_slice(&self.auth[..MAC_LEN])
                    .map_err(|_| PrivateDocumentEnvelopeError::SenderNotAuthenticated)
            }
            PrivateDocumentAuthMode::Signature => {
                let recovery_id = RecoveryId::from_u8_masked(
                    self.auth[0].wrapping_sub(COMPACT_SIGNATURE_HEADER_BASE),
                );
                let signature = RecoverableSignature::from_compact(&self.auth[1..], recovery_id)
                    .map_err(|_| PrivateDocumentEnvelopeError::SenderNotAuthenticated)?;
                let signer = signature
                    .recover_ecdsa(Message::from_digest(self.auth_message))
                    .map_err(|_| PrivateDocumentEnvelopeError::SenderNotAuthenticated)?;
                if signer == *sender_public_key {
                    Ok(())
                } else {
                    Err(PrivateDocumentEnvelopeError::SenderNotAuthenticated)
                }
            }
        }
    }
}

/// The outgoing viewing key a sender derives from a secret of its wallet: with it, and nothing
/// else, the sender recovers every entry it built from the entries alone.
pub fn private_document_outgoing_viewing_key(wallet_secret: &[u8; 32]) -> [u8; 32] {
    hash(&[DOMAIN_OVK, wallet_secret])
}

/// Builds an entry of `entry_size` bytes carrying `payload` to `recipient`, for the private
/// document type `target`.
///
/// `esk_seed` must be 32 fresh random bytes for every entry built, never reused, not even on a
/// retry: the entry's keys are derived from it, and a zero nonce is safe only because each key
/// encrypts once. A seed whose hash is not a valid secret key (a chance of about 2^-128) is
/// refused with `InvalidPrivateDocumentEphemeralSeed`; draw another.
pub fn seal_private_document_entry(
    target: &PrivateDocumentTarget,
    entry_size: usize,
    recipient: &PrivateDocumentRecipient,
    sender: &PrivateDocumentSender,
    payload: &[u8],
    outgoing_viewing_key: &[u8; 32],
    esk_seed: &[u8; 32],
) -> Result<Vec<u8>, PrivateDocumentEnvelopeError> {
    if entry_size < PRIVATE_DOCUMENT_ENVELOPE_OVERHEAD_LEN {
        return Err(PrivateDocumentEnvelopeError::InvalidEntryLength(entry_size));
    }
    let capacity = entry_size - PRIVATE_DOCUMENT_ENVELOPE_OVERHEAD_LEN;
    let payload_len = u16::try_from(payload.len())
        .ok()
        .filter(|_| payload.len() <= capacity)
        .ok_or(PrivateDocumentEnvelopeError::PayloadTooLarge {
            payload_len: payload.len(),
            capacity,
        })?;
    let body_len = entry_size - PRIVATE_DOCUMENT_ENVELOPE_WRAPPER_LEN;

    let esk = ephemeral_secret_key(esk_seed)?;
    let epk = PublicKey::from_secret_key(&esk).serialize();
    let recipient_key = recipient.public_key.serialize();
    let key = entry_key(
        &shared_point(&esk, recipient.public_key),
        &epk,
        &recipient_key,
    );

    let (auth_mode, sender_identity_id, sender_key_id) = match sender {
        PrivateDocumentSender::Anonymous => (PrivateDocumentAuthMode::Anonymous, [0; 32], 0),
        PrivateDocumentSender::DeniableMac {
            identity_id,
            key_id,
            ..
        } => (PrivateDocumentAuthMode::DeniableMac, *identity_id, *key_id),
        PrivateDocumentSender::Signature {
            identity_id,
            key_id,
            ..
        } => (PrivateDocumentAuthMode::Signature, *identity_id, *key_id),
    };
    let auth_message = auth_message(
        target,
        auth_mode,
        &sender_identity_id,
        sender_key_id,
        &recipient.identity_id,
        payload,
    );
    let mut auth = [0u8; AUTH_LEN];
    match sender {
        PrivateDocumentSender::Anonymous => {}
        PrivateDocumentSender::DeniableMac { secret_key, .. } => {
            let mac_key = mac_key(&shared_point(secret_key, recipient.public_key));
            let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&mac_key)
                .expect("HMAC takes a key of any length");
            mac.update(&auth_message);
            auth[..MAC_LEN].copy_from_slice(&mac.finalize().into_bytes());
        }
        PrivateDocumentSender::Signature { secret_key, .. } => {
            let (recovery_id, compact) = RecoverableSignature::sign_ecdsa_recoverable(
                Message::from_digest(auth_message),
                secret_key,
            )
            .serialize_compact();
            auth[0] = COMPACT_SIGNATURE_HEADER_BASE + u8::from(recovery_id);
            auth[1..].copy_from_slice(&compact);
        }
    }

    let mut body = Vec::with_capacity(body_len);
    body.push(PRIVATE_DOCUMENT_ENVELOPE_VERSION);
    body.push(auth_mode as u8);
    body.extend_from_slice(&sender_identity_id);
    body.extend_from_slice(&sender_key_id.to_le_bytes());
    body.extend_from_slice(&auth);
    body.extend_from_slice(&recipient.identity_id);
    body.extend_from_slice(&payload_len.to_le_bytes());
    body.extend_from_slice(payload);
    body.resize(body_len, 0);

    let commitment = commitment(&key, target, &recipient_key, &body);
    let body_ct = encrypt(&key, &commitment, &body)?;
    let mut sender_recovery = Vec::with_capacity(SENDER_RECOVERY_PLAINTEXT_LEN);
    sender_recovery.extend_from_slice(&recipient_key);
    sender_recovery.extend_from_slice(esk_seed);
    let out_ct = encrypt(
        &outgoing_cipher_key(outgoing_viewing_key, &epk),
        &commitment,
        &sender_recovery,
    )?;

    let mut entry = Vec::with_capacity(entry_size);
    entry.extend_from_slice(&commitment);
    entry.extend_from_slice(&epk);
    entry.extend_from_slice(&body_ct);
    entry.extend_from_slice(&out_ct);
    debug_assert_eq!(entry.len(), entry_size);
    Ok(entry)
}

/// Opens `entry` of the private document type `target` with the first of
/// `recipient_secret_keys` it is encrypted to, for the identity `recipient_identity_id`.
///
/// `Ok(None)` when it is for none of the keys, the case of nearly every entry a wallet scans:
/// a key whose decryption fails, or whose commitment does not match, is passed over for the
/// next. An entry that opens but is malformed, or names another recipient identity, is refused
/// with an error. The sender is not yet checked: see
/// [`OpenedPrivateDocumentEntry::verify_sender`].
pub fn open_private_document_entry(
    entry: &[u8],
    target: &PrivateDocumentTarget,
    recipient_identity_id: &[u8; 32],
    recipient_secret_keys: &[SecretKey],
) -> Result<Option<OpenedPrivateDocumentEntry>, PrivateDocumentEnvelopeError> {
    let parts = EntryParts::split(entry)?;
    let Ok(epk) = PublicKey::from_slice(parts.epk) else {
        // No key opens an entry whose ephemeral key is not a point.
        return Ok(None);
    };
    for (index, secret_key) in recipient_secret_keys.iter().enumerate() {
        let recipient_public_key = PublicKey::from_secret_key(secret_key);
        let recipient_key = recipient_public_key.serialize();
        let key = entry_key(&shared_point(secret_key, &epk), parts.epk, &recipient_key);
        let Ok(body) = decrypt(&key, parts.commitment, parts.body_ct) else {
            continue;
        };
        if commitment(&key, target, &recipient_key, &body) != *parts.commitment {
            continue;
        }
        let opened = parse_body(
            &body,
            target,
            *parts.commitment,
            recipient_public_key,
            Some(index),
        )?;
        if opened.recipient_identity_id != *recipient_identity_id {
            return Err(PrivateDocumentEnvelopeError::WrongRecipient);
        }
        return Ok(Some(opened));
    }
    Ok(None)
}

/// Recovers `entry` of the private document type `target` that the holder of
/// `outgoing_viewing_key` built, as after a restore from seed: `Ok(None)` when it built another
/// sender's entry. An entry whose sender recovery opens but whose body does not is refused.
pub fn recover_sent_private_document_entry(
    entry: &[u8],
    target: &PrivateDocumentTarget,
    outgoing_viewing_key: &[u8; 32],
) -> Result<Option<OpenedPrivateDocumentEntry>, PrivateDocumentEnvelopeError> {
    let parts = EntryParts::split(entry)?;
    let Ok(sender_recovery) = decrypt(
        &outgoing_cipher_key(outgoing_viewing_key, parts.epk),
        parts.commitment,
        parts.out_ct,
    ) else {
        return Ok(None);
    };
    let malformed = || PrivateDocumentEnvelopeError::Malformed("sender recovery");
    let recipient_public_key =
        PublicKey::from_slice(&sender_recovery[..EPHEMERAL_KEY_LEN]).map_err(|_| malformed())?;
    let esk_seed: [u8; 32] = sender_recovery[EPHEMERAL_KEY_LEN..]
        .try_into()
        .map_err(|_| malformed())?;
    let esk = ephemeral_secret_key(&esk_seed).map_err(|_| malformed())?;
    if PublicKey::from_secret_key(&esk).serialize() != *parts.epk {
        return Err(malformed());
    }
    let recipient_key = recipient_public_key.serialize();
    let key = entry_key(
        &shared_point(&esk, &recipient_public_key),
        parts.epk,
        &recipient_key,
    );
    let body = decrypt(&key, parts.commitment, parts.body_ct).map_err(|_| malformed())?;
    if commitment(&key, target, &recipient_key, &body) != *parts.commitment {
        return Err(malformed());
    }
    parse_body(&body, target, *parts.commitment, recipient_public_key, None).map(Some)
}

/// An entry cut into its four parts.
struct EntryParts<'a> {
    commitment: &'a [u8; 32],
    epk: &'a [u8; 33],
    body_ct: &'a [u8],
    out_ct: &'a [u8],
}

impl<'a> EntryParts<'a> {
    fn split(entry: &'a [u8]) -> Result<Self, PrivateDocumentEnvelopeError> {
        if entry.len() < PRIVATE_DOCUMENT_ENVELOPE_OVERHEAD_LEN {
            return Err(PrivateDocumentEnvelopeError::InvalidEntryLength(
                entry.len(),
            ));
        }
        let (commitment, rest) = entry.split_at(COMMITMENT_LEN);
        let (epk, rest) = rest.split_at(EPHEMERAL_KEY_LEN);
        let (body_ct, out_ct) = rest.split_at(rest.len() - SENDER_RECOVERY_LEN);
        Ok(Self {
            commitment: commitment.try_into().expect("32 bytes"),
            epk: epk.try_into().expect("33 bytes"),
            body_ct,
            out_ct,
        })
    }
}

/// Reads a decrypted body, refusing one no sender of this version writes.
fn parse_body(
    body: &[u8],
    target: &PrivateDocumentTarget,
    commitment: [u8; 32],
    recipient_public_key: PublicKey,
    recipient_key_index: Option<usize>,
) -> Result<OpenedPrivateDocumentEntry, PrivateDocumentEnvelopeError> {
    let malformed = PrivateDocumentEnvelopeError::Malformed;
    if body.len() < PRIVATE_DOCUMENT_ENVELOPE_BODY_HEADER_LEN {
        return Err(malformed("body shorter than its header"));
    }
    if body[0] != PRIVATE_DOCUMENT_ENVELOPE_VERSION {
        return Err(PrivateDocumentEnvelopeError::UnsupportedVersion(body[0]));
    }
    let auth_mode =
        PrivateDocumentAuthMode::from_byte(body[1]).ok_or(malformed("unknown auth mode"))?;
    let sender_identity_id: [u8; 32] = body[2..34].try_into().expect("32 bytes");
    let sender_key_id = u32::from_le_bytes(body[34..38].try_into().expect("4 bytes"));
    let auth: [u8; AUTH_LEN] = body[38..103].try_into().expect("65 bytes");
    let recipient_identity_id: [u8; 32] = body[103..135].try_into().expect("32 bytes");
    let payload_len = u16::from_le_bytes(body[135..137].try_into().expect("2 bytes")) as usize;
    let rest = &body[PRIVATE_DOCUMENT_ENVELOPE_BODY_HEADER_LEN..];
    if payload_len > rest.len() {
        return Err(malformed("payload longer than the body"));
    }
    let (payload, padding) = rest.split_at(payload_len);
    if padding.iter().any(|byte| *byte != 0) {
        return Err(malformed("padding is not zeros"));
    }
    let well_formed_auth = match auth_mode {
        PrivateDocumentAuthMode::Anonymous => {
            sender_identity_id == [0; 32] && sender_key_id == 0 && auth == [0; AUTH_LEN]
        }
        PrivateDocumentAuthMode::DeniableMac => auth[MAC_LEN..].iter().all(|byte| *byte == 0),
        PrivateDocumentAuthMode::Signature => {
            (COMPACT_SIGNATURE_HEADER_BASE..COMPACT_SIGNATURE_HEADER_BASE + 4).contains(&auth[0])
        }
    };
    if !well_formed_auth {
        return Err(malformed("sender fields do not match the auth mode"));
    }
    let auth_message = auth_message(
        target,
        auth_mode,
        &sender_identity_id,
        sender_key_id,
        &recipient_identity_id,
        payload,
    );
    Ok(OpenedPrivateDocumentEntry {
        commitment,
        recipient_key_index,
        recipient_public_key,
        recipient_identity_id,
        auth_mode,
        sender_identity_id,
        sender_key_id,
        payload: payload.to_vec(),
        auth,
        auth_message,
    })
}

fn hash(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn type_name_length(target: &PrivateDocumentTarget) -> [u8; 4] {
    (target.document_type_name.len() as u32).to_le_bytes()
}

fn ephemeral_secret_key(esk_seed: &[u8; 32]) -> Result<SecretKey, PrivateDocumentEnvelopeError> {
    SecretKey::from_secret_bytes(hash(&[DOMAIN_ESK, esk_seed]))
        .map_err(|_| PrivateDocumentEnvelopeError::InvalidEphemeralSeed)
}

/// `secret_key * public_key`, compressed, computed in constant time: the secret keys are a
/// wallet's long-lived keys and the points are an attacker's, so the multiplication must not leak
/// the key through timing. libsecp256k1's ECDH multiplies in constant time; it hands back the
/// point's coordinates, compressed here as `0x02 | (y & 1)` then `x`.
fn shared_point(secret_key: &SecretKey, public_key: &PublicKey) -> [u8; 33] {
    let xy = shared_secret_point(public_key, secret_key);
    let mut point = [0u8; 33];
    point[0] = 0x02 | (xy[63] & 1);
    point[1..].copy_from_slice(&xy[..32]);
    point
}

fn entry_key(shared_point: &[u8; 33], epk: &[u8; 33], recipient_key: &[u8; 33]) -> [u8; 32] {
    hash(&[DOMAIN_KEY, shared_point, epk, recipient_key])
}

fn commitment(
    key: &[u8; 32],
    target: &PrivateDocumentTarget,
    recipient_key: &[u8; 33],
    body: &[u8],
) -> [u8; 32] {
    hash(&[
        DOMAIN_CM,
        key,
        &target.contract_id,
        &type_name_length(target),
        target.document_type_name.as_bytes(),
        recipient_key,
        body,
    ])
}

fn outgoing_cipher_key(outgoing_viewing_key: &[u8; 32], epk: &[u8; 33]) -> [u8; 32] {
    hash(&[DOMAIN_OCK, outgoing_viewing_key, epk])
}

fn mac_key(shared_point: &[u8; 33]) -> [u8; 32] {
    hash(&[DOMAIN_MAC, shared_point])
}

fn auth_message(
    target: &PrivateDocumentTarget,
    auth_mode: PrivateDocumentAuthMode,
    sender_identity_id: &[u8; 32],
    sender_key_id: u32,
    recipient_identity_id: &[u8; 32],
    payload: &[u8],
) -> [u8; 32] {
    hash(&[
        DOMAIN_AUTH,
        &[PRIVATE_DOCUMENT_ENVELOPE_VERSION, auth_mode as u8],
        &target.contract_id,
        &type_name_length(target),
        target.document_type_name.as_bytes(),
        sender_identity_id,
        &sender_key_id.to_le_bytes(),
        recipient_identity_id,
        &(payload.len() as u16).to_le_bytes(),
        payload,
    ])
}

fn encrypt(
    key: &[u8; 32],
    aad: &[u8; 32],
    plaintext: &[u8],
) -> Result<Vec<u8>, PrivateDocumentEnvelopeError> {
    ChaCha20Poly1305::new(Key::from_slice(key))
        .encrypt(
            Nonce::from_slice(&[0; 12]),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| PrivateDocumentEnvelopeError::Malformed("encryption failed"))
}

fn decrypt(
    key: &[u8; 32],
    aad: &[u8; 32],
    ciphertext: &[u8],
) -> Result<Vec<u8>, PrivateDocumentEnvelopeError> {
    ChaCha20Poly1305::new(Key::from_slice(key))
        .decrypt(
            Nonce::from_slice(&[0; 12]),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| PrivateDocumentEnvelopeError::DecryptionFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DashPay's `privateContactRequest`: 589-byte entries, whose largest payload is 290 bytes.
    const ENTRY_SIZE: usize = 589;
    const LARGEST_PAYLOAD: usize = 290;
    const SENDER_IDENTITY: [u8; 32] = [0xA0; 32];
    const RECIPIENT_IDENTITY: [u8; 32] = [0xB0; 32];

    fn target() -> PrivateDocumentTarget<'static> {
        PrivateDocumentTarget {
            contract_id: [7; 32],
            document_type_name: "privateContactRequest",
        }
    }

    fn secret_key(byte: u8) -> SecretKey {
        SecretKey::from_secret_bytes([byte; 32]).expect("a valid secret key")
    }

    fn public_key(byte: u8) -> PublicKey {
        PublicKey::from_secret_key(&secret_key(byte))
    }

    fn payload() -> Vec<u8> {
        (0..LARGEST_PAYLOAD).map(|i| i as u8).collect()
    }

    fn outgoing_viewing_key() -> [u8; 32] {
        private_document_outgoing_viewing_key(&[0x0F; 32])
    }

    /// An entry to the recipient key `secret_key(2)` from `sender`, built from `esk_seed`.
    fn seal(sender: &PrivateDocumentSender, esk_seed: u8) -> Vec<u8> {
        seal_private_document_entry(
            &target(),
            ENTRY_SIZE,
            &PrivateDocumentRecipient {
                identity_id: RECIPIENT_IDENTITY,
                public_key: &public_key(2),
            },
            sender,
            &payload(),
            &outgoing_viewing_key(),
            &[esk_seed; 32],
        )
        .expect("the payload fits")
    }

    fn open(entry: &[u8], keys: &[SecretKey]) -> Option<OpenedPrivateDocumentEntry> {
        open_private_document_entry(entry, &target(), &RECIPIENT_IDENTITY, keys)
            .expect("the entry is well formed")
    }

    fn deniable_mac_sender(secret_key: &SecretKey) -> PrivateDocumentSender<'_> {
        PrivateDocumentSender::DeniableMac {
            identity_id: SENDER_IDENTITY,
            key_id: 4,
            secret_key,
        }
    }

    /// The recipient opens an entry with whichever of its keys it is encrypted to, reads the
    /// payload and the sender it claims, and checks the deniable MAC against the sender's key.
    #[test]
    fn should_open_an_entry_with_its_key_and_check_a_deniable_mac() {
        let sender_key = secret_key(1);
        let entry = seal(&deniable_mac_sender(&sender_key), 9);
        assert_eq!(entry.len(), ENTRY_SIZE);

        let opened =
            open(&entry, &[secret_key(3), secret_key(2)]).expect("the second key opens it");
        assert_eq!(opened.recipient_key_index(), Some(1));
        assert_eq!(opened.recipient_public_key(), &public_key(2));
        assert_eq!(opened.recipient_identity_id(), &RECIPIENT_IDENTITY);
        assert_eq!(opened.auth_mode(), PrivateDocumentAuthMode::DeniableMac);
        assert_eq!(opened.sender_identity_id(), &SENDER_IDENTITY);
        assert_eq!(opened.sender_key_id(), 4);
        assert_eq!(opened.payload(), payload().as_slice());

        opened
            .verify_sender(&public_key(1), &secret_key(2))
            .expect("the sender's key wrote the MAC");
        assert!(matches!(
            opened.verify_sender(&public_key(5), &secret_key(2)),
            Err(PrivateDocumentEnvelopeError::SenderNotAuthenticated)
        ));
        assert!(matches!(
            opened.verify_sender(&public_key(1), &secret_key(3)),
            Err(PrivateDocumentEnvelopeError::SenderNotAuthenticated)
        ));
    }

    /// A signed entry is checked against the sender's key alone.
    #[test]
    fn should_check_a_signature_against_the_sender_key() {
        let sender_key = secret_key(1);
        let entry = seal(
            &PrivateDocumentSender::Signature {
                identity_id: SENDER_IDENTITY,
                key_id: 4,
                secret_key: &sender_key,
            },
            9,
        );
        let opened = open(&entry, &[secret_key(2)]).expect("it opens");
        assert_eq!(opened.auth_mode(), PrivateDocumentAuthMode::Signature);
        opened
            .verify_sender(&public_key(1), &secret_key(2))
            .expect("the sender's key signed it");
        assert!(matches!(
            opened.verify_sender(&public_key(5), &secret_key(2)),
            Err(PrivateDocumentEnvelopeError::SenderNotAuthenticated)
        ));
    }

    /// An anonymous entry names no sender, and there is nothing to check.
    #[test]
    fn should_open_an_anonymous_entry_without_a_sender() {
        let entry = seal(&PrivateDocumentSender::Anonymous, 9);
        let opened = open(&entry, &[secret_key(2)]).expect("it opens");
        assert_eq!(opened.auth_mode(), PrivateDocumentAuthMode::Anonymous);
        assert_eq!(opened.sender_identity_id(), &[0; 32]);
        assert_eq!(opened.sender_key_id(), 0);
        assert_eq!(opened.payload(), payload().as_slice());
        assert!(matches!(
            opened.verify_sender(&public_key(1), &secret_key(2)),
            Err(PrivateDocumentEnvelopeError::AnonymousSender)
        ));
    }

    /// Nearly every entry a wallet scans is for someone else: it is passed over, not refused.
    #[test]
    fn should_pass_over_an_entry_for_other_keys() {
        let sender_key = secret_key(1);
        let entry = seal(&deniable_mac_sender(&sender_key), 9);
        assert_eq!(open(&entry, &[secret_key(3), secret_key(1)]), None);
        assert_eq!(open(&entry, &[]), None);
    }

    /// A byte changed in the commitment, the ephemeral key or the body, or the entry read as one
    /// of another contract or type, opens for nobody.
    #[test]
    fn should_not_open_an_entry_changed_or_moved_to_another_type() {
        let sender_key = secret_key(1);
        let entry = seal(&deniable_mac_sender(&sender_key), 9);
        for position in [0, 31, 33, 64, 65, 300, ENTRY_SIZE - 82] {
            let mut changed = entry.clone();
            changed[position] ^= 1;
            assert_eq!(open(&changed, &[secret_key(2)]), None, "byte {position}");
        }
        for moved in [
            PrivateDocumentTarget {
                contract_id: [8; 32],
                ..target()
            },
            PrivateDocumentTarget {
                document_type_name: "contactRequest",
                ..target()
            },
        ] {
            assert_eq!(
                open_private_document_entry(&entry, &moved, &RECIPIENT_IDENTITY, &[secret_key(2)])
                    .expect("well formed"),
                None
            );
        }
    }

    /// An entry that opens under the identity's key but names another identity is a replay,
    /// and is refused.
    #[test]
    fn should_refuse_an_entry_naming_another_recipient_identity() {
        let sender_key = secret_key(1);
        let entry = seal(&deniable_mac_sender(&sender_key), 9);
        assert!(matches!(
            open_private_document_entry(&entry, &target(), &[0xC0; 32], &[secret_key(2)]),
            Err(PrivateDocumentEnvelopeError::WrongRecipient)
        ));
    }

    /// The sender recovers its own entry, recipient and payload included, from its outgoing
    /// viewing key alone; another sender's key recovers nothing, and nor does a changed
    /// sender-recovery ciphertext.
    #[test]
    fn should_recover_a_sent_entry_with_the_outgoing_viewing_key() {
        let sender_key = secret_key(1);
        let entry = seal(&deniable_mac_sender(&sender_key), 9);
        let recovered =
            recover_sent_private_document_entry(&entry, &target(), &outgoing_viewing_key())
                .expect("well formed")
                .expect("the sender built it");
        assert_eq!(recovered.recipient_key_index(), None);
        assert_eq!(recovered.recipient_public_key(), &public_key(2));
        assert_eq!(recovered.payload(), payload().as_slice());
        let opened = open(&entry, &[secret_key(2)]).expect("it opens");
        assert_eq!(
            OpenedPrivateDocumentEntry {
                recipient_key_index: Some(0),
                ..recovered.clone()
            },
            opened,
            "the sender recovers what the recipient opens"
        );

        let other_sender = private_document_outgoing_viewing_key(&[0x1F; 32]);
        assert_eq!(
            recover_sent_private_document_entry(&entry, &target(), &other_sender)
                .expect("well formed"),
            None
        );
        let mut changed = entry.clone();
        changed[ENTRY_SIZE - 1] ^= 1;
        assert_eq!(
            recover_sent_private_document_entry(&changed, &target(), &outgoing_viewing_key())
                .expect("well formed"),
            None
        );
    }

    /// An entry holds a payload of at most its size minus 299 bytes: a larger payload is refused,
    /// an entry smaller than the envelope is refused for its size, and the smallest entry, 299
    /// bytes, carries an empty payload.
    #[test]
    fn should_refuse_a_payload_the_entry_cannot_hold() {
        assert_eq!(PRIVATE_DOCUMENT_ENVELOPE_WRAPPER_LEN, 162);
        assert_eq!(PRIVATE_DOCUMENT_ENVELOPE_BODY_HEADER_LEN, 137);
        assert_eq!(PRIVATE_DOCUMENT_ENVELOPE_OVERHEAD_LEN, 299);
        let seal_into = |entry_size: usize, payload: &[u8]| {
            seal_private_document_entry(
                &target(),
                entry_size,
                &PrivateDocumentRecipient {
                    identity_id: RECIPIENT_IDENTITY,
                    public_key: &public_key(2),
                },
                &PrivateDocumentSender::Anonymous,
                payload,
                &outgoing_viewing_key(),
                &[9; 32],
            )
        };
        assert_eq!(
            seal_into(ENTRY_SIZE, &vec![1; LARGEST_PAYLOAD + 1]),
            Err(PrivateDocumentEnvelopeError::PayloadTooLarge {
                payload_len: LARGEST_PAYLOAD + 1,
                capacity: LARGEST_PAYLOAD,
            })
        );
        assert_eq!(
            seal_into(298, &[]),
            Err(PrivateDocumentEnvelopeError::InvalidEntryLength(298))
        );
        let smallest = seal_into(299, &[]).expect("an empty payload fills the smallest entry");
        assert_eq!(smallest.len(), 299);
        assert_eq!(
            open(&smallest, &[secret_key(2)])
                .expect("it opens")
                .payload(),
            &[] as &[u8]
        );
        assert_eq!(
            open_private_document_entry(&[0; 298], &target(), &RECIPIENT_IDENTITY, &[]),
            Err(PrivateDocumentEnvelopeError::InvalidEntryLength(298))
        );
    }

    /// A body no sender of this version writes is refused once it decrypts: another version,
    /// an unknown auth mode, a payload longer than the body, padding that is not zeros, and
    /// sender fields an anonymous entry does not carry.
    #[test]
    fn should_refuse_a_body_no_sender_writes() {
        let well_formed = || {
            let mut body = vec![0u8; ENTRY_SIZE - PRIVATE_DOCUMENT_ENVELOPE_WRAPPER_LEN];
            body[0] = PRIVATE_DOCUMENT_ENVELOPE_VERSION;
            body[135..137].copy_from_slice(&3u16.to_le_bytes());
            body
        };
        let parse = |body: &[u8]| parse_body(body, &target(), [0; 32], public_key(2), Some(0));
        parse(&well_formed()).expect("an anonymous body of 3 zero bytes");

        let mut other_version = well_formed();
        other_version[0] = 2;
        assert!(matches!(
            parse(&other_version),
            Err(PrivateDocumentEnvelopeError::UnsupportedVersion(2))
        ));
        for (position, value) in [
            (1, 3u8),              // auth mode
            (136, 2),              // a payload length over the body
            (ENTRY_SIZE - 163, 1), // the last padding byte
            (2, 1),                // a sender identity on an anonymous entry
            (40, 1),               // an auth on an anonymous entry
        ] {
            let mut body = well_formed();
            body[position] = value;
            assert!(
                matches!(
                    parse(&body),
                    Err(PrivateDocumentEnvelopeError::Malformed(_))
                ),
                "byte {position}"
            );
        }
    }

    /// The same inputs build the same entry, byte for byte, in every mode; another seed builds
    /// an unlinkable one. The hashes pin version 1 of the format.
    #[test]
    fn should_build_the_same_entry_from_the_same_inputs() {
        let sender_key = secret_key(1);
        let senders = [
            PrivateDocumentSender::Anonymous,
            deniable_mac_sender(&sender_key),
            PrivateDocumentSender::Signature {
                identity_id: SENDER_IDENTITY,
                key_id: 4,
                secret_key: &sender_key,
            },
        ];
        let pinned: Vec<String> = senders
            .iter()
            .map(|sender| {
                let entry = seal(sender, 9);
                assert_eq!(entry, seal(sender, 9));
                let other = seal(sender, 10);
                assert_ne!(entry[..COMMITMENT_LEN], other[..COMMITMENT_LEN]);
                assert_ne!(entry[COMMITMENT_LEN..65], other[COMMITMENT_LEN..65]);
                hex(&hash(&[&entry]))
            })
            .collect();
        assert_eq!(
            pinned,
            [
                "abb644128f3e73aade8b6210573fba942709e7e44eaeffef92dc02d5187f37d3",
                "bedf5084aca776ae520e1395e4a4f92628f4589f04b244630acc890649f74a82",
                "a1054055987e953aeeb76deb9998db79eebe3e798a7296eadeb90575bc6e0e56",
            ]
        );
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
