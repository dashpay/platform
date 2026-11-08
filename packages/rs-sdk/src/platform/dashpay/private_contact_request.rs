//! DashPay private contact requests (`privateContactRequest`, DashPay v3, protocol version 15).
//!
//! A private contact request is an entry of the `privateContactRequest` store sealed to one of
//! the recipient's DashPay keys: only the recipient learns that it exists, who sent it and what
//! it holds. The sender pays for it from the shielded pool, so no identity appears on chain.
//! Inside the entry the sender names its identity and DashPay encryption key, with a deniable
//! MAC: the recipient learns who wrote it, but cannot prove that to anyone else.
//!
//! - Sending: [`seal_private_contact_request`] (or [`Sdk::seal_private_contact_request`]) builds
//!   the entry, and a wallet appends it, paying from its notes, with
//!   [`PrivateDocumentCreateFromShieldedPool`](crate::platform::transition::private_document_create_from_shielded_pool::PrivateDocumentCreateFromShieldedPool)
//!   or the dpp builder.
//! - Receiving: [`Sdk::scan_private_contact_requests`] reads a page of the store, opens the
//!   wallet's entries, checks each sender against its identity's key and reads the request.
//!
//! Each identity seals with the outgoing viewing key of its DashPay encryption key
//! ([`private_contact_request_outgoing_viewing_key`]), so a wallet restored from seed recovers
//! the requests it sent from the store alone.

use super::contact_request::{recipient_key_purpose_is_valid, sender_key_purpose_is_valid};
use crate::platform::private_document_scan::{
    PrivateDocumentScan, PrivateDocumentScanKeys, ReceivedPrivateDocumentEntry,
};
use crate::platform::{Fetch, Identifier};
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::{
    decode_private_document_payload, encode_private_document_payload, DocumentTypeRef,
};
use dpp::data_contract::DataContract;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType};
use dpp::platform_value::{BinaryData, Value};
use dpp::secp256k1::rand::rngs::StdRng;
use dpp::secp256k1::rand::{RngCore, SeedableRng};
use dpp::secp256k1::{PublicKey, SecretKey};
use dpp::version::PlatformVersion;
use platform_encryption::{
    private_document_outgoing_viewing_key, seal_private_document_entry,
    PrivateDocumentEnvelopeError, PrivateDocumentRecipient, PrivateDocumentSender,
    PrivateDocumentTarget, COMPACT_XPUB_LEN,
};
use std::collections::{BTreeMap, BTreeSet};
use zeroize::Zeroize;

/// The DashPay document type of private contact requests.
pub const PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME: &str = "privateContactRequest";

/// The most UTF-8 bytes of `accountLabel` (`maxBytes`).
pub const PRIVATE_CONTACT_REQUEST_ACCOUNT_LABEL_MAX_BYTES: usize = 63;
/// The fewest bytes of `autoAcceptProof` (`minItems`).
pub const PRIVATE_CONTACT_REQUEST_AUTO_ACCEPT_PROOF_MIN_LEN: usize = 38;
/// The most bytes of `autoAcceptProof` (`maxItems`).
pub const PRIVATE_CONTACT_REQUEST_AUTO_ACCEPT_PROOF_MAX_LEN: usize = 102;
/// The bytes of `shieldedAddress`: an Orchard diversifier (11) and transmission key (32).
pub const PRIVATE_CONTACT_REQUEST_SHIELDED_ADDRESS_LEN: usize = 43;

const ACCOUNT_REFERENCE: &str = "accountReference";
const EXTENDED_PUBLIC_KEY: &str = "extendedPublicKey";
const ACCOUNT_LABEL: &str = "accountLabel";
const AUTO_ACCEPT_PROOF: &str = "autoAcceptProof";
const SHIELDED_ADDRESS: &str = "shieldedAddress";

/// What a private contact request carries, in the clear inside its entry. Unlike a public
/// `contactRequest`, nothing in it is encrypted again: the entry already is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivateContactRequest {
    /// The DIP-15 account reference of the sender's account the recipient pays to.
    pub account_reference: u32,
    /// The DIP-15 compact extended public key the recipient pays to: the parent fingerprint
    /// (4 bytes), the chain code (32) and the public key (33).
    pub extended_public_key: [u8; COMPACT_XPUB_LEN],
    /// A label for the account, at most 63 UTF-8 bytes.
    pub account_label: Option<String>,
    /// A DIP-15 auto-accept proof, 38 to 102 bytes.
    pub auto_accept_proof: Option<Vec<u8>>,
    /// The Orchard address the recipient pays this contact to privately.
    pub shielded_address: Option<[u8; PRIVATE_CONTACT_REQUEST_SHIELDED_ADDRESS_LEN]>,
}

impl PrivateContactRequest {
    /// The request as the type's properties, refused when a value is out of the schema's
    /// bounds. dpp's schema validation is not part of the SDK's features, so the bounds the
    /// fields' types do not hold are checked here.
    pub fn to_properties(&self) -> Result<BTreeMap<String, Value>, Error> {
        self.check_bounds()?;
        let mut properties = BTreeMap::from([
            (
                ACCOUNT_REFERENCE.to_string(),
                Value::U32(self.account_reference),
            ),
            (
                EXTENDED_PUBLIC_KEY.to_string(),
                Value::Bytes(self.extended_public_key.to_vec()),
            ),
        ]);
        if let Some(label) = &self.account_label {
            properties.insert(ACCOUNT_LABEL.to_string(), Value::Text(label.clone()));
        }
        if let Some(proof) = &self.auto_accept_proof {
            properties.insert(AUTO_ACCEPT_PROOF.to_string(), Value::Bytes(proof.clone()));
        }
        if let Some(address) = &self.shielded_address {
            properties.insert(SHIELDED_ADDRESS.to_string(), Value::Bytes(address.to_vec()));
        }
        Ok(properties)
    }

    /// The request a private contact request's properties hold, refused when one is missing,
    /// unknown, of the wrong type or out of the schema's bounds.
    pub fn from_properties(mut properties: BTreeMap<String, Value>) -> Result<Self, Error> {
        let invalid =
            |message: String| Error::Generic(format!("private contact request: {message}"));
        // An absent optional property may read back as null.
        properties.retain(|_, value| !value.is_null());
        let mut take_bytes = |name: &str| -> Result<Option<Vec<u8>>, Error> {
            properties
                .remove(name)
                .map(|value| {
                    value
                        .into_binary_bytes()
                        .map_err(|error| invalid(format!("{name}: {error}")))
                })
                .transpose()
        };
        let extended_public_key = take_bytes(EXTENDED_PUBLIC_KEY)?
            .ok_or_else(|| invalid(format!("{EXTENDED_PUBLIC_KEY} is missing")))?;
        let auto_accept_proof = take_bytes(AUTO_ACCEPT_PROOF)?;
        let shielded_address = take_bytes(SHIELDED_ADDRESS)?;
        let account_reference = properties
            .remove(ACCOUNT_REFERENCE)
            .ok_or_else(|| invalid(format!("{ACCOUNT_REFERENCE} is missing")))?
            .to_integer::<u32>()
            .map_err(|error| invalid(format!("{ACCOUNT_REFERENCE}: {error}")))?;
        let account_label = properties
            .remove(ACCOUNT_LABEL)
            .map(|value| {
                value
                    .into_text()
                    .map_err(|error| invalid(format!("{ACCOUNT_LABEL}: {error}")))
            })
            .transpose()?;
        if let Some(name) = properties.keys().next() {
            return Err(invalid(format!("unknown property {name}")));
        }

        let request = Self {
            account_reference,
            extended_public_key: extended_public_key.as_slice().try_into().map_err(|_| {
                invalid(format!(
                    "{EXTENDED_PUBLIC_KEY} is {} bytes, not {COMPACT_XPUB_LEN}",
                    extended_public_key.len()
                ))
            })?,
            account_label,
            auto_accept_proof,
            shielded_address: shielded_address
                .map(|address| {
                    address.as_slice().try_into().map_err(|_| {
                        invalid(format!(
                            "{SHIELDED_ADDRESS} is {} bytes, not \
                             {PRIVATE_CONTACT_REQUEST_SHIELDED_ADDRESS_LEN}",
                            address.len()
                        ))
                    })
                })
                .transpose()?,
        };
        request.check_bounds()?;
        Ok(request)
    }

    fn check_bounds(&self) -> Result<(), Error> {
        if let Some(label) = &self.account_label {
            if label.len() > PRIVATE_CONTACT_REQUEST_ACCOUNT_LABEL_MAX_BYTES {
                return Err(Error::Generic(format!(
                    "private contact request: {ACCOUNT_LABEL} is {} bytes, more than \
                     {PRIVATE_CONTACT_REQUEST_ACCOUNT_LABEL_MAX_BYTES}",
                    label.len()
                )));
            }
        }
        if let Some(proof) = &self.auto_accept_proof {
            if !(PRIVATE_CONTACT_REQUEST_AUTO_ACCEPT_PROOF_MIN_LEN
                ..=PRIVATE_CONTACT_REQUEST_AUTO_ACCEPT_PROOF_MAX_LEN)
                .contains(&proof.len())
            {
                return Err(Error::Generic(format!(
                    "private contact request: {AUTO_ACCEPT_PROOF} is {} bytes, not \
                     {PRIVATE_CONTACT_REQUEST_AUTO_ACCEPT_PROOF_MIN_LEN} to \
                     {PRIVATE_CONTACT_REQUEST_AUTO_ACCEPT_PROOF_MAX_LEN}",
                    proof.len()
                )));
            }
        }
        Ok(())
    }
}

/// The outgoing viewing key an identity seals its private contact requests with, derived from
/// the secret key of its DashPay encryption key: with it a wallet recovers the requests the
/// identity sent from the store alone, as after a restore from seed. An identity that changed
/// its encryption key scans with the key of each.
pub fn private_contact_request_outgoing_viewing_key(encryption_secret_key: &SecretKey) -> [u8; 32] {
    let mut secret = encryption_secret_key.to_secret_bytes();
    let outgoing_viewing_key = private_document_outgoing_viewing_key(&secret);
    secret.zeroize();
    outgoing_viewing_key
}

/// The sender of a private contact request: its identity, and the id and secret key of the
/// DashPay encryption key it names in the entry.
pub struct PrivateContactRequestSender<'a> {
    /// The sending identity.
    pub identity: &'a Identity,
    /// The id of its enabled `ECDSA_SECP256K1` `ENCRYPTION` key.
    pub encryption_key_id: KeyID,
    /// That key's secret key.
    pub encryption_secret_key: &'a SecretKey,
}

/// Seals `request` from `sender` to the key `recipient_key_id` of `recipient`: an entry of the
/// `privateContactRequest` store of `dashpay_contract`, of the type's entry size. The sender
/// key must be the identity's enabled `ECDSA_SECP256K1` `ENCRYPTION` key whose secret key is
/// given; the recipient key an enabled `ECDSA_SECP256K1` `DECRYPTION` or `ENCRYPTION` key.
///
/// `esk_seed` must be 32 fresh random bytes for every entry, never reused, not even on a retry
/// ([`Sdk::seal_private_contact_request`] draws one).
pub fn seal_private_contact_request(
    dashpay_contract: &DataContract,
    sender: &PrivateContactRequestSender,
    recipient: &Identity,
    recipient_key_id: KeyID,
    request: &PrivateContactRequest,
    esk_seed: &[u8; 32],
) -> Result<BinaryData, Error> {
    let document_type = private_contact_request_type(dashpay_contract)?;
    let entry_size = document_type
        .private_document()
        .map(|config| config.entry_size())
        .ok_or_else(|| {
            Error::Generic(format!(
                "DashPay {PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME} is not a private type"
            ))
        })?;

    let sender_key = sender
        .identity
        .get_public_key_by_id(sender.encryption_key_id)
        .ok_or_else(|| {
            Error::Generic(format!(
                "the sender identity {} has no key {}",
                sender.identity.id(),
                sender.encryption_key_id
            ))
        })?;
    if !sender_key_purpose_is_valid(sender_key.purpose()) {
        return Err(Error::Generic(format!(
            "the sender key {} is a {} key, not an ENCRYPTION key",
            sender.encryption_key_id,
            sender_key.purpose()
        )));
    }
    if ecdh_public_key(sender_key, "sender")?
        != PublicKey::from_secret_key(sender.encryption_secret_key)
    {
        return Err(Error::Generic(format!(
            "the secret key given is not the one of the sender key {}",
            sender.encryption_key_id
        )));
    }

    let recipient_key = recipient
        .get_public_key_by_id(recipient_key_id)
        .ok_or_else(|| {
            Error::Generic(format!(
                "the recipient identity {} has no key {recipient_key_id}",
                recipient.id()
            ))
        })?;
    if !recipient_key_purpose_is_valid(recipient_key.purpose()) {
        return Err(Error::Generic(format!(
            "the recipient key {recipient_key_id} is a {} key, not a DECRYPTION or \
             ENCRYPTION key",
            recipient_key.purpose()
        )));
    }
    let recipient_public_key = ecdh_public_key(recipient_key, "recipient")?;

    let payload = encode_private_document_payload(document_type, request.to_properties()?)?;
    let outgoing_viewing_key =
        private_contact_request_outgoing_viewing_key(sender.encryption_secret_key);
    let entry = seal_private_document_entry(
        &PrivateDocumentTarget {
            contract_id: dashpay_contract.id().to_buffer(),
            document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
        },
        entry_size as usize,
        &PrivateDocumentRecipient {
            identity_id: recipient.id().to_buffer(),
            public_key: &recipient_public_key,
        },
        &PrivateDocumentSender::DeniableMac {
            identity_id: sender.identity.id().to_buffer(),
            key_id: sender.encryption_key_id,
            secret_key: sender.encryption_secret_key,
        },
        &payload,
        &outgoing_viewing_key,
        esk_seed,
    )
    .map_err(|error| Error::Generic(format!("sealing a private contact request: {error}")))?;
    Ok(BinaryData::new(entry))
}

/// The public key of an enabled `ECDSA_SECP256K1` identity key, which ECDH runs on.
fn ecdh_public_key(key: &IdentityPublicKey, role: &str) -> Result<PublicKey, Error> {
    if key.key_type() != KeyType::ECDSA_SECP256K1 {
        return Err(Error::Generic(format!(
            "the {role} key {} is a {} key, not ECDSA_SECP256K1",
            key.id(),
            key.key_type()
        )));
    }
    if key.disabled_at().is_some() {
        return Err(Error::Generic(format!(
            "the {role} key {} is disabled",
            key.id()
        )));
    }
    PublicKey::from_slice(key.data().as_slice()).map_err(|error| {
        Error::Generic(format!(
            "the {role} key {} is not a secp256k1 point: {error}",
            key.id()
        ))
    })
}

fn private_contact_request_type(
    dashpay_contract: &DataContract,
) -> Result<DocumentTypeRef<'_>, Error> {
    dashpay_contract
        .document_type_for_name(PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME)
        .map_err(|_| {
            Error::Generic(format!(
                "the DashPay contract has no {PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME} type: \
                 it is declared from DashPay v3, protocol version 15"
            ))
        })
}

/// A private contact request to one of the wallet's identities, from a sender whose key was
/// checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceivedPrivateContactRequest {
    /// The entry's position in the store.
    pub position: u64,
    /// The height of the block that appended it.
    pub block_height: u64,
    /// The time of the block that appended it, in milliseconds.
    pub block_time_ms: u64,
    /// The wallet's identity it is to.
    pub recipient_id: Identifier,
    /// The key of that identity it is encrypted to.
    pub recipient_key_id: KeyID,
    /// The identity that sent it.
    pub sender_id: Identifier,
    /// The sender's encryption key it names, which wrote its MAC.
    pub sender_key_id: KeyID,
    /// The entry's commitment: a copy of the entry appended again has the same one.
    pub commitment: [u8; 32],
    /// The request.
    pub request: PrivateContactRequest,
}

/// A private contact request one of the wallet's identities sent, recovered with its outgoing
/// viewing key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentPrivateContactRequest {
    /// The entry's position in the store.
    pub position: u64,
    /// The height of the block that appended it.
    pub block_height: u64,
    /// The time of the block that appended it, in milliseconds.
    pub block_time_ms: u64,
    /// The wallet's identity that sent it.
    pub sender_id: Identifier,
    /// The identity it is to.
    pub recipient_id: Identifier,
    /// The recipient key it is encrypted to.
    pub recipient_public_key: PublicKey,
    /// The entry's commitment.
    pub commitment: [u8; 32],
    /// The request.
    pub request: PrivateContactRequest,
}

/// Why an entry that opened for one of the wallet's identities is not taken as a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrivateContactRequestRejection {
    /// The entry itself is refused: malformed, of an unknown envelope version, or naming
    /// another identity than its key's.
    Envelope(PrivateDocumentEnvelopeError),
    /// It names no sender, and a contact request is from someone.
    AnonymousSender,
    /// The identity it names as its sender does not exist.
    SenderNotFound,
    /// The sender key it names is missing, not an `ECDSA_SECP256K1` `ENCRYPTION` key, or was
    /// disabled before the entry was appended.
    SenderKeyNotUsable(String),
    /// The sender key it names did not write it.
    SenderNotAuthenticated,
    /// Its payload is not a valid private contact request. One written after DashPay gained a
    /// property the contract the SDK holds lacks reads as invalid until the contract is
    /// refetched, so a wallet keeps the position to read again.
    InvalidRequest(String),
    /// An earlier entry of the scan holds a request from the same sender to the same
    /// recipient for the same account reference, which is kept.
    Duplicate,
}

/// An entry of the store refused as a private contact request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RejectedPrivateContactRequest {
    /// The entry's position in the store.
    pub position: u64,
    /// The height of the block that appended it.
    pub block_height: u64,
    /// Why it is refused.
    pub reason: PrivateContactRequestRejection,
}

/// What one page of the `privateContactRequest` store holds for the wallet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrivateContactRequestScan {
    /// The requests to the wallet's identities, in store order.
    pub received: Vec<ReceivedPrivateContactRequest>,
    /// The requests the wallet's identities sent, in store order.
    pub sent: Vec<SentPrivateContactRequest>,
    /// The entries refused, in store order.
    pub rejected: Vec<RejectedPrivateContactRequest>,
    /// The position to scan from next.
    pub next_position: u64,
    /// How many entries the store held when the page was read.
    pub total_count: u64,
}

impl PrivateContactRequestScan {
    /// Whether the scan reached the store's last entry.
    pub fn is_complete(&self) -> bool {
        self.next_position >= self.total_count
    }
}

/// The identity ids the received entries of `scan` name as their senders, which
/// [`private_contact_requests_from_scan`] checks them against.
pub fn private_contact_request_senders(scan: &PrivateDocumentScan) -> BTreeSet<Identifier> {
    scan.received
        .iter()
        .map(|received| Identifier::from(*received.entry.sender_identity_id()))
        .filter(|sender_id| *sender_id != Identifier::default())
        .collect()
}

/// Reads the private contact requests of a scan of DashPay's `privateContactRequest` store:
/// decodes each payload, checks each received entry's sender against the key its identity, as
/// fetched into `sender_identities` (`None` for one that does not exist), holds under the id
/// the entry names, and keeps the first request from a sender to a recipient for an account
/// reference. A wallet keeps the requests it already holds from earlier scans, and drops a
/// later one from the same sender for the same account reference as this does.
pub fn private_contact_requests_from_scan(
    dashpay_contract: &DataContract,
    scan: PrivateDocumentScan,
    keys: &PrivateDocumentScanKeys,
    sender_identities: &BTreeMap<Identifier, Option<Identity>>,
    platform_version: &PlatformVersion,
) -> Result<PrivateContactRequestScan, Error> {
    let document_type = private_contact_request_type(dashpay_contract)?;
    let decode = |payload: &[u8]| -> Result<PrivateContactRequest, PrivateContactRequestRejection> {
        let properties = decode_private_document_payload(document_type, payload, platform_version)
            .map_err(|error| PrivateContactRequestRejection::InvalidRequest(error.to_string()))?;
        PrivateContactRequest::from_properties(properties)
            .map_err(|error| PrivateContactRequestRejection::InvalidRequest(error.to_string()))
    };

    let mut result = PrivateContactRequestScan {
        next_position: scan.next_position,
        total_count: scan.total_count,
        ..Default::default()
    };
    let mut rejected: Vec<RejectedPrivateContactRequest> = scan
        .rejected
        .into_iter()
        .map(|rejected| RejectedPrivateContactRequest {
            position: rejected.position,
            block_height: rejected.block_height,
            reason: PrivateContactRequestRejection::Envelope(rejected.error),
        })
        .collect();

    let mut seen = BTreeSet::new();
    for received in scan.received {
        let reject = |reason| RejectedPrivateContactRequest {
            position: received.position,
            block_height: received.block_height,
            reason,
        };
        match read_received(&received, keys, sender_identities, &decode) {
            Ok(request) => {
                let sender_id = Identifier::from(*received.entry.sender_identity_id());
                if !seen.insert((
                    received.recipient_identity_id,
                    sender_id,
                    request.account_reference,
                )) {
                    rejected.push(reject(PrivateContactRequestRejection::Duplicate));
                    continue;
                }
                result.received.push(ReceivedPrivateContactRequest {
                    position: received.position,
                    block_height: received.block_height,
                    block_time_ms: received.block_time_ms,
                    recipient_id: received.recipient_identity_id,
                    recipient_key_id: received.recipient_key_id,
                    sender_id,
                    sender_key_id: received.entry.sender_key_id(),
                    commitment: *received.entry.commitment(),
                    request,
                });
            }
            Err(reason) => rejected.push(reject(reason)),
        }
    }

    for sent in scan.sent {
        match decode(sent.entry.payload()) {
            Ok(request) => result.sent.push(SentPrivateContactRequest {
                position: sent.position,
                block_height: sent.block_height,
                block_time_ms: sent.block_time_ms,
                sender_id: sent.sender_identity_id,
                recipient_id: Identifier::from(*sent.entry.recipient_identity_id()),
                recipient_public_key: *sent.entry.recipient_public_key(),
                commitment: *sent.entry.commitment(),
                request,
            }),
            Err(reason) => rejected.push(RejectedPrivateContactRequest {
                position: sent.position,
                block_height: sent.block_height,
                reason,
            }),
        }
    }

    rejected.sort_by_key(|rejected| rejected.position);
    result.rejected = rejected;
    Ok(result)
}

/// The request a received entry holds, once its sender is checked.
fn read_received(
    received: &ReceivedPrivateDocumentEntry,
    keys: &PrivateDocumentScanKeys,
    sender_identities: &BTreeMap<Identifier, Option<Identity>>,
    decode: &impl Fn(&[u8]) -> Result<PrivateContactRequest, PrivateContactRequestRejection>,
) -> Result<PrivateContactRequest, PrivateContactRequestRejection> {
    let sender_id = Identifier::from(*received.entry.sender_identity_id());
    if sender_id == Identifier::default() {
        return Err(PrivateContactRequestRejection::AnonymousSender);
    }
    let request = decode(received.entry.payload())?;
    let sender = sender_identities
        .get(&sender_id)
        .and_then(Option::as_ref)
        .ok_or(PrivateContactRequestRejection::SenderNotFound)?;
    let key_id = received.entry.sender_key_id();
    let unusable = |message: String| PrivateContactRequestRejection::SenderKeyNotUsable(message);
    let key = sender
        .get_public_key_by_id(key_id)
        .ok_or_else(|| unusable(format!("the sender has no key {key_id}")))?;
    if !sender_key_purpose_is_valid(key.purpose()) {
        return Err(unusable(format!(
            "the sender key {key_id} is a {} key, not an ENCRYPTION key",
            key.purpose()
        )));
    }
    if key.key_type() != KeyType::ECDSA_SECP256K1 {
        return Err(unusable(format!(
            "the sender key {key_id} is a {} key, not ECDSA_SECP256K1",
            key.key_type()
        )));
    }
    // A key disabled after the entry was appended still wrote it: keys are disabled, never
    // removed, and the entry is immutable.
    if let Some(disabled_at) = key.disabled_at() {
        if disabled_at <= received.block_time_ms {
            return Err(unusable(format!(
                "the sender key {key_id} was disabled at {disabled_at}, before the entry"
            )));
        }
    }
    let sender_public_key = PublicKey::from_slice(key.data().as_slice()).map_err(|error| {
        unusable(format!(
            "the sender key {key_id} is not a secp256k1 point: {error}"
        ))
    })?;
    keys.verify_sender(received, &sender_public_key)
        .map_err(|_| PrivateContactRequestRejection::SenderNotAuthenticated)?;
    Ok(request)
}

impl Sdk {
    /// Seals `request` from `sender` to the key `recipient_key_id` of `recipient`, as
    /// [`seal_private_contact_request`] does, with the DashPay contract and a fresh ephemeral
    /// seed. The entry is appended by a transition paid from the sender's notes.
    pub async fn seal_private_contact_request(
        &self,
        sender: &PrivateContactRequestSender<'_>,
        recipient: &Identity,
        recipient_key_id: KeyID,
        request: &PrivateContactRequest,
    ) -> Result<BinaryData, Error> {
        let dashpay_contract = self.fetch_dashpay_contract().await?;
        // A seed whose hash is no secret key, a chance of about 2^-128, fails the seal, and a
        // retry draws another.
        let mut esk_seed = [0u8; 32];
        StdRng::from_os_rng().fill_bytes(&mut esk_seed);
        let sealed = seal_private_contact_request(
            &dashpay_contract,
            sender,
            recipient,
            recipient_key_id,
            request,
            &esk_seed,
        );
        esk_seed.zeroize();
        sealed
    }

    /// Reads the page of DashPay's `privateContactRequest` store from `start_position` and
    /// returns the requests it holds for the wallet whose `keys` are given: those to its
    /// identities, each sender checked against the identity key the entry names, and those its
    /// identities sent. The sender identities are fetched once each.
    ///
    /// A wallet keeps [`PrivateContactRequestScan::next_position`] as its cursor once it has
    /// stored what the page held, and scans again from it until
    /// [`PrivateContactRequestScan::is_complete`]. A failed fetch fails the scan, to be
    /// retried from the same cursor.
    pub async fn scan_private_contact_requests(
        &self,
        start_position: u64,
        keys: &PrivateDocumentScanKeys,
    ) -> Result<PrivateContactRequestScan, Error> {
        let dashpay_contract = self.fetch_dashpay_contract().await?;
        let scan = self
            .scan_private_document_page(
                dashpay_contract.id(),
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                start_position,
                keys,
            )
            .await?;
        let mut sender_identities = BTreeMap::new();
        for sender_id in private_contact_request_senders(&scan) {
            sender_identities.insert(sender_id, Identity::fetch(self, sender_id).await?);
        }
        private_contact_requests_from_scan(
            &dashpay_contract,
            scan,
            keys,
            &sender_identities,
            self.version(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::private_document_entries::{PrivateDocumentEntries, PrivateDocumentEntry};
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::DocumentPropertyType;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityV0, Purpose, SecurityLevel};
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};

    const SENDER_ID: [u8; 32] = [0xA0; 32];
    const RECIPIENT_ID: [u8; 32] = [0xB0; 32];
    /// The time of the block the test entries are appended in.
    const BLOCK_TIME_MS: u64 = 1_700_000_000_000;

    fn dashpay() -> DataContract {
        load_system_data_contract(SystemDataContract::Dashpay, PlatformVersion::latest())
            .expect("DashPay loads at the latest version")
    }

    fn secret_key(byte: u8) -> SecretKey {
        SecretKey::from_secret_bytes([byte; 32]).expect("a valid secret key")
    }

    fn key(
        id: KeyID,
        purpose: Purpose,
        key_type: KeyType,
        secret_key: &SecretKey,
        disabled_at: Option<u64>,
    ) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: None,
            key_type,
            read_only: false,
            data: BinaryData::new(PublicKey::from_secret_key(secret_key).serialize().to_vec()),
            disabled_at,
        })
    }

    fn identity(id: [u8; 32], keys: Vec<IdentityPublicKey>) -> Identity {
        Identity::V0(IdentityV0 {
            id: Identifier::from(id),
            public_keys: keys.into_iter().map(|key| (key.id(), key)).collect(),
            balance: 0,
            revision: 0,
        })
    }

    /// The sender, whose encryption key 1 is `secret_key(1)`, disabled at `disabled_at`.
    fn sender_identity(disabled_at: Option<u64>) -> Identity {
        identity(
            SENDER_ID,
            vec![
                key(
                    0,
                    Purpose::AUTHENTICATION,
                    KeyType::ECDSA_SECP256K1,
                    &secret_key(9),
                    None,
                ),
                key(
                    1,
                    Purpose::ENCRYPTION,
                    KeyType::ECDSA_SECP256K1,
                    &secret_key(1),
                    disabled_at,
                ),
            ],
        )
    }

    /// The recipient, whose decryption key 2 is `secret_key(2)`, and whose authentication key 0
    /// and BLS encryption key 3 no request may be sealed to.
    fn recipient_identity() -> Identity {
        identity(
            RECIPIENT_ID,
            vec![
                key(
                    0,
                    Purpose::AUTHENTICATION,
                    KeyType::ECDSA_SECP256K1,
                    &secret_key(8),
                    None,
                ),
                key(
                    2,
                    Purpose::DECRYPTION,
                    KeyType::ECDSA_SECP256K1,
                    &secret_key(2),
                    None,
                ),
                key(
                    3,
                    Purpose::ENCRYPTION,
                    KeyType::BLS12_381,
                    &secret_key(7),
                    None,
                ),
                key(
                    4,
                    Purpose::ENCRYPTION,
                    KeyType::ECDSA_SECP256K1,
                    &secret_key(6),
                    Some(1),
                ),
            ],
        )
    }

    /// A request filling every property to its bound: the largest payload the type holds.
    fn largest_request() -> PrivateContactRequest {
        PrivateContactRequest {
            account_reference: u32::MAX,
            extended_public_key: [0x33; COMPACT_XPUB_LEN],
            account_label: Some("l".repeat(PRIVATE_CONTACT_REQUEST_ACCOUNT_LABEL_MAX_BYTES)),
            auto_accept_proof: Some(vec![
                0x44;
                PRIVATE_CONTACT_REQUEST_AUTO_ACCEPT_PROOF_MAX_LEN
            ]),
            shielded_address: Some([0x55; PRIVATE_CONTACT_REQUEST_SHIELDED_ADDRESS_LEN]),
        }
    }

    fn smallest_request(account_reference: u32) -> PrivateContactRequest {
        PrivateContactRequest {
            account_reference,
            extended_public_key: [0x33; COMPACT_XPUB_LEN],
            account_label: None,
            auto_accept_proof: None,
            shielded_address: None,
        }
    }

    fn seal(
        contract: &DataContract,
        sender: &Identity,
        request: &PrivateContactRequest,
        seed: u8,
    ) -> Vec<u8> {
        seal_private_contact_request(
            contract,
            &PrivateContactRequestSender {
                identity: sender,
                encryption_key_id: 1,
                encryption_secret_key: &secret_key(1),
            },
            &recipient_identity(),
            2,
            request,
            &[seed; 32],
        )
        .expect("the request seals")
        .to_vec()
    }

    fn page(entries: Vec<Vec<u8>>) -> PrivateDocumentEntries {
        PrivateDocumentEntries {
            total_count: entries.len() as u64,
            entries: entries
                .into_iter()
                .zip(0u64..)
                .map(|(data, position)| PrivateDocumentEntry {
                    position,
                    block_height: 10 + position,
                    block_time_ms: BLOCK_TIME_MS,
                    data,
                })
                .collect(),
        }
    }

    /// The recipient's keys, and the sender's outgoing viewing key.
    fn scan_keys() -> PrivateDocumentScanKeys {
        PrivateDocumentScanKeys::new()
            .with_recipient(Identifier::from(RECIPIENT_ID), &[(2, secret_key(2))])
            .with_sender(
                Identifier::from(SENDER_ID),
                private_contact_request_outgoing_viewing_key(&secret_key(1)),
            )
    }

    fn read(
        contract: &DataContract,
        entries: Vec<Vec<u8>>,
        sender: Option<Identity>,
    ) -> PrivateContactRequestScan {
        let keys = scan_keys();
        let target = PrivateDocumentTarget {
            contract_id: contract.id().to_buffer(),
            document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
        };
        let scan = keys.scan_page(&target, 0, &page(entries));
        let senders = private_contact_request_senders(&scan);
        let sender_identities = senders
            .into_iter()
            .map(|sender_id| (sender_id, sender.clone()))
            .collect();
        private_contact_requests_from_scan(
            contract,
            scan,
            &keys,
            &sender_identities,
            PlatformVersion::latest(),
        )
        .expect("DashPay declares private contact requests")
    }

    /// The bounds the request checks are DashPay's: the schema's sizes, as dpp parses them.
    #[test]
    fn should_check_the_bounds_dashpay_declares() {
        let contract = dashpay();
        let document_type = private_contact_request_type(&contract).expect("DashPay v3");
        let property_type = |name: &str| {
            document_type
                .properties()
                .get(name)
                .unwrap_or_else(|| panic!("{name} is declared"))
                .property_type
                .clone()
        };
        let byte_array_sizes = |name: &str| match property_type(name) {
            DocumentPropertyType::ByteArray(sizes) => (
                sizes.min_size.map(usize::from),
                sizes.max_size.map(usize::from),
            ),
            other => panic!("{name} is a byte array, not {other:?}"),
        };
        assert_eq!(
            byte_array_sizes(EXTENDED_PUBLIC_KEY),
            (Some(COMPACT_XPUB_LEN), Some(COMPACT_XPUB_LEN))
        );
        assert_eq!(
            byte_array_sizes(AUTO_ACCEPT_PROOF),
            (
                Some(PRIVATE_CONTACT_REQUEST_AUTO_ACCEPT_PROOF_MIN_LEN),
                Some(PRIVATE_CONTACT_REQUEST_AUTO_ACCEPT_PROOF_MAX_LEN)
            )
        );
        assert_eq!(
            byte_array_sizes(SHIELDED_ADDRESS),
            (
                Some(PRIVATE_CONTACT_REQUEST_SHIELDED_ADDRESS_LEN),
                Some(PRIVATE_CONTACT_REQUEST_SHIELDED_ADDRESS_LEN)
            )
        );
        match property_type(ACCOUNT_LABEL) {
            DocumentPropertyType::String(sizes) => assert_eq!(
                sizes.max_bytes.map(usize::from),
                Some(PRIVATE_CONTACT_REQUEST_ACCOUNT_LABEL_MAX_BYTES)
            ),
            other => panic!("{ACCOUNT_LABEL} is a string, not {other:?}"),
        }
        // The schema's `maximum` is u32::MAX, which the field's type holds.
        assert!(property_type(ACCOUNT_REFERENCE).is_integer());
        assert_eq!(document_type.properties().len(), 5);
    }

    /// A request reads back from its payload as it was written, the largest one included, and
    /// a request without its optional properties reads them as absent.
    #[test]
    fn should_read_a_request_back_from_its_payload() {
        let contract = dashpay();
        let document_type = private_contact_request_type(&contract).expect("DashPay v3");
        for request in [largest_request(), smallest_request(7)] {
            let payload = encode_private_document_payload(
                document_type,
                request.to_properties().expect("in bounds"),
            )
            .expect("the payload encodes");
            let properties =
                decode_private_document_payload(document_type, &payload, PlatformVersion::latest())
                    .expect("the payload decodes");
            assert_eq!(
                PrivateContactRequest::from_properties(properties).expect("a request"),
                request
            );
        }
    }

    /// A request out of the schema's bounds is refused when written, and its properties when
    /// read, as are properties missing the required ones or holding one the type lacks.
    #[test]
    fn should_refuse_a_request_out_of_the_schema_bounds() {
        let out_of_bounds = [
            PrivateContactRequest {
                account_label: Some(
                    "l".repeat(PRIVATE_CONTACT_REQUEST_ACCOUNT_LABEL_MAX_BYTES + 1),
                ),
                ..smallest_request(1)
            },
            PrivateContactRequest {
                auto_accept_proof: Some(vec![
                    0;
                    PRIVATE_CONTACT_REQUEST_AUTO_ACCEPT_PROOF_MIN_LEN - 1
                ]),
                ..smallest_request(1)
            },
            PrivateContactRequest {
                auto_accept_proof: Some(vec![
                    0;
                    PRIVATE_CONTACT_REQUEST_AUTO_ACCEPT_PROOF_MAX_LEN + 1
                ]),
                ..smallest_request(1)
            },
        ];
        for request in &out_of_bounds {
            assert!(request.to_properties().is_err(), "{request:?}");
        }

        let properties = || smallest_request(1).to_properties().expect("in bounds");
        let with = |name: &str, value: Value| {
            let mut properties = properties();
            properties.insert(name.to_string(), value);
            properties
        };
        let without = |name: &str| {
            let mut properties = properties();
            properties.remove(name);
            properties
        };
        for (case, properties) in [
            (
                "an account reference below 0",
                with(ACCOUNT_REFERENCE, Value::I64(-1)),
            ),
            (
                "an account reference above u32::MAX",
                with(ACCOUNT_REFERENCE, Value::I64(i64::from(u32::MAX) + 1)),
            ),
            (
                "an extended public key short of 69 bytes",
                with(
                    EXTENDED_PUBLIC_KEY,
                    Value::Bytes(vec![0; COMPACT_XPUB_LEN - 1]),
                ),
            ),
            (
                "a shielded address of 44 bytes",
                with(SHIELDED_ADDRESS, Value::Bytes(vec![0; 44])),
            ),
            (
                "a label too long",
                with(ACCOUNT_LABEL, Value::Text("l".repeat(64))),
            ),
            (
                "a label of the wrong type",
                with(ACCOUNT_LABEL, Value::U32(1)),
            ),
            (
                "an unknown property",
                with("note", Value::Text("hi".to_string())),
            ),
            ("no account reference", without(ACCOUNT_REFERENCE)),
            ("no extended public key", without(EXTENDED_PUBLIC_KEY)),
        ] {
            assert!(
                PrivateContactRequest::from_properties(properties).is_err(),
                "{case} is refused"
            );
        }
        // An absent optional property may read back as null.
        assert_eq!(
            PrivateContactRequest::from_properties(with(ACCOUNT_LABEL, Value::Null))
                .expect("a request"),
            smallest_request(1)
        );
    }

    /// The recipient reads a request it received with its sender checked, the sender recovers
    /// the one it sent, and the largest request fills the entry exactly.
    #[test]
    fn should_seal_a_request_its_recipient_reads_and_its_sender_recovers() {
        let contract = dashpay();
        let sender = sender_identity(None);
        let entry = seal(&contract, &sender, &largest_request(), 1);
        assert_eq!(entry.len(), 589);

        let scan = read(&contract, vec![entry], Some(sender));
        assert_eq!(scan.rejected, vec![]);
        assert_eq!(scan.received.len(), 1);
        let received = &scan.received[0];
        assert_eq!(received.position, 0);
        assert_eq!(received.block_height, 10);
        assert_eq!(received.block_time_ms, BLOCK_TIME_MS);
        assert_eq!(received.recipient_id, Identifier::from(RECIPIENT_ID));
        assert_eq!(received.recipient_key_id, 2);
        assert_eq!(received.sender_id, Identifier::from(SENDER_ID));
        assert_eq!(received.sender_key_id, 1);
        assert_eq!(received.request, largest_request());

        assert_eq!(scan.sent.len(), 1);
        let sent = &scan.sent[0];
        assert_eq!(sent.sender_id, Identifier::from(SENDER_ID));
        assert_eq!(sent.recipient_id, Identifier::from(RECIPIENT_ID));
        assert_eq!(
            sent.recipient_public_key,
            PublicKey::from_secret_key(&secret_key(2))
        );
        assert_eq!(sent.commitment, received.commitment);
        assert_eq!(sent.request, largest_request());
        assert!(scan.is_complete());
    }

    /// A received request is refused when its sender does not exist, when the key it names was
    /// disabled before it was appended, or when the identity's key under that id did not write
    /// it; a key disabled after it was appended still wrote it.
    #[test]
    fn should_refuse_a_request_whose_sender_does_not_check() {
        let contract = dashpay();
        let entry = seal(&contract, &sender_identity(None), &smallest_request(1), 1);
        let reasons = |sender: Option<Identity>| {
            read(&contract, vec![entry.clone()], sender)
                .rejected
                .into_iter()
                .map(|rejected| rejected.reason)
                .collect::<Vec<_>>()
        };

        assert_eq!(
            reasons(None),
            vec![PrivateContactRequestRejection::SenderNotFound]
        );
        assert!(matches!(
            reasons(Some(sender_identity(Some(BLOCK_TIME_MS)))).as_slice(),
            [PrivateContactRequestRejection::SenderKeyNotUsable(_)]
        ));
        assert_eq!(
            reasons(Some(sender_identity(Some(BLOCK_TIME_MS + 1)))),
            vec![]
        );
        // The identity's key 1 is another key than the one that wrote the MAC.
        let impostor = identity(
            SENDER_ID,
            vec![key(
                1,
                Purpose::ENCRYPTION,
                KeyType::ECDSA_SECP256K1,
                &secret_key(5),
                None,
            )],
        );
        assert_eq!(
            reasons(Some(impostor)),
            vec![PrivateContactRequestRejection::SenderNotAuthenticated]
        );
        // Key 1 is not an encryption key.
        let wrong_purpose = identity(
            SENDER_ID,
            vec![key(
                1,
                Purpose::AUTHENTICATION,
                KeyType::ECDSA_SECP256K1,
                &secret_key(1),
                None,
            )],
        );
        assert!(matches!(
            reasons(Some(wrong_purpose)).as_slice(),
            [PrivateContactRequestRejection::SenderKeyNotUsable(_)]
        ));
    }

    /// An anonymous entry is no contact request, and a second request from the same sender for
    /// the same account reference is a duplicate of the first, which is kept.
    #[test]
    fn should_refuse_an_anonymous_entry_and_a_duplicate_request() {
        let contract = dashpay();
        let sender = sender_identity(None);
        let document_type = private_contact_request_type(&contract).expect("DashPay v3");
        let anonymous = seal_private_document_entry(
            &PrivateDocumentTarget {
                contract_id: contract.id().to_buffer(),
                document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
            },
            589,
            &PrivateDocumentRecipient {
                identity_id: RECIPIENT_ID,
                public_key: &PublicKey::from_secret_key(&secret_key(2)),
            },
            &PrivateDocumentSender::Anonymous,
            &encode_private_document_payload(
                document_type,
                smallest_request(1).to_properties().expect("in bounds"),
            )
            .expect("the payload encodes"),
            &[0; 32],
            &[9; 32],
        )
        .expect("the payload fits");

        let scan = read(
            &contract,
            vec![
                seal(&contract, &sender, &smallest_request(1), 1),
                anonymous,
                seal(&contract, &sender, &smallest_request(1), 2),
                seal(&contract, &sender, &smallest_request(2), 3),
            ],
            Some(sender),
        );
        assert_eq!(
            scan.received
                .iter()
                .map(|received| (received.position, received.request.account_reference))
                .collect::<Vec<_>>(),
            vec![(0, 1), (3, 2)]
        );
        assert_eq!(
            scan.rejected,
            vec![
                RejectedPrivateContactRequest {
                    position: 1,
                    block_height: 11,
                    reason: PrivateContactRequestRejection::AnonymousSender,
                },
                RejectedPrivateContactRequest {
                    position: 2,
                    block_height: 12,
                    reason: PrivateContactRequestRejection::Duplicate,
                },
            ]
        );
    }

    /// A request is sealed only from the sender's enabled ECDSA encryption key with its own
    /// secret key, and only to an enabled ECDSA decryption or encryption key of the recipient.
    #[test]
    fn should_refuse_to_seal_with_keys_dashpay_does_not_use() {
        let contract = dashpay();
        let sender = sender_identity(None);
        let attempt = |encryption_key_id: KeyID, secret: u8, recipient_key_id: KeyID| {
            seal_private_contact_request(
                &contract,
                &PrivateContactRequestSender {
                    identity: &sender,
                    encryption_key_id,
                    encryption_secret_key: &secret_key(secret),
                },
                &recipient_identity(),
                recipient_key_id,
                &smallest_request(1),
                &[1; 32],
            )
        };
        attempt(1, 1, 2).expect("the sender's encryption key to the recipient's decryption key");
        for (case, encryption_key_id, secret, recipient_key_id) in [
            ("the sender's authentication key", 0, 9, 2),
            ("another secret key than the sender key's", 1, 5, 2),
            ("a sender key the identity lacks", 7, 1, 2),
            ("the recipient's authentication key", 1, 1, 0),
            ("the recipient's BLS key", 1, 1, 3),
            ("the recipient's disabled key", 1, 1, 4),
            ("a recipient key the identity lacks", 1, 1, 9),
        ] {
            assert!(
                attempt(encryption_key_id, secret, recipient_key_id).is_err(),
                "{case}"
            );
        }
        let disabled = sender_identity(Some(1));
        assert!(seal_private_contact_request(
            &contract,
            &PrivateContactRequestSender {
                identity: &disabled,
                encryption_key_id: 1,
                encryption_secret_key: &secret_key(1),
            },
            &recipient_identity(),
            2,
            &smallest_request(1),
            &[1; 32],
        )
        .is_err());
    }
}
