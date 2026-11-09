//! DashPay private contact requests (`privateContactRequest`, DashPay v3, protocol version 15).
//!
//! A private contact request is an entry of the `privateContactRequest` store sealed to one of
//! the recipient's DashPay keys: only the recipient learns that it exists, who sent it and what
//! it holds. The sender pays for it from the shielded pool, so no identity appears on chain.
//! Inside the entry the sender names its identity and DashPay encryption key, which the pair's
//! shared key is derived from, and signs it with one of its signing keys, as it signs a public
//! request's transition. The signature is inside the encrypted entry: only the recipient sees
//! it, and could show it to someone else. A stolen recipient key forges nothing, as it forges no
//! public request.
//!
//! - Sending: [`private_contact_request_signing_data`] is what the sender's identity signer
//!   signs, [`seal_private_contact_request`] builds the entry with the signature (or
//!   [`Sdk::seal_private_contact_request`] does both), and a wallet appends it, paying from its
//!   notes, with
//!   [`PrivateDocumentCreateFromShieldedPool`](crate::platform::transition::private_document_create_from_shielded_pool::PrivateDocumentCreateFromShieldedPool)
//!   or the dpp builder.
//! - Receiving: [`Sdk::scan_private_contact_requests`] reads a page of the store, opens the
//!   wallet's entries, checks each sender against its identity's keys and reads the request.
//!
//! Each identity seals with the outgoing viewing key of its DashPay encryption key
//! ([`private_contact_request_outgoing_viewing_key`]), so a wallet restored from seed recovers
//! the requests it sent from the store alone.

use super::contact_request::{recipient_key_purpose_is_valid, sender_key_purpose_is_valid};
use crate::error::StaleNodeError;
use crate::platform::private_document_scan::{
    PrivateDocumentScan, PrivateDocumentScanKeys, ReceivedPrivateDocumentEntry,
    SentPrivateDocumentEntry,
};
use crate::platform::{Fetch, Identifier};
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::property_constraints::DocumentSystemValues;
use dpp::data_contract::document_type::{
    decode_private_document_payload, encode_private_document_payload, DocumentTypeRef,
};
use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
use dpp::data_contract::DataContract;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::identity_public_key::contract_bounds::DocumentTypeBoundsCheck;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::{BinaryData, Value};
use dpp::secp256k1::rand::rngs::StdRng;
use dpp::secp256k1::rand::{RngCore, SeedableRng};
use dpp::secp256k1::{PublicKey, SecretKey};
use dpp::util::hash::ripemd160_sha256;
use dpp::version::feature_initial_protocol_versions::PRIVATE_DOCUMENT_CREATE_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION;
use dpp::version::PlatformVersion;
use futures::{StreamExt, TryStreamExt};
use platform_encryption::{
    private_document_outgoing_viewing_key, private_document_signing_data,
    recover_private_document_signer, seal_private_document_entry, OpenedPrivateDocumentEntry,
    PrivateDocumentAuthMode, PrivateDocumentEnvelopeError, PrivateDocumentRecipient,
    PrivateDocumentSender, PrivateDocumentTarget, COMPACT_XPUB_LEN,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use zeroize::{Zeroize, Zeroizing};

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
    /// The request as the type's properties. Their bounds are the schema's, which sealing
    /// checks with dpp's document validation.
    pub fn to_properties(&self) -> BTreeMap<String, Value> {
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
        properties
    }

    /// The request a private contact request's properties hold, refused when one is missing,
    /// unknown or of the wrong type, or when the extended public key or the shielded address,
    /// both fixed-size fields, is not its size. The schema's other bounds, such as the account
    /// label's length and the auto-accept proof's size, are not checked here: dpp's document
    /// validation of the type's schema checks them when a request is sealed and when one is
    /// read, before this. A caller parsing properties itself validates them the same way first,
    /// or gets a request holding what it passed.
    pub fn from_properties(mut properties: BTreeMap<String, Value>) -> Result<Self, Error> {
        let invalid =
            |message: String| Error::Generic(format!("private contact request: {message}"));
        // An absent optional property may read back as null. A null under any other name is
        // left for the unknown property check below.
        properties.retain(|name, value| {
            !(value.is_null()
                && [ACCOUNT_LABEL, AUTO_ACCEPT_PROOF, SHIELDED_ADDRESS].contains(&name.as_str()))
        });
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
        Ok(request)
    }
}

/// Checks `properties` against the `privateContactRequest` schema of `dashpay_contract` as dpp
/// checks a document's properties: the schema's bounds and types, its required properties, and
/// the rest it says. The payload codec leaves this to the sender before encoding and to the
/// recipient after decoding.
fn validate_request_properties(
    dashpay_contract: &DataContract,
    properties: &BTreeMap<String, Value>,
    platform_version: &PlatformVersion,
) -> Result<(), String> {
    let result = dashpay_contract
        .validate_document_properties(
            PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
            Value::from(properties.clone()),
            &DocumentSystemValues::default(),
            platform_version,
        )
        .map_err(|error| error.to_string())?;
    match result.errors.first() {
        None => Ok(()),
        Some(error) => Err(error.to_string()),
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

/// The key types a private contact request may be signed with: those a public request's
/// transition may be signed with, whose signatures recover their key.
pub const PRIVATE_CONTACT_REQUEST_SIGNING_KEY_TYPES: [KeyType; 2] =
    [KeyType::ECDSA_SECP256K1, KeyType::ECDSA_HASH160];

/// The security levels an `AUTHENTICATION` key may sign a private contact request at: the ones
/// Platform lets a document of the type be signed at
/// ([`SecurityLevel::document_signing_levels`] of the type's requirement), from `CRITICAL` to
/// the requirement. A private type cannot declare a requirement, so it has the default, `HIGH`,
/// and this is `[CRITICAL, HIGH]`. A `privateContactRequest` type that is not private, in a
/// contract parsed without full validation, may require `MASTER`: then this is `[MASTER]`, as
/// Platform signs such a type's documents with master keys only. Sealing and reading refuse a
/// type that is not private.
pub fn private_contact_request_signing_security_levels(
    dashpay_contract: &DataContract,
) -> Result<Vec<SecurityLevel>, Error> {
    Ok(private_contact_request_type(dashpay_contract)?
        .security_level_requirement()
        .document_signing_levels())
}

/// What a private contact request's sender signs, before sealing it: the entry's signing data
/// for `request` from the encryption key `encryption_key_id` of `sender` to the key
/// `recipient_key_id` of `recipient`. The sender's identity signer signs it as data, with one of
/// its keys [`private_contact_request_signing_security_levels`] and
/// [`PRIVATE_CONTACT_REQUEST_SIGNING_KEY_TYPES`] allow, as it signs a public request's
/// transition; [`private_contact_request_signature`] does that.
pub fn private_contact_request_signing_data(
    dashpay_contract: &DataContract,
    sender: &Identity,
    encryption_key_id: KeyID,
    recipient: &Identity,
    recipient_key_id: KeyID,
    request: &PrivateContactRequest,
    platform_version: &PlatformVersion,
) -> Result<[u8; 32], Error> {
    Ok(SealInputs::new(
        dashpay_contract,
        sender,
        encryption_key_id,
        recipient,
        recipient_key_id,
        request,
        platform_version,
    )?
    .signing_data)
}

/// `signer`'s signature with `signing_key` of a private contact request's `signing_data`, in the
/// 65-byte compact form the entry carries.
pub async fn private_contact_request_signature<S>(
    signer: &S,
    signing_key: &IdentityPublicKey,
    signing_data: &[u8; 32],
) -> Result<[u8; 65], Error>
where
    S: Signer<IdentityPublicKey> + ?Sized,
{
    let signed = signer.sign(signing_key, signing_data).await?;
    let mut signature: [u8; 65] = signed.as_slice().try_into().map_err(|_| {
        Error::Generic(format!(
            "the signer's signature with key {} is {} bytes, not a 65-byte compact signature",
            signing_key.id(),
            signed.len()
        ))
    })?;
    // A header for an uncompressed key (27 + recovery id) names the same signature.
    if (27..31).contains(&signature[0]) {
        signature[0] += 4;
    }
    Ok(signature)
}

/// The sender of a private contact request: its identity, the DashPay encryption key it names
/// in the entry, and its signature of the request.
pub struct PrivateContactRequestSender<'a> {
    /// The sending identity.
    pub identity: &'a Identity,
    /// The id of its enabled `ECDSA_SECP256K1` `ENCRYPTION` key, which the pair's shared key is
    /// derived from.
    pub encryption_key_id: KeyID,
    /// That key's secret key, which the outgoing viewing key is derived from.
    pub encryption_secret_key: &'a SecretKey,
    /// The id of the enabled signing key that signed the request.
    pub signing_key_id: KeyID,
    /// That key's signature of [`private_contact_request_signing_data`].
    pub signature: &'a [u8; 65],
}

/// Seals `request` from `sender` to the key `recipient_key_id` of `recipient`: an entry of the
/// `privateContactRequest` store of `dashpay_contract`, of the type's entry size.
///
/// - The encryption key must be the sender's enabled `ECDSA_SECP256K1` `ENCRYPTION` key whose
///   secret key is given.
/// - The signature must be the signing key's, of [`private_contact_request_signing_data`]. The
///   signing key must be an enabled `AUTHENTICATION` key of the sender at a level and of a type
///   that may sign the type's documents.
/// - The recipient key must be an enabled `ECDSA_SECP256K1` `DECRYPTION` or `ENCRYPTION` key.
///
/// - The request must satisfy the type's schema (dpp's document validation).
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
    platform_version: &PlatformVersion,
) -> Result<BinaryData, Error> {
    let inputs = SealInputs::new(
        dashpay_contract,
        sender.identity,
        sender.encryption_key_id,
        recipient,
        recipient_key_id,
        request,
        platform_version,
    )?;
    if inputs.encryption_public_key != PublicKey::from_secret_key(sender.encryption_secret_key) {
        return Err(Error::Generic(format!(
            "the secret key given is not the one of the sender key {}",
            sender.encryption_key_id
        )));
    }
    let signing_key = sender
        .identity
        .get_public_key_by_id(sender.signing_key_id)
        .ok_or_else(|| {
            Error::Generic(format!(
                "the sender identity {} has no key {}",
                sender.identity.id(),
                sender.signing_key_id
            ))
        })?;
    if let Some(problem) = signing_key_problem(
        signing_key,
        &private_contact_request_signing_security_levels(dashpay_contract)?,
        dashpay_contract.id(),
        None,
    ) {
        return Err(Error::Generic(problem));
    }
    let signer = recover_private_document_signer(sender.signature, &inputs.signing_data)
        .map_err(|error| Error::Generic(format!("the request's signature: {error}")))?;
    if !key_is(signing_key, &signer) {
        return Err(Error::Generic(format!(
            "the signature is not the sender key {}'s signature of this request",
            sender.signing_key_id
        )));
    }

    // Wiped on every return.
    let outgoing_viewing_key = Zeroizing::new(private_contact_request_outgoing_viewing_key(
        sender.encryption_secret_key,
    ));
    let entry = seal_private_document_entry(
        &inputs.target(),
        inputs.entry_size,
        &PrivateDocumentRecipient {
            identity_id: recipient.id().to_buffer(),
            public_key: &inputs.recipient_public_key,
        },
        &PrivateDocumentSender::Signature {
            identity_id: sender.identity.id().to_buffer(),
            key_id: sender.encryption_key_id,
            signature: sender.signature,
        },
        &inputs.payload,
        &outgoing_viewing_key,
        esk_seed,
    )
    .map_err(|error| Error::Generic(format!("sealing a private contact request: {error}")))?;
    Ok(BinaryData::new(entry))
}

/// What sealing a request takes from its keys and the contract, once checked.
struct SealInputs {
    contract_id: [u8; 32],
    entry_size: usize,
    payload: Vec<u8>,
    encryption_public_key: PublicKey,
    recipient_public_key: PublicKey,
    signing_data: [u8; 32],
}

impl SealInputs {
    fn new(
        dashpay_contract: &DataContract,
        sender: &Identity,
        encryption_key_id: KeyID,
        recipient: &Identity,
        recipient_key_id: KeyID,
        request: &PrivateContactRequest,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let document_type = private_contact_request_type(dashpay_contract)?;
        let entry_size = document_type
            .private_document()
            .map(|config| config.entry_size())
            .ok_or_else(|| {
                Error::Generic(format!(
                    "DashPay {PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME} is not a private type"
                ))
            })?;
        let encryption_public_key =
            encryption_key(sender, encryption_key_id, dashpay_contract.id(), None)
                .map_err(Error::Generic)?;

        let recipient_public_key =
            recipient_key(recipient, recipient_key_id, dashpay_contract.id(), None)
                .map_err(Error::Generic)?;

        let properties = request.to_properties();
        validate_request_properties(dashpay_contract, &properties, platform_version)
            .map_err(|error| Error::Generic(format!("private contact request: {error}")))?;
        let payload = encode_private_document_payload(document_type, properties)?;
        let contract_id = dashpay_contract.id().to_buffer();
        let signing_data = private_document_signing_data(
            &PrivateDocumentTarget {
                contract_id,
                document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
            },
            &sender.id().to_buffer(),
            encryption_key_id,
            &PrivateDocumentRecipient {
                identity_id: recipient.id().to_buffer(),
                public_key: &recipient_public_key,
            },
            &payload,
        );
        Ok(Self {
            contract_id,
            entry_size: entry_size as usize,
            payload,
            encryption_public_key,
            recipient_public_key,
            signing_data,
        })
    }

    fn target(&self) -> PrivateDocumentTarget<'static> {
        PrivateDocumentTarget {
            contract_id: self.contract_id,
            document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
        }
    }
}

/// The public key of `identity`'s DashPay encryption key `key_id`: an `ECDSA_SECP256K1`
/// `ENCRYPTION` key, enabled at `at_ms` (now when `None`).
fn encryption_key(
    identity: &Identity,
    key_id: KeyID,
    dashpay_id: Identifier,
    at_ms: Option<u64>,
) -> Result<PublicKey, String> {
    let key = identity
        .get_public_key_by_id(key_id)
        .ok_or_else(|| format!("the sender identity {} has no key {key_id}", identity.id()))?;
    if !sender_key_purpose_is_valid(key.purpose()) {
        return Err(format!(
            "the sender key {key_id} is a {} key, not an ENCRYPTION key",
            key.purpose()
        ));
    }
    ecdh_public_key(key, "sender", dashpay_id, at_ms)
}

/// The public key of `recipient`'s key `key_id` that a private contact request may be
/// encrypted to: an `ECDSA_SECP256K1` `DECRYPTION` or `ENCRYPTION` key, enabled at `at_ms` (now
/// when `None`) and bound, if at all, to DashPay's contact requests. Sealing checks it now and
/// reading at the entry's block time; the private format has no older keys to tolerate.
fn recipient_key(
    recipient: &Identity,
    key_id: KeyID,
    dashpay_id: Identifier,
    at_ms: Option<u64>,
) -> Result<PublicKey, String> {
    let key = recipient.get_public_key_by_id(key_id).ok_or_else(|| {
        format!(
            "the recipient identity {} has no key {key_id}",
            recipient.id()
        )
    })?;
    if !recipient_key_purpose_is_valid(key.purpose()) {
        return Err(format!(
            "the recipient key {key_id} is a {} key, not a DECRYPTION or ENCRYPTION key",
            key.purpose()
        ));
    }
    ecdh_public_key(key, "recipient", dashpay_id, at_ms)
}

/// The public key of an `ECDSA_SECP256K1` identity key not disabled at `at_ms` (now when
/// `None`), and bound, if at all, to the DashPay contract `dashpay_id` or its contact requests,
/// which ECDH runs on. A key disabled after `at_ms` was not disabled then: keys are disabled,
/// never removed.
fn ecdh_public_key(
    key: &IdentityPublicKey,
    role: &str,
    dashpay_id: Identifier,
    at_ms: Option<u64>,
) -> Result<PublicKey, String> {
    if key.key_type() != KeyType::ECDSA_SECP256K1 {
        return Err(format!(
            "the {role} key {} is a {} key, not ECDSA_SECP256K1",
            key.id(),
            key.key_type()
        ));
    }
    if let Some(problem) = disabled_problem(key, role, at_ms).or_else(|| {
        bounds_problem(
            key,
            role,
            dashpay_id,
            &PRIVATE_CONTACT_REQUEST_ENCRYPTION_KEY_DOCUMENT_TYPES,
        )
    }) {
        return Err(problem);
    }
    PublicKey::from_slice(key.data().as_slice()).map_err(|error| {
        format!(
            "the {role} key {} is not a secp256k1 point: {error}",
            key.id()
        )
    })
}

/// Why `key` was not enabled at `at_ms` (now when `None`), if it was not.
fn disabled_problem(key: &IdentityPublicKey, role: &str, at_ms: Option<u64>) -> Option<String> {
    let disabled_at = key.disabled_at()?;
    match at_ms {
        None => Some(format!("the {role} key {} is disabled", key.id())),
        Some(at_ms) if disabled_at <= at_ms => Some(format!(
            "the {role} key {} was disabled at {disabled_at}, before the entry",
            key.id()
        )),
        Some(_) => None,
    }
}

/// The DashPay document types a private contact request's ENCRYPTION and DECRYPTION keys may be
/// bound to. A private request carries what a public `contactRequest` does, encrypted to the
/// same keys, and DashPay asks for those keys to be bound to `contactRequest`.
const PRIVATE_CONTACT_REQUEST_ENCRYPTION_KEY_DOCUMENT_TYPES: [&str; 2] =
    ["contactRequest", PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME];

/// Why the `role` key `key` may not be used for a private contact request of the DashPay
/// contract `dashpay_id`, if its contract bounds keep it from one: a bound key is used only for
/// one of `document_type_names` of DashPay, as dpp's `ContractBounds::check_document_type`
/// decides. A contract group bound depends on the group's members, which a client does not
/// read, so it is refused.
fn bounds_problem(
    key: &IdentityPublicKey,
    role: &str,
    dashpay_id: Identifier,
    document_type_names: &[&str],
) -> Option<String> {
    let bounds = key.contract_bounds()?;
    let mut contract_group_id = None;
    for document_type_name in document_type_names {
        match bounds.check_document_type(&dashpay_id, document_type_name) {
            DocumentTypeBoundsCheck::Allowed => return None,
            DocumentTypeBoundsCheck::Denied => {}
            DocumentTypeBoundsCheck::RequiresContractGroupMembership {
                contract_group_id: group_id,
            } => contract_group_id = Some(group_id),
        }
    }
    let scope = document_type_names.join(" or ");
    Some(match contract_group_id {
        Some(group_id) => format!(
            "the {role} key {} is bound to the contract group {group_id}, whose members a client \
             does not read, not to DashPay's {scope}",
            key.id()
        ),
        None => format!(
            "the {role} key {} is bound to the {} {}{}, not to DashPay's {scope}",
            key.id(),
            bounds.contract_bounds_type_string(),
            bounds.identifier(),
            bounds
                .document_type()
                .map(|name| format!(" {name}"))
                .unwrap_or_default()
        ),
    })
}

/// Why `key` may not sign a private contact request of the DashPay contract `dashpay_id` at
/// `at_ms` (now, by this device's clock, when `None`), if it may not: it must be an
/// `AUTHENTICATION` key at one of `levels`, of a signing key type, bound to no other contract or
/// document type, enabled then and not expired then (the expiry instant itself is expired, as
/// dpp's `is_expired_at` says). The reader checks it at the entry's block time, which decides;
/// the seal and the key list check it now, so a key about to be refused is not offered.
fn signing_key_problem(
    key: &IdentityPublicKey,
    levels: &[SecurityLevel],
    dashpay_id: Identifier,
    at_ms: Option<u64>,
) -> Option<String> {
    if key.purpose() != Purpose::AUTHENTICATION {
        return Some(format!(
            "the signing key {} is a {} key, not an AUTHENTICATION key",
            key.id(),
            key.purpose()
        ));
    }
    if !levels.contains(&key.security_level()) {
        return Some(format!(
            "the signing key {} is a {} key, which signs no {PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME}",
            key.id(),
            key.security_level()
        ));
    }
    if !PRIVATE_CONTACT_REQUEST_SIGNING_KEY_TYPES.contains(&key.key_type()) {
        return Some(format!(
            "the signing key {} is a {} key, whose signature names no key",
            key.id(),
            key.key_type()
        ));
    }
    // A bound key signs only inside its bounds, as a transition's does. A group bound depends
    // on the group's members when the entry was appended, which a client cannot read, so it
    // signs no private request.
    if let Some(problem) = bounds_problem(
        key,
        "signing",
        dashpay_id,
        &[PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME],
    ) {
        return Some(problem);
    }
    let expiry_checked_at = at_ms.unwrap_or_else(now_ms);
    if key.is_expired_at(expiry_checked_at) {
        return Some(format!(
            "the signing key {} expired at {}, by {expiry_checked_at}",
            key.id(),
            key.expires_at().unwrap_or_default()
        ));
    }
    disabled_problem(key, "signing", at_ms)
}

/// This device's clock, in milliseconds since the Unix epoch: block times are measured the same
/// way. A clock before the epoch reads as 0.
fn now_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or_default()
    }
}

/// The keys of `identity` that may sign a private contact request now, in key id order: its
/// enabled, unexpired `AUTHENTICATION` keys at a level the type's documents are signed at, of a
/// signing key type, bound to no other contract or document type. A wallet signs with the first
/// its signer holds.
pub fn private_contact_request_signing_keys<'a>(
    dashpay_contract: &DataContract,
    identity: &'a Identity,
) -> Result<Vec<&'a IdentityPublicKey>, Error> {
    let levels = private_contact_request_signing_security_levels(dashpay_contract)?;
    Ok(identity
        .public_keys()
        .values()
        .filter(|key| signing_key_problem(key, &levels, dashpay_contract.id(), None).is_none())
        .collect())
}

/// Whether the identity key `key` is the secp256k1 key `public_key`.
fn key_is(key: &IdentityPublicKey, public_key: &PublicKey) -> bool {
    match key.key_type() {
        KeyType::ECDSA_SECP256K1 => PublicKey::from_slice(key.data().as_slice())
            .is_ok_and(|key_public_key| key_public_key == *public_key),
        KeyType::ECDSA_HASH160 => {
            key.data().as_slice() == ripemd160_sha256(&public_key.serialize()).as_slice()
        }
        _ => false,
    }
}

/// Whether `contract`, the DashPay a context provider holds, declares private contact requests.
#[cfg(test)]
fn holds_private_contact_requests(contract: Option<&DataContract>) -> bool {
    contract.is_some_and(|contract| private_contact_request_type(contract).is_ok())
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
    /// The sender's encryption key it names, which the pair's shared key is derived from.
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
    /// The sender's encryption key it names.
    pub sender_key_id: KeyID,
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
    /// It carries a deniable MAC instead of a signature: anyone holding the recipient's key
    /// could have written that, so it proves nothing about its sender.
    NotSigned,
    /// The identity it names as its sender does not exist.
    SenderNotFound,
    /// The encryption key it names is missing, not an `ECDSA_SECP256K1` `ENCRYPTION` key, or
    /// was disabled before the entry was appended; or the sender key that signed it may not
    /// sign the type's documents, or was disabled before the entry was appended.
    SenderKeyNotUsable(String),
    /// No key of the sender signed it; or, for a sent entry, it names another identity or key
    /// than the outgoing viewing key that recovered it is derived from.
    SenderNotAuthenticated,
    /// The recipient key it is encrypted to was disabled before it was appended, or the
    /// recipient identity was not given to check it.
    RecipientKeyNotUsable(String),
    /// Its payload is not a valid private contact request. The sender is checked first, so the
    /// sender wrote it so. One written after DashPay gained a property the contract the SDK
    /// holds lacks reads as invalid until the contract is refetched, so a wallet keeps the
    /// position to read again.
    InvalidRequest(String),
    /// An earlier entry of the scan holds a request from the same sender to the same
    /// recipient for the same account reference, which is kept. Received and sent requests are
    /// counted apart, so a request the wallet both sent and received is kept on both sides.
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

/// An identity a scan's entries are checked against, as fetched: `None` when it does not exist,
/// and the height of the block its proof is of. An entry is checked against a snapshot proved
/// at or after the block that appended it, so a key disabled in that block shows as disabled.
#[derive(Clone, Debug, PartialEq)]
pub struct ProvedIdentity {
    /// The identity, `None` when the proof shows it does not exist.
    pub identity: Option<Identity>,
    /// The height of the block the proof is of.
    pub proved_at_height: u64,
}

/// The error for an identity proved at `received_height`, before the block `expected_height` of
/// an entry it checks: the node's snapshot is older than the entry, so fetching it again, from
/// another node or later, may succeed ([`StaleNodeError::Height`], which `can_retry` takes as
/// retryable).
fn stale_identity(expected_height: u64, received_height: u64) -> Error {
    Error::StaleNode(StaleNodeError::Height {
        expected_height,
        received_height,
        tolerance_blocks: 0,
    })
}

/// How many identities [`Sdk::fetch_private_contact_request_identities`] fetches at once.
const PRIVATE_CONTACT_REQUEST_IDENTITY_FETCHES: usize = 4;

/// [`Sdk::fetch_private_contact_request_identities`] with `fetch` proving one identity: each
/// identity `needed` lists is fetched, a few at a time, until proved at or after its height,
/// twice at most. A stale second proof fails the whole fetch with the retryable
/// `Error::StaleNode(StaleNodeError::Height)`, and a failed fetch with its own error.
async fn fetch_proved_identities<F, Fut>(
    needed: &BTreeMap<Identifier, u64>,
    fetch: F,
) -> Result<BTreeMap<Identifier, ProvedIdentity>, Error>
where
    F: Fn(Identifier) -> Fut + Sync,
    Fut: std::future::Future<Output = Result<ProvedIdentity, Error>> + Send,
{
    let fetch = &fetch;
    // Owned pairs: a future borrowing the map's entries through the stream's closure is not
    // `Send` (rust-lang/rust#100013), and hosts spawn this one.
    let needed: Vec<(Identifier, u64)> = needed
        .iter()
        .map(|(identity_id, height)| (*identity_id, *height))
        .collect();
    futures::stream::iter(needed)
        .map(|(identity_id, height)| async move {
            let mut proved = fetch(identity_id).await?;
            if proved.proved_at_height < height {
                proved = fetch(identity_id).await?;
            }
            if proved.proved_at_height < height {
                tracing::debug!(
                    identity = %identity_id,
                    proved_at = proved.proved_at_height,
                    needed = height,
                    "an identity was proved twice before the block of an entry it checks"
                );
                return Err(stale_identity(height, proved.proved_at_height));
            }
            Ok((identity_id, proved))
        })
        .buffer_unordered(PRIVATE_CONTACT_REQUEST_IDENTITY_FETCHES)
        .try_collect()
        .await
}

/// The identities whose keys [`private_contact_requests_from_scan`] checks the entries of
/// `scan` against, each with the height of the latest block whose entry it checks: a snapshot
/// of it must be proved at or after that block. They are the senders signed received entries
/// name, the wallet's identities those are to, and the wallet's identities that sent the
/// signed recovered ones. Anonymous entries and entries with a deniable MAC are refused without
/// reading an identity, so they add none.
///
/// Fetching a sender right after the page that holds its request tells the node that serves
/// both which identity the wallet looked up, and so that someone it knows sent the wallet a
/// request. A wallet that knows a sender already (a contact, or one fetched for other reasons)
/// passes the identity it holds, and fetches the others from another node or later, as its
/// privacy needs.
pub fn private_contact_request_identities(scan: &PrivateDocumentScan) -> BTreeMap<Identifier, u64> {
    let signed = |entry: &OpenedPrivateDocumentEntry| {
        entry.auth_mode() == PrivateDocumentAuthMode::Signature
    };
    let mut needed: BTreeMap<Identifier, u64> = BTreeMap::new();
    let mut need = |identity_id: Identifier, block_height: u64| {
        if identity_id != Identifier::default() {
            let height = needed.entry(identity_id).or_default();
            *height = (*height).max(block_height);
        }
    };
    for received in scan
        .received
        .iter()
        .filter(|received| signed(&received.entry))
    {
        need(
            Identifier::from(*received.entry.sender_identity_id()),
            received.block_height,
        );
        need(received.recipient_identity_id, received.block_height);
    }
    for sent in scan.sent.iter().filter(|sent| signed(&sent.entry)) {
        need(sent.sender_identity_id, sent.block_height);
    }
    needed
}

/// Reads the private contact requests of a scan of DashPay's `privateContactRequest` store.
///
/// `identities` holds the identities [`private_contact_request_identities`] lists, each proved
/// at or after the height it lists. One proved earlier fails the read with the retryable
/// `Error::StaleNode(StaleNodeError::Height)`, to be fetched again; a missing one, or one given
/// under another id, is the caller's error, which fetching again does not lift. Each entry, received or sent, is checked before its payload is read:
/// - it must be signed, and its sender identity must exist;
/// - the encryption key it names must be the sender's `ECDSA_SECP256K1` `ENCRYPTION` key, not
///   disabled when the entry was appended, and bound, if at all, to DashPay or its contact
///   requests, public or private;
/// - its signature must be by a key of the sender that may sign the type's documents, neither
///   disabled nor expired then;
/// - a received entry's recipient key must not have been disabled then either, and be bound
///   the same way;
/// - a sent entry must name the identity and key its outgoing viewing key is derived from;
/// - its payload must satisfy the type's schema (dpp's document validation).
///
/// The keys are those of the identities given, which may be newer than the entry. That a key
/// was already the identity's when the entry was appended is not established: Platform records
/// when a key was disabled, not when it was added. Adding a key takes the identity's master key,
/// so a key added later still speaks for the identity's owner.
///
/// The first request from a sender to a recipient for an account reference is kept. A wallet
/// keeps the requests it already holds from earlier scans, and drops a later one from the same
/// sender for the same account reference as this does.
pub fn private_contact_requests_from_scan(
    dashpay_contract: &DataContract,
    scan: PrivateDocumentScan,
    identities: &BTreeMap<Identifier, ProvedIdentity>,
    platform_version: &PlatformVersion,
) -> Result<PrivateContactRequestScan, Error> {
    // A contract whose type is not private holds no store, as sealing refuses it: an empty
    // scan of it is refused too, rather than handing back its cursor.
    if private_contact_request_type(dashpay_contract)?
        .private_document()
        .is_none()
    {
        return Err(Error::Generic(format!(
            "DashPay {PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME} is not a private type"
        )));
    }
    // A scan of another store's page is refused whatever it holds: even with no entry the
    // wallet's keys open, its cursor is that store's, which must not be kept as DashPay's.
    if !scan.is_of_store(
        &dashpay_contract.id(),
        PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
    ) {
        return Err(Error::Generic(format!(
            "the scan is of the store {} {}, not DashPay's \
             {PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME}",
            scan.contract_id, scan.document_type_name
        )));
    }
    // An entry opens only for the store it was sealed for: one of another store in a DashPay
    // scan is refused, not read as a DashPay request.
    let dashpay_id = dashpay_contract.id().to_buffer();
    if let Some(entry) = scan
        .received
        .iter()
        .map(|received| &received.entry)
        .chain(scan.sent.iter().map(|sent| &sent.entry))
        .find(|entry| {
            entry.contract_id() != &dashpay_id
                || entry.document_type_name() != PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME
        })
    {
        return Err(Error::Generic(format!(
            "the scan holds an entry of the store {} {}, not DashPay's \
             {PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME}",
            Identifier::from(*entry.contract_id()),
            entry.document_type_name()
        )));
    }
    for (identity_id, height) in private_contact_request_identities(&scan) {
        match identities.get(&identity_id) {
            None => {
                return Err(Error::Generic(format!(
                    "the identity {identity_id}, which an entry of the scan is checked against, \
                     was not given"
                )))
            }
            Some(ProvedIdentity {
                identity: Some(identity),
                ..
            }) if identity.id() != identity_id => {
                return Err(Error::Generic(format!(
                    "the identity given for {identity_id} is {}",
                    identity.id()
                )))
            }
            Some(proved) if proved.proved_at_height < height => {
                tracing::debug!(
                    identity = %identity_id,
                    proved_at = proved.proved_at_height,
                    needed = height,
                    "an identity was proved before the block of an entry it checks"
                );
                return Err(stale_identity(height, proved.proved_at_height));
            }
            Some(_) => {}
        }
    }
    let document_type = private_contact_request_type(dashpay_contract)?;
    let signing_levels = private_contact_request_signing_security_levels(dashpay_contract)?;
    let reader = EntryReader {
        identities,
        signing_levels: &signing_levels,
        dashpay_id: dashpay_contract.id(),
        decode: &|payload: &[u8]| {
            let properties =
                decode_private_document_payload(document_type, payload, platform_version).map_err(
                    |error| PrivateContactRequestRejection::InvalidRequest(error.to_string()),
                )?;
            validate_request_properties(dashpay_contract, &properties, platform_version)
                .map_err(PrivateContactRequestRejection::InvalidRequest)?;
            PrivateContactRequest::from_properties(properties)
                .map_err(|error| PrivateContactRequestRejection::InvalidRequest(error.to_string()))
        },
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
        match reader.read_received(&received) {
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

    let mut seen_sent = BTreeSet::new();
    for sent in scan.sent {
        let reject = |reason| RejectedPrivateContactRequest {
            position: sent.position,
            block_height: sent.block_height,
            reason,
        };
        match reader.read_sent(&sent) {
            Ok(request) => {
                let recipient_id = Identifier::from(*sent.entry.recipient_identity_id());
                if !seen_sent.insert((
                    sent.sender_identity_id,
                    recipient_id,
                    request.account_reference,
                )) {
                    rejected.push(reject(PrivateContactRequestRejection::Duplicate));
                    continue;
                }
                result.sent.push(SentPrivateContactRequest {
                    position: sent.position,
                    block_height: sent.block_height,
                    block_time_ms: sent.block_time_ms,
                    sender_id: sent.sender_identity_id,
                    sender_key_id: sent.entry.sender_key_id(),
                    recipient_id,
                    recipient_public_key: *sent.entry.recipient_public_key(),
                    commitment: *sent.entry.commitment(),
                    request,
                });
            }
            Err(reason) => rejected.push(reject(reason)),
        }
    }

    rejected.sort_by_key(|rejected| rejected.position);
    result.rejected = rejected;
    Ok(result)
}

type DecodeRequest<'a> =
    dyn Fn(&[u8]) -> Result<PrivateContactRequest, PrivateContactRequestRejection> + 'a;

/// Reads the entries of one scan against the identities fetched for it.
struct EntryReader<'a> {
    identities: &'a BTreeMap<Identifier, ProvedIdentity>,
    signing_levels: &'a [SecurityLevel],
    dashpay_id: Identifier,
    decode: &'a DecodeRequest<'a>,
}

impl EntryReader<'_> {
    fn identity(&self, identity_id: &Identifier) -> Option<&Identity> {
        self.identities
            .get(identity_id)
            .and_then(|proved| proved.identity.as_ref())
    }

    /// The request a received entry holds, once its sender and recipient key are checked.
    fn read_received(
        &self,
        received: &ReceivedPrivateDocumentEntry,
    ) -> Result<PrivateContactRequest, PrivateContactRequestRejection> {
        self.check_sender(&received.entry, received.block_time_ms)?;
        // The key the entry opened with, checked as sealing checks it, at the entry's time.
        let recipient_key_problem = match self.identity(&received.recipient_identity_id) {
            None => Some("the recipient identity does not exist".to_string()),
            Some(recipient) => match recipient_key(
                recipient,
                received.recipient_key_id,
                self.dashpay_id,
                Some(received.block_time_ms),
            ) {
                Err(problem) => Some(problem),
                Ok(public_key) if public_key != *received.entry.recipient_public_key() => {
                    Some(format!(
                        "the recipient key {} is not the key the entry is encrypted to",
                        received.recipient_key_id
                    ))
                }
                Ok(_) => None,
            },
        };
        if let Some(problem) = recipient_key_problem {
            return Err(PrivateContactRequestRejection::RecipientKeyNotUsable(
                problem,
            ));
        }
        (self.decode)(received.entry.payload())
    }

    /// The request a sent entry holds, once it is checked to be the identity's own: someone
    /// else holding its encryption key could build entries its outgoing viewing key recovers.
    fn read_sent(
        &self,
        sent: &SentPrivateDocumentEntry,
    ) -> Result<PrivateContactRequest, PrivateContactRequestRejection> {
        if Identifier::from(*sent.entry.sender_identity_id()) != sent.sender_identity_id
            || sent.entry.sender_key_id() != sent.outgoing_key_id
        {
            return Err(PrivateContactRequestRejection::SenderNotAuthenticated);
        }
        self.check_sender(&sent.entry, sent.block_time_ms)?;
        (self.decode)(sent.entry.payload())
    }

    /// Checks that the identity an entry names as its sender wrote it: by the keys of the proved
    /// identity, which may be newer than the entry, none disabled (nor, for the signing key,
    /// expired) at the entry's `block_time_ms`.
    /// [`private_contact_requests_from_scan`] says what this does not establish.
    fn check_sender(
        &self,
        entry: &OpenedPrivateDocumentEntry,
        block_time_ms: u64,
    ) -> Result<(), PrivateContactRequestRejection> {
        let sender_id = Identifier::from(*entry.sender_identity_id());
        match entry.auth_mode() {
            PrivateDocumentAuthMode::Anonymous => {
                return Err(PrivateContactRequestRejection::AnonymousSender)
            }
            // Anyone holding the recipient's key could have written a deniable MAC.
            PrivateDocumentAuthMode::DeniableMac => {
                return Err(PrivateContactRequestRejection::NotSigned)
            }
            PrivateDocumentAuthMode::Signature => {}
        }
        if sender_id == Identifier::default() {
            return Err(PrivateContactRequestRejection::AnonymousSender);
        }
        let sender = self
            .identity(&sender_id)
            .ok_or(PrivateContactRequestRejection::SenderNotFound)?;
        encryption_key(
            sender,
            entry.sender_key_id(),
            self.dashpay_id,
            Some(block_time_ms),
        )
        .map_err(PrivateContactRequestRejection::SenderKeyNotUsable)?;
        let signer = entry
            .recover_signer()
            .map_err(|_| PrivateContactRequestRejection::SenderNotAuthenticated)?;
        let mut problem = None;
        for key in sender.public_keys().values() {
            if !key_is(key, &signer) {
                continue;
            }
            match signing_key_problem(
                key,
                self.signing_levels,
                self.dashpay_id,
                Some(block_time_ms),
            ) {
                None => return Ok(()),
                Some(key_problem) => {
                    problem.get_or_insert(key_problem);
                }
            }
        }
        Err(problem.map_or(
            PrivateContactRequestRejection::SenderNotAuthenticated,
            PrivateContactRequestRejection::SenderKeyNotUsable,
        ))
    }
}

impl Sdk {
    /// Seals `request` from the encryption key `encryption_key_id` of `sender`, whose secret key
    /// is given, to the key `recipient_key_id` of `recipient`, as
    /// [`seal_private_contact_request`] does: with the DashPay contract, a fresh ephemeral seed,
    /// and `signer`'s signature with the first of the sender's keys that may sign the request
    /// and that it holds. The entry is appended by a transition paid from the sender's notes.
    #[allow(clippy::too_many_arguments)]
    pub async fn seal_private_contact_request<S>(
        &self,
        sender: &Identity,
        encryption_key_id: KeyID,
        encryption_secret_key: &SecretKey,
        signer: &S,
        recipient: &Identity,
        recipient_key_id: KeyID,
        request: &PrivateContactRequest,
    ) -> Result<BinaryData, Error>
    where
        S: Signer<IdentityPublicKey> + ?Sized,
    {
        let dashpay_contract = self.fetch_private_contact_request_contract().await?;
        let signing_key = private_contact_request_signing_keys(&dashpay_contract, sender)?
            .into_iter()
            .find(|key| signer.can_sign_with(key))
            .ok_or_else(|| {
                Error::Generic(format!(
                    "the signer holds no key of {} that signs a private contact request: an \
                     enabled AUTHENTICATION key at HIGH or CRITICAL, bound to no other contract",
                    sender.id()
                ))
            })?;
        let signing_data = private_contact_request_signing_data(
            &dashpay_contract,
            sender,
            encryption_key_id,
            recipient,
            recipient_key_id,
            request,
            self.version(),
        )?;
        let signature =
            private_contact_request_signature(signer, signing_key, &signing_data).await?;
        // A seed whose hash is no secret key, a chance of about 2^-128, fails the seal, and a
        // retry draws another.
        let mut esk_seed = [0u8; 32];
        StdRng::try_from_os_rng()
            .map_err(|error| {
                Error::Generic(format!(
                    "private contact request: the OS RNG failed: {error}"
                ))
            })?
            .fill_bytes(&mut esk_seed);
        let sealed = seal_private_contact_request(
            &dashpay_contract,
            &PrivateContactRequestSender {
                identity: sender,
                encryption_key_id,
                encryption_secret_key,
                signing_key_id: signing_key.id(),
                signature: &signature,
            },
            recipient,
            recipient_key_id,
            request,
            &esk_seed,
            self.version(),
        );
        esk_seed.zeroize();
        sealed
    }

    /// The DashPay contract, with its `privateContactRequest` type, as the context provider
    /// holds it ([`Sdk::fetch_private_document_contract`]): a page's proof is verified against
    /// the contract the provider gives. The network's protocol version is established as 15 or
    /// later first, as an unpinned SDK starts below it and would otherwise resolve a DashPay
    /// without the type.
    async fn fetch_private_contact_request_contract(&self) -> Result<Arc<DataContract>, Error> {
        self.require_protocol_version(
            PRIVATE_DOCUMENT_CREATE_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
            "DashPay private contact requests",
        )
        .await?;
        let dashpay_id = self.get_dashpay_contract_id()?;
        self.fetch_private_document_contract(dashpay_id, PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME)
            .await
    }

    /// Fetches the identities `needed` lists ([`private_contact_request_identities`]), a few at
    /// a time, each proved at or after the height listed: one proved earlier, which a node may
    /// serve within the SDK's height tolerance, is fetched once more, and fails the fetch if it
    /// still is.
    pub async fn fetch_private_contact_request_identities(
        &self,
        needed: &BTreeMap<Identifier, u64>,
    ) -> Result<BTreeMap<Identifier, ProvedIdentity>, Error> {
        fetch_proved_identities(needed, |identity_id| async move {
            let (identity, metadata) =
                Identity::fetch_with_metadata(self, identity_id, None).await?;
            Ok(ProvedIdentity {
                identity,
                proved_at_height: metadata.height,
            })
        })
        .await
    }

    /// Reads the page of DashPay's `privateContactRequest` store from `start_position` and
    /// returns the requests it holds for the wallet whose `keys` are given: those to its
    /// identities and those its identities sent, each checked against the identities' keys as
    /// [`private_contact_requests_from_scan`] says. The identities are fetched once each, a few
    /// at a time, each proved at or after the entries it checks.
    ///
    /// A wallet keeps [`PrivateContactRequestScan::next_position`] as its cursor once it has
    /// stored what the page held, and scans again from it until
    /// [`PrivateContactRequestScan::is_complete`]. A failed fetch fails the scan, to be
    /// retried from the same cursor.
    ///
    /// The identities are fetched right after the page, which tells the serving node which
    /// identities the wallet looked up (see [`private_contact_request_identities`]). A wallet
    /// that cares calls [`Sdk::scan_private_document_page`] and
    /// [`private_contact_requests_from_scan`] itself, with identities it already holds.
    pub async fn scan_private_contact_requests(
        &self,
        start_position: u64,
        keys: &PrivateDocumentScanKeys,
    ) -> Result<PrivateContactRequestScan, Error> {
        let dashpay_contract = self.fetch_private_contact_request_contract().await?;
        let scan = self
            .scan_private_document_page(
                dashpay_contract.id(),
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                start_position,
                keys,
            )
            .await?;
        let identities = self
            .fetch_private_contact_request_identities(&private_contact_request_identities(&scan))
            .await?;
        private_contact_requests_from_scan(&dashpay_contract, scan, &identities, self.version())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::private_document_entries::{PrivateDocumentEntries, PrivateDocumentEntry};
    use dpp::address_funds::AddressWitness;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::DocumentPropertyType;
    use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::IdentityV0;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::ProtocolError;
    use platform_encryption::sign_private_document_signing_data;
    use rs_dapi_client::CanRetry;

    const SENDER_ID: [u8; 32] = [0xA0; 32];
    const RECIPIENT_ID: [u8; 32] = [0xB0; 32];
    /// The time of the block the test entries are appended in.
    const BLOCK_TIME_MS: u64 = 1_700_000_000_000;
    /// The height the test identities are proved at, after every test entry's block.
    const PROVED_AT_HEIGHT: u64 = 1_000;

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
            // Signing keys sign at HIGH, which DashPay's types require.
            security_level: if purpose == Purpose::AUTHENTICATION {
                SecurityLevel::HIGH
            } else {
                SecurityLevel::MEDIUM
            },
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

    /// `identity` with `key` added, or replacing its key of the same id.
    fn with_key(mut identity: Identity, key: IdentityPublicKey) -> Identity {
        identity.add_public_key(key);
        identity
    }

    /// `identity` with its key `key_id` bound by `bounds`.
    fn with_bounds(identity: Identity, key_id: KeyID, bounds: Option<ContractBounds>) -> Identity {
        let mut key = identity
            .get_public_key_by_id(key_id)
            .expect("the identity has the key")
            .clone();
        match &mut key {
            IdentityPublicKey::V0(key) => key.contract_bounds = bounds,
            IdentityPublicKey::V1(key) => key.contract_bounds = bounds,
        }
        with_key(identity, key)
    }

    /// The sender, whose signing key 0 is `secret_key(9)` and whose encryption key 1 is
    /// `secret_key(1)`, disabled at `disabled_at`.
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

    /// The signature by `signing_secret` of `request` from the sender's encryption key 1 to the
    /// recipient's key 2, as an identity signer makes it.
    fn signature(
        contract: &DataContract,
        sender: &Identity,
        request: &PrivateContactRequest,
        signing_secret: &SecretKey,
    ) -> [u8; 65] {
        sign_private_document_signing_data(
            &private_contact_request_signing_data(
                contract,
                sender,
                1,
                &recipient_identity(),
                2,
                request,
                PlatformVersion::latest(),
            )
            .expect("the keys check"),
            signing_secret,
        )
    }

    /// `request` from the sender's encryption key 1, signed by its key 0, to the recipient's
    /// key 2.
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
                signing_key_id: 0,
                signature: &signature(contract, sender, request, &secret_key(9)),
            },
            &recipient_identity(),
            2,
            request,
            &[seed; 32],
            PlatformVersion::latest(),
        )
        .expect("the request seals")
        .to_vec()
    }

    /// An entry of `payload` to `recipient_key` of `recipient_id`, from `sender`, sealed with
    /// the sender's outgoing viewing key: one no wallet of these tests' would build.
    fn seal_raw(
        contract: &DataContract,
        recipient_id: [u8; 32],
        recipient_key: &SecretKey,
        sender: &PrivateDocumentSender,
        payload: &[u8],
    ) -> Vec<u8> {
        seal_private_document_entry(
            &PrivateDocumentTarget {
                contract_id: contract.id().to_buffer(),
                document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
            },
            589,
            &PrivateDocumentRecipient {
                identity_id: recipient_id,
                public_key: &PublicKey::from_secret_key(recipient_key),
            },
            sender,
            payload,
            &private_contact_request_outgoing_viewing_key(&secret_key(1)),
            &[9; 32],
        )
        .expect("the payload fits")
    }

    fn payload(contract: &DataContract, request: &PrivateContactRequest) -> Vec<u8> {
        encode_private_document_payload(
            private_contact_request_type(contract).expect("DashPay v3"),
            request.to_properties(),
        )
        .expect("the payload encodes")
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

    /// The recipient's keys alone.
    fn recipient_keys() -> PrivateDocumentScanKeys {
        PrivateDocumentScanKeys::new()
            .with_recipient(Identifier::from(RECIPIENT_ID), &[(2, secret_key(2))])
    }

    /// The recipient's keys, and the sender's outgoing viewing key, of its encryption key 1.
    fn scan_keys() -> PrivateDocumentScanKeys {
        recipient_keys().with_sender(
            Identifier::from(SENDER_ID),
            1,
            private_contact_request_outgoing_viewing_key(&secret_key(1)),
        )
    }

    /// Reads `entries` with `keys`, fetching the identities from `identities`: one not there
    /// does not exist.
    fn read(
        contract: &DataContract,
        entries: Vec<Vec<u8>>,
        keys: &PrivateDocumentScanKeys,
        identities: &[Identity],
    ) -> PrivateContactRequestScan {
        let target = PrivateDocumentTarget {
            contract_id: contract.id().to_buffer(),
            document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
        };
        let scan = keys.scan_page(&target, 0, &page(entries));
        let fetched = private_contact_request_identities(&scan)
            .into_keys()
            .map(|identity_id| {
                let identity = identities
                    .iter()
                    .find(|identity| identity.id() == identity_id)
                    .cloned();
                (
                    identity_id,
                    ProvedIdentity {
                        identity,
                        proved_at_height: PROVED_AT_HEIGHT,
                    },
                )
            })
            .collect();
        private_contact_requests_from_scan(contract, scan, &fetched, PlatformVersion::latest())
            .expect("DashPay declares private contact requests")
    }

    fn reasons(scan: PrivateContactRequestScan) -> Vec<PrivateContactRequestRejection> {
        scan.rejected
            .into_iter()
            .map(|rejected| rejected.reason)
            .collect()
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
            let payload = encode_private_document_payload(document_type, request.to_properties())
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
            // The bound is in bytes: 32 two-byte characters are 64 bytes.
            PrivateContactRequest {
                account_label: Some(
                    "é".repeat(PRIVATE_CONTACT_REQUEST_ACCOUNT_LABEL_MAX_BYTES.div_ceil(2)),
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
        // dpp's document validation refuses them, when sealing and when reading.
        let contract = dashpay();
        let sender = sender_identity(None);
        for request in &out_of_bounds {
            assert!(
                private_contact_request_signing_data(
                    &contract,
                    &sender,
                    1,
                    &recipient_identity(),
                    2,
                    request,
                    PlatformVersion::latest(),
                )
                .is_err(),
                "{request:?} is refused when sealing"
            );
            let payload = payload(&contract, request);
            let signing_data = private_document_signing_data(
                &PrivateDocumentTarget {
                    contract_id: contract.id().to_buffer(),
                    document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                },
                &SENDER_ID,
                1,
                &PrivateDocumentRecipient {
                    identity_id: RECIPIENT_ID,
                    public_key: &PublicKey::from_secret_key(&secret_key(2)),
                },
                &payload,
            );
            let signature = sign_private_document_signing_data(&signing_data, &secret_key(9));
            let entry = seal_raw(
                &contract,
                RECIPIENT_ID,
                &secret_key(2),
                &PrivateDocumentSender::Signature {
                    identity_id: SENDER_ID,
                    key_id: 1,
                    signature: &signature,
                },
                &payload,
            );
            assert!(
                matches!(
                    reasons(read(
                        &contract,
                        vec![entry],
                        &recipient_keys(),
                        &[sender.clone(), recipient_identity()],
                    ))
                    .as_slice(),
                    [PrivateContactRequestRejection::InvalidRequest(_)]
                ),
                "{request:?} is refused when reading"
            );
        }

        let properties = || smallest_request(1).to_properties();
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
                "a label of the wrong type",
                with(ACCOUNT_LABEL, Value::U32(1)),
            ),
            (
                "an unknown property",
                with("note", Value::Text("hi".to_string())),
            ),
            ("an unknown property set to null", with("note", Value::Null)),
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

        let scan = read(
            &contract,
            vec![entry],
            &scan_keys(),
            &[sender, recipient_identity()],
        );
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
        assert_eq!(sent.sender_key_id, 1);
        assert_eq!(sent.recipient_id, Identifier::from(RECIPIENT_ID));
        assert_eq!(
            sent.recipient_public_key,
            PublicKey::from_secret_key(&secret_key(2))
        );
        assert_eq!(sent.commitment, received.commitment);
        assert_eq!(sent.request, largest_request());
        assert!(scan.is_complete());
    }

    /// A received request is refused when its sender does not exist; when the encryption key it
    /// names is not an enabled encryption key of the sender; or when no key of the sender that
    /// may sign DashPay's documents, enabled then, signed it. A key disabled after the entry was
    /// appended was enabled when it was written.
    #[test]
    fn should_refuse_a_request_whose_sender_does_not_check() {
        let contract = dashpay();
        let entry = seal(&contract, &sender_identity(None), &smallest_request(1), 1);
        let reasons_with = |sender: Option<Identity>| {
            let mut identities = vec![recipient_identity()];
            identities.extend(sender);
            reasons(read(
                &contract,
                vec![entry.clone()],
                &recipient_keys(),
                &identities,
            ))
        };
        let not_usable = |reasons: Vec<PrivateContactRequestRejection>| {
            matches!(
                reasons.as_slice(),
                [PrivateContactRequestRejection::SenderKeyNotUsable(_)]
            )
        };
        let signing_key = |security_level, disabled_at| {
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: 0,
                purpose: Purpose::AUTHENTICATION,
                security_level,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(
                    PublicKey::from_secret_key(&secret_key(9))
                        .serialize()
                        .to_vec(),
                ),
                disabled_at,
            })
        };

        assert_eq!(reasons_with(Some(sender_identity(None))), vec![]);
        assert_eq!(
            reasons_with(None),
            vec![PrivateContactRequestRejection::SenderNotFound]
        );

        // The encryption key named, disabled before and after the entry.
        assert!(not_usable(reasons_with(Some(sender_identity(Some(
            BLOCK_TIME_MS
        ))))));
        assert_eq!(
            reasons_with(Some(sender_identity(Some(BLOCK_TIME_MS + 1)))),
            vec![]
        );
        // Key 1 is not an encryption key.
        assert!(not_usable(reasons_with(Some(with_key(
            sender_identity(None),
            key(
                1,
                Purpose::AUTHENTICATION,
                KeyType::ECDSA_SECP256K1,
                &secret_key(1),
                None,
            ),
        )))));

        // The signing key, disabled before and after the entry, and at a level that signs no
        // DashPay document.
        assert!(not_usable(reasons_with(Some(with_key(
            sender_identity(None),
            signing_key(SecurityLevel::HIGH, Some(BLOCK_TIME_MS)),
        )))));
        assert_eq!(
            reasons_with(Some(with_key(
                sender_identity(None),
                signing_key(SecurityLevel::HIGH, Some(BLOCK_TIME_MS + 1)),
            ))),
            vec![]
        );
        assert!(not_usable(reasons_with(Some(with_key(
            sender_identity(None),
            signing_key(SecurityLevel::MEDIUM, None),
        )))));
        assert!(not_usable(reasons_with(Some(with_key(
            sender_identity(None),
            signing_key(SecurityLevel::MASTER, None),
        )))));
        // The signing key's public key under another purpose is no signing key.
        assert!(not_usable(reasons_with(Some(with_key(
            sender_identity(None),
            key(
                0,
                Purpose::ENCRYPTION,
                KeyType::ECDSA_SECP256K1,
                &secret_key(9),
                None,
            ),
        )))));
        // The identity holds no key that signed it.
        assert_eq!(
            reasons_with(Some(with_key(
                sender_identity(None),
                key(
                    0,
                    Purpose::AUTHENTICATION,
                    KeyType::ECDSA_SECP256K1,
                    &secret_key(5),
                    None,
                ),
            ))),
            vec![PrivateContactRequestRejection::SenderNotAuthenticated]
        );
        // A hash160 signing key signs too: the signature recovers its key.
        assert_eq!(
            reasons_with(Some(with_key(
                sender_identity(None),
                IdentityPublicKey::V0(IdentityPublicKeyV0 {
                    id: 0,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::CRITICAL,
                    contract_bounds: None,
                    key_type: KeyType::ECDSA_HASH160,
                    read_only: false,
                    data: BinaryData::new(
                        ripemd160_sha256(&PublicKey::from_secret_key(&secret_key(9)).serialize())
                            .to_vec(),
                    ),
                    disabled_at: None,
                }),
            ))),
            vec![]
        );
    }

    /// A stolen recipient key forges nothing: an entry with a deniable MAC, which the
    /// recipient's key computes, is refused, and so is one the recipient signed itself naming
    /// the sender.
    #[test]
    fn should_refuse_an_entry_a_stolen_recipient_key_could_have_written() {
        let contract = dashpay();
        let request = smallest_request(1);
        let mac = seal_raw(
            &contract,
            RECIPIENT_ID,
            &secret_key(2),
            &PrivateDocumentSender::DeniableMac {
                identity_id: SENDER_ID,
                key_id: 1,
                secret_key: &secret_key(1),
            },
            &payload(&contract, &request),
        );
        let forged_signature =
            signature(&contract, &sender_identity(None), &request, &secret_key(2));
        let forged = seal_raw(
            &contract,
            RECIPIENT_ID,
            &secret_key(2),
            &PrivateDocumentSender::Signature {
                identity_id: SENDER_ID,
                key_id: 1,
                signature: &forged_signature,
            },
            &payload(&contract, &request),
        );
        assert_eq!(
            reasons(read(
                &contract,
                vec![mac, forged],
                &recipient_keys(),
                &[sender_identity(None), recipient_identity()],
            )),
            vec![
                PrivateContactRequestRejection::NotSigned,
                PrivateContactRequestRejection::SenderNotAuthenticated,
            ]
        );
    }

    /// A request is refused when the recipient key it is encrypted to was disabled before it
    /// was appended, or when the recipient identity is not given to check it.
    #[test]
    fn should_refuse_a_request_to_a_recipient_key_disabled_before_it() {
        let contract = dashpay();
        let entry = seal(&contract, &sender_identity(None), &smallest_request(1), 1);
        let reasons_with = |recipient: Option<Identity>| {
            let mut identities = vec![sender_identity(None)];
            identities.extend(recipient);
            reasons(read(
                &contract,
                vec![entry.clone()],
                &recipient_keys(),
                &identities,
            ))
        };
        let disabled_at = |disabled_at| {
            with_key(
                recipient_identity(),
                key(
                    2,
                    Purpose::DECRYPTION,
                    KeyType::ECDSA_SECP256K1,
                    &secret_key(2),
                    Some(disabled_at),
                ),
            )
        };
        assert!(matches!(
            reasons_with(Some(disabled_at(BLOCK_TIME_MS))).as_slice(),
            [PrivateContactRequestRejection::RecipientKeyNotUsable(_)]
        ));
        assert_eq!(reasons_with(Some(disabled_at(BLOCK_TIME_MS + 1))), vec![]);
        assert!(matches!(
            reasons_with(None).as_slice(),
            [PrivateContactRequestRejection::RecipientKeyNotUsable(_)]
        ));
    }

    /// A request is read only through a recipient key sealing would have encrypted to: one
    /// whose record is an AUTHENTICATION or TRANSFER key holding the point it opened with, or
    /// an ECDSA_HASH160 record of that point, is refused.
    #[test]
    fn should_refuse_a_request_through_a_key_it_could_not_be_sealed_to() {
        let contract = dashpay();
        let entry = seal(&contract, &sender_identity(None), &smallest_request(1), 1);
        let read_with = |recipient_key: IdentityPublicKey| {
            reasons(read(
                &contract,
                vec![entry.clone()],
                &recipient_keys(),
                &[
                    sender_identity(None),
                    with_key(recipient_identity(), recipient_key),
                ],
            ))
        };
        assert_eq!(
            read_with(key(
                2,
                Purpose::DECRYPTION,
                KeyType::ECDSA_SECP256K1,
                &secret_key(2),
                None
            )),
            vec![],
            "the key it was sealed to"
        );
        for purpose in [Purpose::AUTHENTICATION, Purpose::TRANSFER] {
            assert!(
                matches!(
                    read_with(key(
                        2,
                        purpose,
                        KeyType::ECDSA_SECP256K1,
                        &secret_key(2),
                        None
                    ))
                    .as_slice(),
                    [PrivateContactRequestRejection::RecipientKeyNotUsable(_)]
                ),
                "{purpose}"
            );
        }
        let hash160 = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 2,
            purpose: Purpose::DECRYPTION,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: false,
            data: BinaryData::new(
                ripemd160_sha256(&PublicKey::from_secret_key(&secret_key(2)).serialize()).to_vec(),
            ),
            disabled_at: None,
        });
        assert!(matches!(
            read_with(hash160).as_slice(),
            [PrivateContactRequestRejection::RecipientKeyNotUsable(_)]
        ));
    }

    /// An entry the sender's outgoing viewing key recovers is the sender's only when it names
    /// the identity and key that key is derived from and one of the identity's signing keys
    /// signed it: someone holding the encryption key could build the others.
    #[test]
    fn should_refuse_a_sent_entry_the_identity_did_not_write() {
        let contract = dashpay();
        let sender = sender_identity(None);
        let request = smallest_request(1);
        let signed_by = |identity_id: [u8; 32], key_id: KeyID, signing_secret: &SecretKey| {
            let signing_data = private_document_signing_data(
                &PrivateDocumentTarget {
                    contract_id: contract.id().to_buffer(),
                    document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                },
                &identity_id,
                key_id,
                &PrivateDocumentRecipient {
                    identity_id: [0xD0; 32],
                    public_key: &PublicKey::from_secret_key(&secret_key(40)),
                },
                &payload(&contract, &request),
            );
            let signature = sign_private_document_signing_data(&signing_data, signing_secret);
            seal_raw(
                &contract,
                [0xD0; 32],
                &secret_key(40),
                &PrivateDocumentSender::Signature {
                    identity_id,
                    key_id,
                    signature: &signature,
                },
                &payload(&contract, &request),
            )
        };
        let entries = vec![
            signed_by(SENDER_ID, 1, &secret_key(9)),
            // Another key than the outgoing viewing key's.
            signed_by(SENDER_ID, 3, &secret_key(9)),
            // Another identity.
            signed_by([0xC0; 32], 1, &secret_key(9)),
            // Signed by no key of the sender.
            signed_by(SENDER_ID, 1, &secret_key(5)),
        ];
        let scan = read(&contract, entries.clone(), &scan_keys(), &[sender]);
        assert_eq!(
            scan.sent
                .iter()
                .map(|sent| (sent.position, sent.recipient_id))
                .collect::<Vec<_>>(),
            vec![(0, Identifier::from([0xD0; 32]))]
        );
        assert_eq!(
            reasons(scan),
            vec![PrivateContactRequestRejection::SenderNotAuthenticated; 3]
        );
        // The encryption key was disabled before the entry: a thief wrote it with the key.
        assert!(matches!(
            reasons(read(
                &contract,
                entries[..1].to_vec(),
                &scan_keys(),
                &[sender_identity(Some(BLOCK_TIME_MS))],
            ))
            .as_slice(),
            [PrivateContactRequestRejection::SenderKeyNotUsable(_)]
        ));
    }

    /// The sender is checked before the payload is read: an entry from no known sender is
    /// refused as such, whatever it holds.
    #[test]
    fn should_check_the_sender_before_reading_the_payload() {
        let contract = dashpay();
        let garbage_signature = sign_private_document_signing_data(&[1; 32], &secret_key(5));
        let entry = seal_raw(
            &contract,
            RECIPIENT_ID,
            &secret_key(2),
            &PrivateDocumentSender::Signature {
                identity_id: [0xC0; 32],
                key_id: 1,
                signature: &garbage_signature,
            },
            &[0xFF; 40],
        );
        assert_eq!(
            reasons(read(
                &contract,
                vec![entry],
                &recipient_keys(),
                &[recipient_identity()],
            )),
            vec![PrivateContactRequestRejection::SenderNotFound]
        );
    }

    /// An anonymous entry is no contact request, and a second request from the same sender for
    /// the same account reference is a duplicate of the first, which is kept.
    #[test]
    fn should_refuse_an_anonymous_entry_and_a_duplicate_request() {
        let contract = dashpay();
        let sender = sender_identity(None);
        let anonymous = seal_raw(
            &contract,
            RECIPIENT_ID,
            &secret_key(2),
            &PrivateDocumentSender::Anonymous,
            &payload(&contract, &smallest_request(1)),
        );

        let scan = read(
            &contract,
            vec![
                seal(&contract, &sender, &smallest_request(1), 1),
                anonymous,
                seal(&contract, &sender, &smallest_request(1), 2),
                seal(&contract, &sender, &smallest_request(2), 3),
            ],
            &recipient_keys(),
            &[sender.clone(), recipient_identity()],
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
    /// secret key, signed by an enabled signing key of the sender at a level DashPay's documents
    /// are signed at, and only to an enabled ECDSA decryption or encryption key of the
    /// recipient.
    #[test]
    fn should_refuse_to_seal_with_keys_dashpay_does_not_use() {
        let contract = dashpay();
        let request = smallest_request(1);
        let attempt = |sender: &Identity,
                       encryption_key_id: KeyID,
                       secret: u8,
                       signing_key_id: KeyID,
                       signing_secret: u8,
                       recipient_key_id: KeyID| {
            let signing_data = private_contact_request_signing_data(
                &contract,
                sender,
                encryption_key_id,
                &recipient_identity(),
                recipient_key_id,
                &request,
                PlatformVersion::latest(),
            )?;
            seal_private_contact_request(
                &contract,
                &PrivateContactRequestSender {
                    identity: sender,
                    encryption_key_id,
                    encryption_secret_key: &secret_key(secret),
                    signing_key_id,
                    signature: &sign_private_document_signing_data(
                        &signing_data,
                        &secret_key(signing_secret),
                    ),
                },
                &recipient_identity(),
                recipient_key_id,
                &request,
                &[1; 32],
                PlatformVersion::latest(),
            )
        };
        let sender = sender_identity(None);
        attempt(&sender, 1, 1, 0, 9, 2).expect(
            "the sender's encryption key, signed by its signing key, to the recipient's \
             decryption key",
        );
        for (case, encryption_key_id, secret, signing_key_id, signing_secret, recipient_key_id) in [
            (
                "the sender's authentication key as the encryption key",
                0,
                9,
                0,
                9,
                2,
            ),
            (
                "another secret key than the encryption key's",
                1,
                5,
                0,
                9,
                2,
            ),
            ("an encryption key the identity lacks", 7, 1, 0, 9, 2),
            ("the encryption key as the signing key", 1, 1, 1, 1, 2),
            (
                "a signature by another key than the signing key",
                1,
                1,
                0,
                5,
                2,
            ),
            ("a signing key the identity lacks", 1, 1, 8, 9, 2),
            ("the recipient's authentication key", 1, 1, 0, 9, 0),
            ("the recipient's BLS key", 1, 1, 0, 9, 3),
            ("the recipient's disabled key", 1, 1, 0, 9, 4),
            ("a recipient key the identity lacks", 1, 1, 0, 9, 9),
        ] {
            assert!(
                attempt(
                    &sender,
                    encryption_key_id,
                    secret,
                    signing_key_id,
                    signing_secret,
                    recipient_key_id
                )
                .is_err(),
                "{case}"
            );
        }
        assert!(
            attempt(&sender_identity(Some(1)), 1, 1, 0, 9, 2).is_err(),
            "a disabled encryption key"
        );
        let medium = with_key(
            sender_identity(None),
            IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: 0,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::MEDIUM,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(
                    PublicKey::from_secret_key(&secret_key(9))
                        .serialize()
                        .to_vec(),
                ),
                disabled_at: None,
            }),
        );
        assert!(
            attempt(&medium, 1, 1, 0, 9, 2).is_err(),
            "a MEDIUM signing key"
        );
    }

    /// A signing key bound to a contract signs only inside its bounds: DashPay, or DashPay's
    /// private contact requests. A group bound, whose members a client cannot read as they
    /// were, signs none. The seal and the reader apply the same rule.
    #[test]
    fn should_sign_only_with_a_key_bound_to_dashpay_or_unbound() {
        let contract = dashpay();
        let dashpay_id = contract.id();
        let other_id = Identifier::from([0x77; 32]);
        let bound = |bounds: Option<ContractBounds>| {
            with_key(
                sender_identity(None),
                IdentityPublicKey::V0(IdentityPublicKeyV0 {
                    id: 0,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::HIGH,
                    contract_bounds: bounds,
                    key_type: KeyType::ECDSA_SECP256K1,
                    read_only: false,
                    data: BinaryData::new(
                        PublicKey::from_secret_key(&secret_key(9))
                            .serialize()
                            .to_vec(),
                    ),
                    disabled_at: None,
                }),
            )
        };
        let document_type = |id, name: &str| {
            Some(ContractBounds::SingleContractDocumentType {
                id,
                document_type_name: name.to_string(),
            })
        };
        let entry = seal(&contract, &sender_identity(None), &smallest_request(1), 1);
        for (case, bounds, allowed) in [
            (
                "bound to DashPay",
                Some(ContractBounds::SingleContract { id: dashpay_id }),
                true,
            ),
            (
                "bound to DashPay's private contact requests",
                document_type(dashpay_id, PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME),
                true,
            ),
            (
                "bound to another contract",
                Some(ContractBounds::SingleContract { id: other_id }),
                false,
            ),
            (
                "bound to another DashPay type",
                document_type(dashpay_id, "contactRequest"),
                false,
            ),
            (
                "bound to a contract group",
                Some(ContractBounds::ContractGroup { id: other_id }),
                false,
            ),
        ] {
            let sender = bound(bounds);
            let reasons = reasons(read(
                &contract,
                vec![entry.clone()],
                &recipient_keys(),
                &[sender.clone(), recipient_identity()],
            ));
            let request = smallest_request(1);
            let sealed = seal_private_contact_request(
                &contract,
                &PrivateContactRequestSender {
                    identity: &sender,
                    encryption_key_id: 1,
                    encryption_secret_key: &secret_key(1),
                    signing_key_id: 0,
                    signature: &signature(&contract, &sender, &request, &secret_key(9)),
                },
                &recipient_identity(),
                2,
                &request,
                &[1; 32],
                PlatformVersion::latest(),
            );
            let signing_keys = private_contact_request_signing_keys(&contract, &sender)
                .expect("DashPay v3")
                .len();
            if allowed {
                assert_eq!(reasons, vec![], "{case}: read");
                assert!(sealed.is_ok(), "{case}: seal");
                assert_eq!(signing_keys, 1, "{case}: listed");
            } else {
                assert!(
                    matches!(
                        reasons.as_slice(),
                        [PrivateContactRequestRejection::SenderKeyNotUsable(_)]
                    ),
                    "{case}: read {reasons:?}"
                );
                assert!(sealed.is_err(), "{case}: seal");
                assert_eq!(signing_keys, 0, "{case}: not listed");
            }
        }
    }

    /// An ENCRYPTION or DECRYPTION key bound to a contract is used only inside its bounds:
    /// DashPay, or DashPay's contact requests, public or private, as DashPay asks for its
    /// contact request keys to be bound to `contactRequest`. A group bound, whose members a
    /// client does not read, is used for none. The seal and the reader apply the same rule to
    /// the sender's key and the recipient's.
    #[test]
    fn should_encrypt_only_with_keys_bound_to_dashpay_contact_requests_or_unbound() {
        let contract = dashpay();
        let dashpay_id = contract.id();
        let other_id = Identifier::from([0x77; 32]);
        let document_type = |id, name: &str| {
            Some(ContractBounds::SingleContractDocumentType {
                id,
                document_type_name: name.to_string(),
            })
        };
        let request = smallest_request(1);
        let signed = signature(&contract, &sender_identity(None), &request, &secret_key(9));
        let entry = seal(&contract, &sender_identity(None), &request, 1);
        let seal_between = |sender: &Identity, recipient: &Identity| {
            seal_private_contact_request(
                &contract,
                &PrivateContactRequestSender {
                    identity: sender,
                    encryption_key_id: 1,
                    encryption_secret_key: &secret_key(1),
                    signing_key_id: 0,
                    signature: &signed,
                },
                recipient,
                2,
                &request,
                &[1; 32],
                PlatformVersion::latest(),
            )
        };
        for (case, bounds, allowed) in [
            ("unbound", None, true),
            (
                "bound to DashPay",
                Some(ContractBounds::SingleContract { id: dashpay_id }),
                true,
            ),
            (
                "bound to DashPay's contact requests",
                document_type(dashpay_id, "contactRequest"),
                true,
            ),
            (
                "bound to DashPay's private contact requests",
                document_type(dashpay_id, PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME),
                true,
            ),
            (
                "bound to another contract",
                Some(ContractBounds::SingleContract { id: other_id }),
                false,
            ),
            (
                "bound to another DashPay type",
                document_type(dashpay_id, "profile"),
                false,
            ),
            (
                "bound to another contract's contactRequest",
                document_type(other_id, "contactRequest"),
                false,
            ),
            (
                "bound to a contract group",
                Some(ContractBounds::ContractGroup { id: other_id }),
                false,
            ),
        ] {
            let sender = with_bounds(sender_identity(None), 1, bounds.clone());
            let recipient = with_bounds(recipient_identity(), 2, bounds);
            let sealed_from = seal_between(&sender, &recipient_identity());
            let sealed_to = seal_between(&sender_identity(None), &recipient);
            let read_from = reasons(read(
                &contract,
                vec![entry.clone()],
                &recipient_keys(),
                &[sender, recipient_identity()],
            ));
            let read_to = reasons(read(
                &contract,
                vec![entry.clone()],
                &recipient_keys(),
                &[sender_identity(None), recipient],
            ));
            if allowed {
                assert!(sealed_from.is_ok(), "{case}: sealed from {sealed_from:?}");
                assert!(sealed_to.is_ok(), "{case}: sealed to {sealed_to:?}");
                assert_eq!(read_from, vec![], "{case}: read from");
                assert_eq!(read_to, vec![], "{case}: read to");
            } else {
                assert!(sealed_from.is_err(), "{case}: sealed from");
                assert!(sealed_to.is_err(), "{case}: sealed to");
                assert!(
                    matches!(
                        read_from.as_slice(),
                        [PrivateContactRequestRejection::SenderKeyNotUsable(_)]
                    ),
                    "{case}: read from {read_from:?}"
                );
                assert!(
                    matches!(
                        read_to.as_slice(),
                        [PrivateContactRequestRejection::RecipientKeyNotUsable(_)]
                    ),
                    "{case}: read to {read_to:?}"
                );
            }
        }
    }

    /// A scan of another store's page is refused, received and sent entries alike, rather
    /// than read as DashPay requests: here a store of another contract with DashPay's types.
    /// So is one holding no entry the wallet's keys open, whose cursor is that store's, of
    /// another contract or of another of DashPay's types; DashPay's own empty page reads.
    #[test]
    fn should_refuse_a_scan_of_another_store() {
        let dashpay_contract = dashpay();
        let mut other = dashpay();
        other.set_id(Identifier::from([0x55; 32]));
        let entry = seal(&other, &sender_identity(None), &smallest_request(1), 1);
        let target = PrivateDocumentTarget {
            contract_id: other.id().to_buffer(),
            document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
        };
        let sender_keys = PrivateDocumentScanKeys::new().with_sender(
            Identifier::from(SENDER_ID),
            1,
            private_contact_request_outgoing_viewing_key(&secret_key(1)),
        );
        for (case, keys) in [("received", recipient_keys()), ("sent", sender_keys)] {
            let scan = keys.scan_page(&target, 0, &page(vec![entry.clone()]));
            assert_eq!(scan.received.len() + scan.sent.len(), 1, "{case}: it opens");
            let identities = private_contact_request_identities(&scan)
                .into_keys()
                .map(|identity_id| {
                    let identity = [sender_identity(None), recipient_identity()]
                        .into_iter()
                        .find(|identity| identity.id() == identity_id);
                    (
                        identity_id,
                        ProvedIdentity {
                            identity,
                            proved_at_height: PROVED_AT_HEIGHT,
                        },
                    )
                })
                .collect();
            let error = private_contact_requests_from_scan(
                &dashpay_contract,
                scan,
                &identities,
                PlatformVersion::latest(),
            )
            .expect_err("another store")
            .to_string();
            assert!(error.contains("not DashPay's"), "{case}: {error}");
        }

        let dashpay_id = dashpay_contract.id().to_buffer();
        for (contract_id, document_type_name) in [
            (
                other.id().to_buffer(),
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
            ),
            (dashpay_id, "contactInfo"),
        ] {
            let target = PrivateDocumentTarget {
                contract_id,
                document_type_name,
            };
            let scan = recipient_keys().scan_page(&target, 0, &page(vec![]));
            let error = private_contact_requests_from_scan(
                &dashpay_contract,
                scan,
                &BTreeMap::new(),
                PlatformVersion::latest(),
            )
            .expect_err("another store's cursor")
            .to_string();
            assert!(
                error.contains("not DashPay's"),
                "{document_type_name}: {error}"
            );
        }
        let own = PrivateDocumentTarget {
            contract_id: dashpay_id,
            document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
        };
        assert!(private_contact_requests_from_scan(
            &dashpay_contract,
            recipient_keys().scan_page(&own, 0, &page(vec![])),
            &BTreeMap::new(),
            PlatformVersion::latest(),
        )
        .is_ok());
    }

    /// A contract whose `privateContactRequest` type is an ordinary public type holds no
    /// private store: a scan of it is refused, even an empty one, rather than handing back its
    /// cursor.
    #[test]
    fn should_refuse_a_scan_of_a_contract_whose_type_is_not_private() {
        use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
        use dpp::platform_value::platform_value;
        use dpp::platform_value::string_encoding::Encoding;

        let id = Identifier::from([0x5E; 32]);
        let public = DataContract::from_value(
            platform_value!({
                "$formatVersion": "1",
                "id": id.to_string(Encoding::Base58),
                "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
                "version": 1,
                "documentSchemas": {
                    PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME: {
                        "type": "object",
                        "properties": {
                            "accountReference": {
                                "type": "integer", "minimum": 0, "position": 0
                            }
                        },
                        "additionalProperties": false
                    }
                }
            }),
            true,
            PlatformVersion::latest(),
        )
        .expect("the contract registers");
        let target = PrivateDocumentTarget {
            contract_id: id.to_buffer(),
            document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
        };
        let error = private_contact_requests_from_scan(
            &public,
            recipient_keys().scan_page(&target, 0, &page(vec![])),
            &BTreeMap::new(),
            PlatformVersion::latest(),
        )
        .expect_err("a public type")
        .to_string();
        assert!(error.contains("is not a private type"), "{error}");
    }

    /// Two recovered sent requests to the same recipient for the same account reference are
    /// one request: the first is kept. A request the wallet both sent and received is kept on
    /// both sides.
    #[test]
    fn should_keep_the_first_of_two_sent_requests_for_one_account_reference() {
        let contract = dashpay();
        let sender = sender_identity(None);
        let entries = vec![
            seal(&contract, &sender, &smallest_request(1), 1),
            seal(&contract, &sender, &smallest_request(1), 2),
        ];
        let scan = read(
            &contract,
            entries,
            &scan_keys(),
            &[sender, recipient_identity()],
        );
        assert_eq!(
            scan.sent
                .iter()
                .map(|sent| sent.position)
                .collect::<Vec<_>>(),
            vec![0]
        );
        assert_eq!(
            scan.received
                .iter()
                .map(|received| received.position)
                .collect::<Vec<_>>(),
            vec![0]
        );
        assert_eq!(
            reasons(scan),
            vec![
                PrivateContactRequestRejection::Duplicate,
                PrivateContactRequestRejection::Duplicate,
            ]
        );
    }

    /// The identities a scan needs are those its signed entries name, each with the latest
    /// block it checks; a reading against one proved before that block, or without one, fails
    /// rather than judging the entry.
    #[test]
    fn should_read_only_against_identities_proved_after_the_entries() {
        let contract = dashpay();
        let sender = sender_identity(None);
        let request = smallest_request(1);
        let mac = seal_raw(
            &contract,
            RECIPIENT_ID,
            &secret_key(2),
            &PrivateDocumentSender::DeniableMac {
                identity_id: [0xC0; 32],
                key_id: 1,
                secret_key: &secret_key(1),
            },
            &payload(&contract, &request),
        );
        let target = PrivateDocumentTarget {
            contract_id: contract.id().to_buffer(),
            document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
        };
        let scan = scan_keys().scan_page(
            &target,
            0,
            &page(vec![seal(&contract, &sender, &request, 1), mac]),
        );
        // Entry 0 is in block 10; the MAC entry, naming 0xC0, needs no identity.
        assert_eq!(
            private_contact_request_identities(&scan),
            BTreeMap::from([
                (Identifier::from(SENDER_ID), 10),
                (Identifier::from(RECIPIENT_ID), 10),
            ])
        );
        let proved = |height| {
            BTreeMap::from([
                (
                    Identifier::from(SENDER_ID),
                    ProvedIdentity {
                        identity: Some(sender.clone()),
                        proved_at_height: height,
                    },
                ),
                (
                    Identifier::from(RECIPIENT_ID),
                    ProvedIdentity {
                        identity: Some(recipient_identity()),
                        proved_at_height: 10,
                    },
                ),
            ])
        };
        assert!(private_contact_requests_from_scan(
            &contract,
            scan.clone(),
            &proved(10),
            PlatformVersion::latest()
        )
        .is_ok());
        let stale = private_contact_requests_from_scan(
            &contract,
            scan.clone(),
            &proved(9),
            PlatformVersion::latest(),
        )
        .expect_err("a sender proved a block before its entry");
        assert!(
            matches!(
                stale,
                Error::StaleNode(StaleNodeError::Height {
                    expected_height: 10,
                    received_height: 9,
                    ..
                })
            ),
            "{stale:?}"
        );
        assert!(stale.can_retry(), "fetching it again may help");
        let mut without_recipient = proved(10);
        without_recipient.remove(&Identifier::from(RECIPIENT_ID));
        let missing = private_contact_requests_from_scan(
            &contract,
            scan,
            &without_recipient,
            PlatformVersion::latest(),
        )
        .expect_err("an identity not given");
        assert!(!missing.can_retry(), "the caller's mistake: {missing:?}");
    }

    /// Below protocol version 15 the SDK refuses private contact requests before resolving
    /// DashPay, whose older versions have no private type.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_refuse_private_contact_requests_below_protocol_version_15() {
        let sdk: Sdk = crate::SdkBuilder::new_mock()
            .with_version(PlatformVersion::get(14).expect("protocol version 14"))
            .build()
            .expect("a mock sdk");
        let error = sdk
            .scan_private_contact_requests(0, &PrivateDocumentScanKeys::new())
            .await
            .expect_err("protocol version 14");
        assert!(
            error
                .to_string()
                .contains("exists from protocol version 15"),
            "{error}"
        );
        let signer = TransitionSigner(BTreeMap::new());
        let error = sdk
            .seal_private_contact_request(
                &sender_identity(None),
                1,
                &secret_key(1),
                &signer,
                &recipient_identity(),
                2,
                &smallest_request(1),
            )
            .await
            .expect_err("protocol version 14");
        assert!(
            error
                .to_string()
                .contains("exists from protocol version 15"),
            "{error}"
        );
    }

    /// A signing key with an expiry signs entries appended before it, not at or after it, and
    /// is not offered once expired.
    #[test]
    fn should_refuse_a_signing_key_expired_when_the_entry_was_appended() {
        let contract = dashpay();
        let entry = seal(&contract, &sender_identity(None), &smallest_request(1), 1);
        let expiring = |expires_at| {
            with_key(
                sender_identity(None),
                IdentityPublicKey::V1(
                    dpp::identity::identity_public_key::v1::IdentityPublicKeyV1 {
                        id: 0,
                        purpose: Purpose::AUTHENTICATION,
                        security_level: SecurityLevel::HIGH,
                        contract_bounds: None,
                        key_type: KeyType::ECDSA_SECP256K1,
                        read_only: false,
                        data: BinaryData::new(
                            PublicKey::from_secret_key(&secret_key(9))
                                .serialize()
                                .to_vec(),
                        ),
                        disabled_at: None,
                        total_budget: None,
                        expires_at: Some(expires_at),
                    },
                ),
            )
        };
        for (case, expires_at, accepted) in [
            ("after the entry", BLOCK_TIME_MS + 1, true),
            ("at the entry", BLOCK_TIME_MS, false),
            ("before the entry", BLOCK_TIME_MS - 1, false),
        ] {
            let reasons = reasons(read(
                &contract,
                vec![entry.clone()],
                &recipient_keys(),
                &[expiring(expires_at), recipient_identity()],
            ));
            if accepted {
                assert_eq!(reasons, vec![], "{case}");
            } else {
                assert!(
                    matches!(
                        reasons.as_slice(),
                        [PrivateContactRequestRejection::SenderKeyNotUsable(_)]
                    ),
                    "{case}: {reasons:?}"
                );
            }
        }
        assert!(
            private_contact_request_signing_keys(&contract, &expiring(1))
                .expect("DashPay v3")
                .is_empty(),
            "an expired key is not offered"
        );
    }

    /// The recipient key an entry names must be the one it is encrypted to, whether it is a
    /// key sealing could not encrypt to or an eligible one holding another point, and an
    /// identity must be given under its own id.
    #[test]
    fn should_refuse_a_recipient_key_or_identity_given_under_another_id() {
        let contract = dashpay();
        let sender = sender_identity(None);
        let entry = seal(&contract, &sender, &smallest_request(1), 1);
        // The recipient's decryption key is registered under its authentication key's id.
        let misregistered = PrivateDocumentScanKeys::new()
            .with_recipient(Identifier::from(RECIPIENT_ID), &[(0, secret_key(2))]);
        assert!(matches!(
            reasons(read(
                &contract,
                vec![entry.clone()],
                &misregistered,
                &[sender.clone(), recipient_identity()],
            ))
            .as_slice(),
            [PrivateContactRequestRejection::RecipientKeyNotUsable(_)]
        ));

        // Registered under an enabled ECDSA ENCRYPTION key, which sealing could encrypt to,
        // holding another point than the one it opened with: refused by the comparison.
        let recipient = with_key(
            recipient_identity(),
            key(
                4,
                Purpose::ENCRYPTION,
                KeyType::ECDSA_SECP256K1,
                &secret_key(6),
                None,
            ),
        );
        let misregistered = PrivateDocumentScanKeys::new()
            .with_recipient(Identifier::from(RECIPIENT_ID), &[(4, secret_key(2))]);
        let rejections = reasons(read(
            &contract,
            vec![entry.clone()],
            &misregistered,
            &[sender.clone(), recipient],
        ));
        assert!(
            matches!(
                rejections.as_slice(),
                [PrivateContactRequestRejection::RecipientKeyNotUsable(problem)]
                    if problem.contains("is not the key the entry is encrypted to")
            ),
            "{rejections:?}"
        );

        let target = PrivateDocumentTarget {
            contract_id: contract.id().to_buffer(),
            document_type_name: PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
        };
        let scan = recipient_keys().scan_page(&target, 0, &page(vec![entry]));
        let swapped = BTreeMap::from([
            (
                Identifier::from(SENDER_ID),
                ProvedIdentity {
                    identity: Some(recipient_identity()),
                    proved_at_height: PROVED_AT_HEIGHT,
                },
            ),
            (
                Identifier::from(RECIPIENT_ID),
                ProvedIdentity {
                    identity: Some(recipient_identity()),
                    proved_at_height: PROVED_AT_HEIGHT,
                },
            ),
        ]);
        assert!(private_contact_requests_from_scan(
            &contract,
            scan,
            &swapped,
            PlatformVersion::latest()
        )
        .is_err());
    }

    /// The fetch of a scan's identities refetches a stale proof once, keeps the second, fails
    /// on two stale proofs or a failed fetch, and keeps a proof that the identity does not
    /// exist.
    #[tokio::test]
    async fn should_fetch_identities_proved_after_their_entries() {
        let id = Identifier::from(SENDER_ID);
        let needed = BTreeMap::from([(id, 10)]);
        let scripted = |answers: Vec<Result<ProvedIdentity, Error>>| {
            let answers = std::sync::Mutex::new(answers.into_iter());
            move |_| {
                let answer = answers
                    .lock()
                    .expect("answers")
                    .next()
                    .expect("no more fetches than answers");
                async move { answer }
            }
        };
        let proved = |identity: Option<Identity>, height| {
            Ok(ProvedIdentity {
                identity,
                proved_at_height: height,
            })
        };

        let fetched = fetch_proved_identities(
            &needed,
            scripted(vec![
                proved(None, 9),
                proved(Some(sender_identity(None)), 10),
            ]),
        )
        .await
        .expect("the second proof is recent enough");
        assert_eq!(
            fetched.get(&id),
            Some(&ProvedIdentity {
                identity: Some(sender_identity(None)),
                proved_at_height: 10,
            })
        );
        let twice_stale = fetch_proved_identities(
            &needed,
            scripted(vec![
                proved(Some(sender_identity(None)), 9),
                proved(Some(sender_identity(None)), 9),
            ]),
        )
        .await
        .expect_err("two stale proofs");
        assert!(
            matches!(
                twice_stale,
                Error::StaleNode(StaleNodeError::Height {
                    expected_height: 10,
                    received_height: 9,
                    tolerance_blocks: 0,
                })
            ),
            "{twice_stale:?}"
        );
        assert!(
            twice_stale.can_retry(),
            "a later fetch may be recent enough"
        );
        assert!(fetch_proved_identities(
            &needed,
            scripted(vec![Err(Error::Generic("unreachable".to_string()))]),
        )
        .await
        .is_err());
        assert_eq!(
            fetch_proved_identities(&needed, scripted(vec![proved(None, 11)]))
                .await
                .expect("a recent proof of absence"),
            BTreeMap::from([(
                id,
                ProvedIdentity {
                    identity: None,
                    proved_at_height: 11,
                },
            )])
        );
    }

    /// The scan's futures are `Send`, so a host can spawn them.
    #[cfg(feature = "mocks")]
    #[test]
    fn should_give_futures_a_host_can_spawn() {
        fn spawnable<T: Send>(_: T) {}
        let sdk: Sdk = crate::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("a mock sdk");
        let keys = PrivateDocumentScanKeys::new();
        let needed = BTreeMap::new();
        spawnable(sdk.scan_private_contact_requests(0, &keys));
        spawnable(sdk.fetch_private_contact_request_identities(&needed));
    }

    /// DashPay's documents are signed at HIGH or CRITICAL.
    #[test]
    fn should_sign_at_the_levels_dashpay_documents_are_signed_at() {
        assert_eq!(
            private_contact_request_signing_security_levels(&dashpay()).expect("DashPay v3"),
            vec![SecurityLevel::CRITICAL, SecurityLevel::HIGH]
        );
    }

    /// The signer a wallet signs its transitions with, as `SimpleSigner` signs: the data's
    /// double SHA-256, in the compact form.
    #[derive(Debug)]
    struct TransitionSigner(BTreeMap<KeyID, SecretKey>);

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for TransitionSigner {
        async fn sign(
            &self,
            key: &IdentityPublicKey,
            data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            let secret_key = self
                .0
                .get(&key.id())
                .ok_or_else(|| ProtocolError::Generic(format!("no key {}", key.id())))?;
            let signature = dpp::dashcore::signer::sign(data, &secret_key.to_secret_bytes())
                .map_err(|error| ProtocolError::Generic(error.to_string()))?;
            Ok(BinaryData::new(signature.to_vec()))
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::Generic("no witnesses".to_string()))
        }

        fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
            self.0.contains_key(&key.id())
        }
    }

    /// The signer a wallet signs a public request's transition with signs a private one: its
    /// signature of the signing data is the one the entry carries.
    #[tokio::test]
    async fn should_seal_with_the_signature_of_a_transition_signer() {
        let contract = dashpay();
        let sender = sender_identity(None);
        let request = largest_request();
        let signing_key = sender.get_public_key_by_id(0).expect("key 0").clone();
        let signer = TransitionSigner(BTreeMap::from([(0, secret_key(9))]));
        let signing_data = private_contact_request_signing_data(
            &contract,
            &sender,
            1,
            &recipient_identity(),
            2,
            &request,
            PlatformVersion::latest(),
        )
        .expect("the keys check");
        let signature = private_contact_request_signature(&signer, &signing_key, &signing_data)
            .await
            .expect("the signer signs");
        assert_eq!(
            signature,
            sign_private_document_signing_data(&signing_data, &secret_key(9))
        );
        let entry = seal_private_contact_request(
            &contract,
            &PrivateContactRequestSender {
                identity: &sender,
                encryption_key_id: 1,
                encryption_secret_key: &secret_key(1),
                signing_key_id: 0,
                signature: &signature,
            },
            &recipient_identity(),
            2,
            &request,
            &[3; 32],
            PlatformVersion::latest(),
        )
        .expect("the request seals");
        let scan = read(
            &contract,
            vec![entry.to_vec()],
            &recipient_keys(),
            &[sender, recipient_identity()],
        );
        assert_eq!(scan.rejected, vec![]);
        assert_eq!(scan.received[0].request, request);
    }

    #[cfg(feature = "mocks")]
    use crate::platform::private_document_entries::held_contract_provider::HeldContractProvider;
    #[cfg(feature = "mocks")]
    use crate::platform::private_document_entries::PrivateDocumentEntriesQuery;
    #[cfg(feature = "mocks")]
    use dash_context_provider::ContextProvider;

    /// DashPay before its private type: as protocol version 14 has it.
    #[cfg(feature = "mocks")]
    fn dashpay_without_private_requests() -> DataContract {
        let contract = load_system_data_contract(
            SystemDataContract::Dashpay,
            PlatformVersion::get(14).expect("protocol version 14"),
        )
        .expect("DashPay loads at protocol version 14");
        assert!(private_contact_request_type(&contract).is_err());
        contract
    }

    /// A mock SDK pinned at the latest version, resolving contracts through `provider`.
    #[cfg(feature = "mocks")]
    fn sdk_with(provider: HeldContractProvider) -> Sdk {
        crate::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .with_context_provider(provider)
            .build()
            .expect("a mock sdk")
    }

    /// An SDK seeded below protocol version 15, whose context provider holds DashPay from
    /// before the private type, on a network running the latest version: it learns the
    /// network's version, fetches the current DashPay, and gives it to the provider, which the
    /// pages are then verified against.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_learn_the_version_and_refresh_a_dashpay_without_private_requests() {
        let provider = HeldContractProvider::new(Some(dashpay_without_private_requests()), true);
        let mut sdk: Sdk = crate::SdkBuilder::new_mock()
            .with_initial_version(PlatformVersion::get(14).expect("protocol version 14"))
            .with_context_provider(provider.clone())
            .build()
            .expect("a mock sdk");
        crate::sdk::test::expect_epoch_refresh(&mut sdk).await;
        let current = dashpay();
        sdk.mock()
            .expect_fetch(current.id(), Some(current.clone()))
            .await
            .expect("the DashPay fetch is expected");

        let contract = sdk
            .fetch_private_contact_request_contract()
            .await
            .expect("the current DashPay");

        assert_eq!(
            sdk.protocol_version_number(),
            PlatformVersion::latest().protocol_version
        );
        assert!(private_contact_request_type(&contract).is_ok());
        assert!(holds_private_contact_requests(provider.held().as_deref()));
    }

    /// A direct page read, as a generic reader makes one, by a client whose provider holds
    /// DashPay from before the private type: the current DashPay is fetched and given to the
    /// provider before the page, which is then verified against it.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_give_a_page_reader_s_provider_the_contract_with_the_private_type() {
        let provider = HeldContractProvider::new(Some(dashpay_without_private_requests()), true);
        let mut sdk = sdk_with(provider.clone());
        let current = dashpay();
        sdk.mock()
            .expect_fetch(current.id(), Some(current.clone()))
            .await
            .expect("the DashPay fetch is expected");
        sdk.mock()
            .expect_fetch(
                PrivateDocumentEntriesQuery::new(
                    current.id(),
                    PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                    0,
                ),
                Some(PrivateDocumentEntries {
                    entries: vec![],
                    total_count: 0,
                }),
            )
            .await
            .expect("the page fetch is expected");

        let scan = sdk
            .scan_private_document_page(
                current.id(),
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                0,
                &PrivateDocumentScanKeys::new(),
            )
            .await
            .expect("the page is read");
        assert!(scan.is_complete());
        assert!(holds_private_contact_requests(provider.held().as_deref()));

        // A provider holding a contract with the type keeps it: the very contract it holds
        // comes back, where a refetch would have given it another.
        let held = provider.held().expect("the provider holds DashPay");
        let resolved = sdk
            .fetch_private_document_contract(
                current.id(),
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
            )
            .await
            .expect("the held contract");
        assert!(Arc::ptr_eq(&resolved, &held));
    }

    /// A newer contract another operation gave the provider while the reader fetched one,
    /// here promoted right after the reader's first read, stays: the older fetched contract
    /// does not replace it, even through a provider whose registration replaces whatever it
    /// holds, and the page is read against the newer one.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_keep_a_newer_contract_promoted_while_the_reader_fetched() {
        let mut stale = dashpay_without_private_requests();
        stale.set_version(1);
        let mut fetched = dashpay();
        fetched.set_version(2);
        let mut promoted = dashpay();
        promoted.set_version(3);
        let provider = HeldContractProvider::promoting(stale, promoted);
        let mut sdk = sdk_with(provider.clone());
        sdk.mock()
            .expect_fetch(fetched.id(), Some(fetched.clone()))
            .await
            .expect("the DashPay fetch is expected");
        sdk.mock()
            .expect_fetch(
                PrivateDocumentEntriesQuery::new(
                    fetched.id(),
                    PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                    0,
                ),
                Some(PrivateDocumentEntries {
                    entries: vec![],
                    total_count: 0,
                }),
            )
            .await
            .expect("the page fetch is expected");

        let resolved = sdk
            .fetch_private_document_contract(
                fetched.id(),
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
            )
            .await
            .expect("the newer contract");
        assert_eq!(resolved.version(), 3);
        assert_eq!(provider.held().expect("held").version(), 3);
        let scan = sdk
            .scan_private_document_page(
                fetched.id(),
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                0,
                &PrivateDocumentScanKeys::new(),
            )
            .await
            .expect("the page is read");
        assert!(scan.is_complete());
        assert_eq!(provider.held().expect("held").version(), 3);
    }

    /// Below protocol version 15 there is no private store: a contract that declares no
    /// private type, on an SDK running 14, is refused as not active, and the page stream ends
    /// with that refusal before any page is fetched, which the mock, answering none, would
    /// fail otherwise.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_refuse_a_private_page_read_below_protocol_version_15() {
        use futures::StreamExt;

        let before = dashpay_without_private_requests();
        let mut sdk: Sdk = crate::SdkBuilder::new_mock()
            .with_version(PlatformVersion::get(14).expect("protocol version 14"))
            .with_context_provider(HeldContractProvider::new(Some(before.clone()), true))
            .build()
            .expect("a mock sdk");
        sdk.mock()
            .expect_fetch(before.id(), Some(before.clone()))
            .await
            .expect("the DashPay fetch is expected");

        let refused = sdk
            .fetch_private_document_contract(
                before.id(),
                PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
            )
            .await;
        assert!(
            matches!(
                &refused,
                Err(Error::ProtocolVersionNotActive { required: 15, .. })
            ),
            "{refused:?}"
        );
        let mut pages = std::pin::pin!(sdk.private_document_pages(
            before.id(),
            PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
            0,
            4,
        ));
        assert!(matches!(
            pages.next().await,
            Some(Err(Error::ProtocolVersionNotActive { .. }))
        ));
        assert!(pages.next().await.is_none());
    }

    /// A DashPay the provider does not hold yet is fetched from the network and given to it,
    /// which the pages are then verified against.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_give_an_empty_provider_the_dashpay_it_fetches() {
        let provider = HeldContractProvider::new(None, true);
        let mut sdk = sdk_with(provider.clone());
        let current = dashpay();
        sdk.mock()
            .expect_fetch(current.id(), Some(current.clone()))
            .await
            .expect("the DashPay fetch is expected");

        let contract = sdk
            .fetch_private_contact_request_contract()
            .await
            .expect("the current DashPay");

        assert!(private_contact_request_type(&contract).is_ok());
        let verified_against = provider
            .get_data_contract(&current.id(), PlatformVersion::latest())
            .expect("the provider answers");
        assert!(holds_private_contact_requests(verified_against.as_deref()));
    }

    /// A DashPay the provider holds with the private type is used as it is: nothing is fetched,
    /// which the mock, expecting no fetch, would refuse.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_use_a_dashpay_the_provider_holds_with_private_requests() {
        let provider = HeldContractProvider::new(Some(dashpay()), false);
        let sdk = sdk_with(provider.clone());

        let contract = sdk
            .fetch_private_contact_request_contract()
            .await
            .expect("the held DashPay");

        assert!(Arc::ptr_eq(
            &contract,
            &provider.held().expect("the provider holds DashPay")
        ));
    }

    /// A refresh that leaves the provider without the private type fails, rather than every
    /// page failing its proof against the old contract; so does a network without DashPay.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_fail_a_refresh_the_provider_does_not_take() {
        let current = dashpay();

        let mut sdk = sdk_with(HeldContractProvider::new(
            Some(dashpay_without_private_requests()),
            false,
        ));
        sdk.mock()
            .expect_fetch(current.id(), Some(current.clone()))
            .await
            .expect("the DashPay fetch is expected");
        let error = sdk
            .fetch_private_contact_request_contract()
            .await
            .expect_err("the provider keeps the old DashPay")
            .to_string();
        assert!(error.contains("does not hold"), "{error}");

        let mut sdk = sdk_with(HeldContractProvider::new(None, false));
        sdk.mock()
            .expect_fetch(current.id(), Some(current.clone()))
            .await
            .expect("the DashPay fetch is expected");
        let error = sdk
            .fetch_private_contact_request_contract()
            .await
            .expect_err("the provider takes no contract")
            .to_string();
        assert!(error.contains("does not hold"), "{error}");

        let mut sdk = sdk_with(HeldContractProvider::new(
            Some(dashpay_without_private_requests()),
            true,
        ));
        sdk.mock()
            .expect_fetch(current.id(), None::<DataContract>)
            .await
            .expect("the DashPay fetch is expected");
        let error = sdk
            .fetch_private_contact_request_contract()
            .await
            .expect_err("the network has no DashPay")
            .to_string();
        assert!(error.contains("is gone"), "{error}");
    }
}
