//! Contact request creation and state transition helpers
//!
//! Implements DIP-15 DashPay contact request functionality

use crate::platform::transition::broadcast::{wait_after_broadcast, BroadcastStateTransition};
use crate::platform::transition::put_document::prepare_document_state_transition;
use crate::platform::transition::waitable::Waitable;
use crate::platform::Document;
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{DocumentType, DocumentTypeRef};
use dpp::data_contract::DataContract;
use dpp::document::DocumentV0;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::Purpose;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, SecurityLevel};
use dpp::platform_value::{Bytes32, Value};
use dpp::prelude::Identifier;
use dpp::secp256k1::rand::rngs::StdRng;
use dpp::secp256k1::rand::{Rng, RngCore, SeedableRng};
use dpp::secp256k1::{PublicKey, SecretKey};
use dpp::state_transition::StateTransition;
use dpp::version::feature_initial_protocol_versions::DASHPAY_CONTACT_REQUEST_SHIELDED_ADDRESS_INITIAL_PROTOCOL_VERSION;
use platform_encryption::{
    derive_shared_key_ecdh, encrypt_account_label, encrypt_extended_public_key,
    encrypt_shielded_address, COMPACT_XPUB_LEN, SHIELDED_ADDRESS_LEN,
};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The DashPay document type of a contact request.
const CONTACT_REQUEST: &str = "contactRequest";

/// The `contactRequest` property that carries the sender's shielded payment address.
const ENCRYPTED_SHIELDED_ADDRESS: &str = "encryptedShieldedAddress";

/// A contact request prepared by [`Sdk::prepare_contact_request`]: the `contactRequest`
/// document type of the DashPay contract the SDK resolves for its protocol version, which the
/// request document is built as and whose rules the key that signs it must meet. A caller
/// choosing the key reads them here, then sends through [`Sdk::submit_contact_request`], so the
/// key is judged by the contract the document is built against and never by another snapshot
/// of it.
#[derive(Debug, Clone)]
pub struct PreparedContactRequest {
    contact_request_document_type: DocumentType,
    with_shielded_address: bool,
}

impl PreparedContactRequest {
    /// The security level the key signing the `contactRequest` document must meet.
    pub fn signing_security_level_requirement(&self) -> SecurityLevel {
        self.contact_request_document_type
            .as_ref()
            .security_level_requirement()
    }

    /// The `contactRequest` document type the request is built as, which the signing key's
    /// contract bounds must cover.
    pub fn contact_request_document_type(&self) -> DocumentTypeRef<'_> {
        self.contact_request_document_type.as_ref()
    }

    /// The id of the DashPay contract the request writes to.
    pub fn dashpay_contract_id(&self) -> Identifier {
        self.contact_request_document_type
            .as_ref()
            .data_contract_id()
    }

    /// Whether the request was prepared to carry a shielded address: the request sent with it
    /// must carry one exactly when this is `true`.
    pub fn with_shielded_address(&self) -> bool {
        self.with_shielded_address
    }
}

/// ECDH provider for contact request encryption
///
/// Supports two modes:
/// 1. Client-side ECDH (preferred for hardware wallets)
/// 2. SDK-side ECDH (for software wallets providing private keys)
pub enum EcdhProvider<F, Fut, G, Gut>
where
    F: FnOnce(&IdentityPublicKey, u32) -> Fut,
    Fut: std::future::Future<Output = Result<SecretKey, Error>>,
    G: FnOnce(&PublicKey) -> Gut,
    Gut: std::future::Future<Output = Result<[u8; 32], Error>>,
{
    /// Client performs ECDH and provides the shared secret directly
    /// This is preferred for hardware wallets that can do ECDH internally
    ClientSide {
        /// Callback to get the shared secret after client performs ECDH
        /// Parameters: recipient's public key
        /// Returns: 32-byte shared secret
        get_shared_secret: G,
    },
    /// SDK performs ECDH using provided private key
    /// This is for software wallets that can provide the private key
    SdkSide {
        /// Callback to get the sender's private encryption key
        /// Parameters: (IdentityPublicKey, key_index)
        /// Returns: Private key for ECDH
        get_private_key: F,
    },
}

/// Recipient identity specification for contact requests
#[derive(Debug, Clone)]
pub enum RecipientIdentity {
    /// Recipient identity ID - the full identity will be fetched from the platform
    Identifier(Identifier),
    /// Complete recipient identity - no fetch required
    Identity(Identity),
}

impl RecipientIdentity {
    /// Get the identifier from the recipient
    pub fn id(&self) -> Identifier {
        match self {
            RecipientIdentity::Identifier(id) => *id,
            RecipientIdentity::Identity(identity) => identity.id(),
        }
    }
}

impl From<Identifier> for RecipientIdentity {
    fn from(id: Identifier) -> Self {
        RecipientIdentity::Identifier(id)
    }
}

impl From<Identity> for RecipientIdentity {
    fn from(identity: Identity) -> Self {
        RecipientIdentity::Identity(identity)
    }
}

/// Input for creating a contact request document
pub struct ContactRequestInput {
    /// The identity sending the contact request (owner)
    pub sender_identity: Identity,
    /// The recipient - can be either an Identifier (will be fetched) or a complete Identity
    pub recipient: RecipientIdentity,
    /// The sender's encryption key index for ECDH
    pub sender_key_index: u32,
    /// The recipient's encryption key index for ECDH
    pub recipient_key_index: u32,
    /// Reference to the DashPay receiving account
    pub account_reference: u32,
    /// Optional account label (UNENCRYPTED string - SDK will encrypt to 48-80 bytes automatically)
    pub account_label: Option<String>,
    /// Optional auto-accept proof (38-102 bytes) - not encrypted
    pub auto_accept_proof: Option<Vec<u8>>,
    /// The sender's shielded payment address for this contact: 43 raw bytes, the address
    /// the sender's wallet derives for this request alone. The SDK encrypts it as
    /// `encryptedShieldedAddress`. `None` sends the request without one.
    pub shielded_address: Option<[u8; SHIELDED_ADDRESS_LEN]>,
}

/// Whether the contact requests of `dashpay`, a DashPay contract, have the property that
/// carries a shielded address. They gain it with DashPay v3 at protocol version 15.
///
/// The schema is asked here alone. The SDK refuses an address against a contract without
/// the property, and a wallet that reads the network's contract asks here whether to send
/// one, so the two cannot come to differ on what the property is.
pub fn dashpay_contact_requests_carry_shielded_address(dashpay: &DataContract) -> bool {
    dashpay
        .document_type_for_name(CONTACT_REQUEST)
        .is_ok_and(carries_shielded_address)
}

/// Whether `contact_request_document_type` has a property for a shielded address.
fn carries_shielded_address(contact_request_document_type: DocumentTypeRef) -> bool {
    contact_request_document_type
        .properties()
        .contains_key(ENCRYPTED_SHIELDED_ADDRESS)
}

/// Fails unless the network's `contactRequest` type has a property for a shielded address.
///
/// It gains it with DashPay v3 at protocol version 15, and consensus refuses a request that
/// carries it before then, so there the caller sends the request without an address.
fn ensure_carries_shielded_address(
    contact_request_document_type: DocumentTypeRef,
) -> Result<(), Error> {
    if carries_shielded_address(contact_request_document_type) {
        return Ok(());
    }
    Err(Error::Generic(format!(
        "the DashPay contract of this network has no {ENCRYPTED_SHIELDED_ADDRESS} on contact \
         requests; it is added at protocol version 15"
    )))
}

/// A contact request's `encryptedShieldedAddress`: `address` under the request's shared key
/// and an IV drawn for this field alone.
///
/// Refused when the network's `contactRequest` type has no such property
/// ([`ensure_carries_shielded_address`]).
fn encrypted_shielded_address(
    contact_request_document_type: DocumentTypeRef,
    shared_key: &[u8; 32],
    address: &[u8; SHIELDED_ADDRESS_LEN],
    rng: &mut impl RngCore,
) -> Result<Vec<u8>, Error> {
    ensure_carries_shielded_address(contact_request_document_type)?;
    let mut iv = [0u8; 16];
    rng.fill_bytes(&mut iv);
    Ok(encrypt_shielded_address(shared_key, &iv, address).to_vec())
}

/// Result of creating a contact request document
#[derive(Debug)]
pub struct ContactRequestResult {
    /// The document id derived from `entropy` alone.
    ///
    /// From protocol version 14 the id of a new document also commits to the
    /// identity contract nonce of its create transition, which is only
    /// assigned when the document is sent, so this is a placeholder there:
    /// the id of the contact request is the one on the document returned by
    /// `send_contact_request`.
    pub id: Identifier,
    /// The owner ID (sender identity ID)
    pub owner_id: Identifier,
    /// The document properties
    pub properties: BTreeMap<String, Value>,
    /// The entropy used to derive `id`.
    ///
    /// This must be reused when broadcasting the document: up to protocol
    /// version 13 the id platform consensus recomputes from the entropy must
    /// match `id` (otherwise the create transition is rejected with
    /// `InvalidDocumentTransitionIdError`), and from protocol version 14 it is
    /// one of the inputs of the final id.
    pub entropy: Bytes32,
}

/// Input for sending a contact request to the platform
pub struct SendContactRequestInput<S: Signer<IdentityPublicKey>> {
    /// The contact request input data
    pub contact_request: ContactRequestInput,
    /// The identity public key to use for signing
    pub identity_public_key: IdentityPublicKey,
    /// The signer for the identity
    pub signer: S,
}

/// Result of sending a contact request
#[derive(Debug)]
pub struct SendContactRequestResult {
    /// The contact request document that was submitted to the platform
    pub document: Document,
    /// The recipient's identity ID
    pub recipient_id: Identifier,
    /// The account reference
    pub account_reference: u32,
}

/// A contact request signed and not yet broadcast
/// ([`Sdk::sign_contact_request`]).
#[derive(Debug, Clone)]
pub struct SignedContactRequest {
    /// The signed `contactRequest` create transition. Broadcasting these same
    /// bytes again is a retry of this request, never a second one: the
    /// transition carries its identity contract nonce.
    pub state_transition: StateTransition,
    /// The recipient's identity ID
    pub recipient_id: Identifier,
    /// The account reference
    pub account_reference: u32,
}

/// Whether `purpose` is acceptable for the `senderKeyIndex` key of a contact
/// request **we are about to mint**. The sender always references its own
/// ENCRYPTION key.
///
/// Mint-side only — see [`sender_key_purpose_is_acceptable_on_receive`] for
/// what we accept from documents already on chain.
pub(crate) fn sender_key_purpose_is_valid(purpose: Purpose) -> bool {
    purpose == Purpose::ENCRYPTION
}

/// Whether `purpose` is acceptable for the `recipientKeyIndex` key of a
/// contact request **we are about to mint**. The newest cohort references the
/// recipient's DECRYPTION key (our original convention); the dominant mobile
/// cohort has no DECRYPTION key and references its ENCRYPTION key. Accept
/// either; reject every other purpose.
///
/// This is the single source of truth for what we are willing to *create*.
/// The recipient-key selector (`select_recipient_key_index`) defers to it so
/// the minted cohort cannot drift between the SDK and wallet layers. It
/// deliberately stays strict: reusing a signing or fund-authorizing key for
/// ECDH is poor key separation, and no new document needs to.
///
/// It is NOT the acceptance policy for inbound documents — a `contactRequest`
/// is immutable, so history cannot be re-minted to fit this rule. See
/// [`recipient_key_purpose_is_acceptable_on_receive`].
pub fn recipient_key_purpose_is_valid(purpose: Purpose) -> bool {
    matches!(purpose, Purpose::DECRYPTION | Purpose::ENCRYPTION)
}

/// Whether `purpose` on the `recipientKeyIndex` key of an **inbound, already
/// on-chain** contact request is acceptable for the ECDH that unwraps the
/// sender's `encryptedPublicKey`.
///
/// Strictly wider than [`recipient_key_purpose_is_valid`], and deliberately
/// so. `contactRequest` documents are immutable and consensus enforces no
/// purpose constraint on these integer fields, so the acceptance policy is the
/// *only* thing standing between a user and their own payment history.
/// Mainnet device logs (2026-08, a 29-contact wallet whose contacts were
/// established through the legacy Android/dashj client) show 27 of 29 inbound
/// requests referencing the recipient's AUTHENTICATION (key ids 0-2) or
/// TRANSFER (key id 3) key — under the mint-side rule every one of those
/// contacts is unpayable forever, with no action the user can take.
///
/// Purpose carries no cryptographic weight here: ECDH is defined over the
/// secp256k1 keypair, and DIP-9's identity-key tree is indexed by key *type*
/// and id, never by purpose, so the same derivation reaches all of them. The
/// gates that do carry weight — `ECDSA_SECP256K1` key type and the
/// disabled-key check — are enforced separately by the caller and are
/// unaffected by this predicate.
///
/// The node-operational purposes (SYSTEM, VOTING, OWNER) stay rejected:
/// nothing on chain references them for DashPay, and they have no business in
/// a payment-channel handshake.
pub fn recipient_key_purpose_is_acceptable_on_receive(purpose: Purpose) -> bool {
    matches!(
        purpose,
        Purpose::DECRYPTION | Purpose::ENCRYPTION | Purpose::AUTHENTICATION | Purpose::TRANSFER
    )
}

/// Receive-side counterpart of [`sender_key_purpose_is_valid`]: whether
/// `purpose` on the `senderKeyIndex` key of an **inbound, already on-chain**
/// contact request is acceptable for ECDH.
///
/// Same reasoning as [`recipient_key_purpose_is_acceptable_on_receive`]. The
/// legacy cohort is narrower on this side — the observed mainnet documents
/// pair an AUTHENTICATION sender key with an AUTHENTICATION recipient key — so
/// only AUTHENTICATION is added. A sender referencing any other purpose has
/// not been seen and stays a purpose mismatch (skip-and-retry, never a
/// permanently broken channel), leaving room to widen again on evidence.
pub fn sender_key_purpose_is_acceptable_on_receive(purpose: Purpose) -> bool {
    matches!(purpose, Purpose::ENCRYPTION | Purpose::AUTHENTICATION)
}

impl Sdk {
    /// Create a contact request document
    ///
    /// This creates a local contact request document according to DIP-15 specification.
    /// The document is not yet submitted to the platform. This method automatically
    /// handles ECDH key derivation and encryption of the extended public key and account label.
    ///
    /// # Arguments
    ///
    /// * `input` - The contact request input containing sender/recipient information and unencrypted data
    /// * `ecdh_provider` - Provider for ECDH key exchange (client-side or SDK-side)
    /// * `get_extended_public_key` - Async function to retrieve the extended public key to share with recipient
    ///   - Parameters: `(account_reference: u32)`
    ///   - Returns: The unencrypted extended public key bytes — the **69-byte
    ///     DIP-15 compact form** (`parentFingerprint(4) ‖ chainCode(32) ‖
    ///     pubKey(33)`), NOT a 78/107-byte BIP32/DIP-14 serialization. A
    ///     non-69-byte return is rejected before encryption.
    ///
    /// # Returns
    ///
    /// Returns a `ContactRequestResult` containing the created document
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The DashPay contract cannot be fetched
    /// - The contactRequest document type is not found
    /// - The sender or recipient doesn't have the required encryption keys
    /// - ECDH encryption fails
    /// - The shared secret, private key, or extended public key cannot be retrieved
    pub async fn create_contact_request<F, Fut, G, Gut, H, Hut>(
        &self,
        input: ContactRequestInput,
        ecdh_provider: EcdhProvider<F, Fut, G, Gut>,
        get_extended_public_key: H,
    ) -> Result<ContactRequestResult, Error>
    where
        F: FnOnce(&IdentityPublicKey, u32) -> Fut,
        Fut: std::future::Future<Output = Result<SecretKey, Error>>,
        G: FnOnce(&PublicKey) -> Gut,
        Gut: std::future::Future<Output = Result<[u8; 32], Error>>,
        H: FnOnce(u32) -> Hut,
        Hut: std::future::Future<Output = Result<Vec<u8>, Error>>,
    {
        let prepared = self
            .prepare_contact_request(input.shielded_address.is_some())
            .await?;
        self.build_contact_request(&prepared, input, ecdh_provider, get_extended_public_key)
            .await
    }

    /// Prepare a contact request: resolve the DashPay contract for the SDK's protocol version
    /// (the context provider's, fetched and proved when it holds none) and its
    /// `contactRequest` document type. Nothing is built, signed or submitted. The returned
    /// [`PreparedContactRequest`] tells what that contract requires of the signing key; send
    /// it with [`Sdk::submit_contact_request`].
    ///
    /// `with_shielded_address` says whether the request will carry a shielded address; the
    /// contract is then resolved at a protocol version that has the property for it. A
    /// request whose address does not match this is refused when sent.
    /// Preparation fails if the resolved document type cannot carry the requested address.
    pub async fn prepare_contact_request(
        &self,
        with_shielded_address: bool,
    ) -> Result<PreparedContactRequest, Error> {
        let dashpay_contract = self
            .dashpay_contract_for_contact_request(with_shielded_address)
            .await?;
        let contact_request_document_type = dashpay_contract
            .document_type_cloned_for_name(CONTACT_REQUEST)
            .map_err(|_| {
                Error::Generic("DashPay contactRequest document type not found".to_string())
            })?;
        if with_shielded_address {
            ensure_carries_shielded_address(contact_request_document_type.as_ref())?;
        }
        Ok(PreparedContactRequest {
            contact_request_document_type,
            with_shielded_address,
        })
    }

    /// The contact request document [`Sdk::create_contact_request`] describes, built as
    /// `prepared`'s `contactRequest` document type.
    async fn build_contact_request<F, Fut, G, Gut, H, Hut>(
        &self,
        prepared: &PreparedContactRequest,
        input: ContactRequestInput,
        ecdh_provider: EcdhProvider<F, Fut, G, Gut>,
        get_extended_public_key: H,
    ) -> Result<ContactRequestResult, Error>
    where
        F: FnOnce(&IdentityPublicKey, u32) -> Fut,
        Fut: std::future::Future<Output = Result<SecretKey, Error>>,
        G: FnOnce(&PublicKey) -> Gut,
        Gut: std::future::Future<Output = Result<[u8; 32], Error>>,
        H: FnOnce(u32) -> Hut,
        Hut: std::future::Future<Output = Result<Vec<u8>, Error>>,
    {
        // The contract was resolved for a request with or without an address; one sent the
        // other way could be judged against a contract lacking the property.
        if input.shielded_address.is_some() != prepared.with_shielded_address {
            let (sent, prepared_for) = if prepared.with_shielded_address {
                ("without", "with")
            } else {
                ("with", "without")
            };
            return Err(Error::InvalidRequest(format!(
                "a contact request {sent} a shielded address was prepared for one {prepared_for} \
                 an address; prepare it again with the request's address"
            )));
        }

        // Validate auto accept proof size if provided
        if let Some(ref proof) = input.auto_accept_proof {
            if proof.len() < 38 || proof.len() > 102 {
                return Err(Error::Generic(format!(
                    "autoAcceptProof must be 38-102 bytes, got {}",
                    proof.len()
                )));
            }
        }

        // Fetch recipient identity if only ID was provided
        let recipient_identity = match input.recipient {
            RecipientIdentity::Identity(identity) => identity,
            RecipientIdentity::Identifier(id) => {
                use crate::platform::Fetch;
                Identity::fetch(self, id)
                    .await?
                    .ok_or_else(|| Error::Generic(format!("Recipient identity {} not found", id)))?
            }
        };

        // Verify sender has the encryption key at the specified index
        let sender_key = input
            .sender_identity
            .public_keys()
            .get(&input.sender_key_index)
            .ok_or_else(|| {
                Error::Generic(format!(
                    "Sender identity does not have encryption key at index {}",
                    input.sender_key_index
                ))
            })?;

        // Sender always references its own ENCRYPTION key (the live
        // convention of both on-chain cohorts).
        if !sender_key_purpose_is_valid(sender_key.purpose()) {
            return Err(Error::Generic(format!(
                "Sender key at index {} is not an encryption key",
                input.sender_key_index
            )));
        }

        // Verify recipient has the referenced key at the specified index.
        let recipient_key = recipient_identity
            .public_keys()
            .get(&input.recipient_key_index)
            .ok_or_else(|| {
                Error::Generic(format!(
                    "Recipient identity does not have a key at index {}",
                    input.recipient_key_index
                ))
            })?;

        // Accept either a DECRYPTION key (newest cohort / our original
        // convention) OR an ENCRYPTION key (the dominant mobile cohort, whose
        // identities carry no DECRYPTION key and reference their ENCRYPTION
        // key for recipientKeyIndex).
        if !recipient_key_purpose_is_valid(recipient_key.purpose()) {
            return Err(Error::Generic(format!(
                "Recipient key at index {} is not a decryption or encryption key",
                input.recipient_key_index
            )));
        }

        // Get the recipient's public key data for ECDH
        let recipient_public_key_data = recipient_key.data();
        let recipient_public_key = PublicKey::from_slice(recipient_public_key_data.as_slice())
            .map_err(|e| Error::Generic(format!("Invalid recipient public key: {}", e)))?;

        // The contract is resolved, and a shielded address refused where contact requests
        // carry none, before the host is asked for a secret or a key: a hardware or keychain
        // prompt is not spent on a request the network refuses, and a callback that fails
        // does not hide that refusal.
        let contact_request_document_type = prepared.contact_request_document_type();
        if input.shielded_address.is_some() {
            ensure_carries_shielded_address(contact_request_document_type)?;
        }

        // Derive shared secret using ECDH (either client-side or SDK-side)
        let shared_key = match ecdh_provider {
            EcdhProvider::ClientSide { get_shared_secret } => {
                // Client performs ECDH and provides the shared secret
                get_shared_secret(&recipient_public_key).await?
            }
            EcdhProvider::SdkSide { get_private_key } => {
                // SDK performs ECDH using the provided private key
                let sender_private_key =
                    get_private_key(sender_key, input.sender_key_index).await?;
                derive_shared_key_ecdh(&sender_private_key, &recipient_public_key)
            }
        };

        // Get the extended public key to encrypt. Per DIP-15 the callback must
        // return the 69-byte COMPACT form (parentFingerprint ‖ chainCode ‖
        // pubKey) — NOT a 78/107-byte BIP32/DIP-14 serialization. Validate the
        // length up front so a malformed producer fails with a precise error
        // instead of the downstream "96-byte" assertion (which a 78-byte input
        // would silently pass while remaining undecryptable by mobile clients).
        let extended_public_key = get_extended_public_key(input.account_reference).await?;
        if extended_public_key.len() != COMPACT_XPUB_LEN {
            return Err(Error::Generic(format!(
                "Extended public key must be the {COMPACT_XPUB_LEN}-byte DIP-15 compact form \
                 (parentFingerprint ‖ chainCode ‖ pubKey), got {} bytes",
                extended_public_key.len()
            )));
        }

        // Generate random IVs for encryption
        let mut rng = StdRng::from_os_rng();
        let mut xpub_iv = [0u8; 16];
        rng.fill_bytes(&mut xpub_iv);

        // Encrypt the extended public key (includes IV prepended)
        let encrypted_public_key =
            encrypt_extended_public_key(&shared_key, &xpub_iv, &extended_public_key);

        // Validate encrypted public key size (must be exactly 96 bytes: 16-byte IV + 80-byte encrypted data)
        if encrypted_public_key.len() != 96 {
            return Err(Error::Generic(format!(
                "Encrypted public key size mismatch: expected 96 bytes, got {}",
                encrypted_public_key.len()
            )));
        }

        // Encrypt the account label if provided (includes IV prepended)
        let encrypted_account_label = if let Some(ref label) = input.account_label {
            let mut label_iv = [0u8; 16];
            rng.fill_bytes(&mut label_iv);
            let encrypted = encrypt_account_label(&shared_key, &label_iv, label);

            // Validate encrypted label size (48-80 bytes: 16-byte IV + 32-64 byte encrypted data)
            if encrypted.len() < 48 || encrypted.len() > 80 {
                return Err(Error::Generic(format!(
                    "Encrypted account label size out of range: expected 48-80 bytes, got {}",
                    encrypted.len()
                )));
            }
            Some(encrypted)
        } else {
            None
        };

        // Generate entropy for document ID
        let mut rng = StdRng::from_os_rng();
        let entropy = Bytes32::new(rng.random());

        // Encrypt the shielded address if provided, under an IV of its own
        let encrypted_shielded_address = input
            .shielded_address
            .map(|address| {
                encrypted_shielded_address(
                    contact_request_document_type,
                    &shared_key,
                    &address,
                    &mut rng,
                )
            })
            .transpose()?;

        // Generate document ID
        let sender_id = input.sender_identity.id().to_owned();
        let document_id = Document::generate_document_id_v0(
            &prepared.dashpay_contract_id(),
            &sender_id,
            contact_request_document_type.name(),
            entropy.as_slice(),
        );

        // Build document properties
        let mut properties = BTreeMap::new();
        let recipient_id = recipient_identity.id().to_owned();
        properties.insert(
            "toUserId".to_string(),
            Value::Identifier(recipient_id.to_buffer()),
        );
        properties.insert(
            "encryptedPublicKey".to_string(),
            Value::Bytes(encrypted_public_key),
        );
        properties.insert(
            "senderKeyIndex".to_string(),
            Value::U32(input.sender_key_index),
        );
        properties.insert(
            "recipientKeyIndex".to_string(),
            Value::U32(input.recipient_key_index),
        );
        properties.insert(
            "accountReference".to_string(),
            Value::U32(input.account_reference),
        );

        // Add optional fields
        if let Some(label) = encrypted_account_label {
            properties.insert("encryptedAccountLabel".to_string(), Value::Bytes(label));
        }
        if let Some(proof) = input.auto_accept_proof {
            properties.insert("autoAcceptProof".to_string(), Value::Bytes(proof));
        }
        if let Some(address) = encrypted_shielded_address {
            properties.insert(
                ENCRYPTED_SHIELDED_ADDRESS.to_string(),
                Value::Bytes(address),
            );
        }

        // Return the essential fields for the contact request, including the
        // entropy that derived `document_id` so the broadcast path can reuse it.
        Ok(ContactRequestResult {
            id: document_id,
            owner_id: sender_id,
            properties,
            entropy,
        })
    }

    /// The DashPay contract a contact request is built against.
    ///
    /// For a request that carries a shielded address the SDK first learns the network's
    /// protocol version. An unpinned SDK starts below 15 and takes the context provider's
    /// DashPay at its own version, which has no property for the address, so the request
    /// would be refused on a network that accepts it. A request without an address resolves
    /// the contract as before, with no extra read.
    async fn dashpay_contract_for_contact_request(
        &self,
        with_shielded_address: bool,
    ) -> Result<Arc<DataContract>, Error> {
        if with_shielded_address {
            self.require_protocol_version(
                DASHPAY_CONTACT_REQUEST_SHIELDED_ADDRESS_INITIAL_PROTOCOL_VERSION,
                "a contact request's shielded address",
            )
            .await?;
        }
        self.fetch_dashpay_contract().await
    }

    /// Send a contact request to the platform
    ///
    /// This creates a contact request document with automatic ECDH encryption and submits it
    /// to the platform as a state transition.
    ///
    /// # Arguments
    ///
    /// * `input` - The send contact request input containing document data, key, and signer
    /// * `ecdh_provider` - Provider for ECDH key exchange (client-side or SDK-side)
    /// * `get_extended_public_key` - Async function to retrieve the extended public key to share with recipient
    ///   - Parameters: `(account_reference: u32)`
    ///   - Returns: The unencrypted extended public key bytes — the **69-byte
    ///     DIP-15 compact form** (`parentFingerprint(4) ‖ chainCode(32) ‖
    ///     pubKey(33)`), NOT a 78/107-byte BIP32/DIP-14 serialization.
    ///
    /// # Returns
    ///
    /// Returns a `SendContactRequestResult` containing the submitted document
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Document creation fails (including ECDH encryption)
    /// - State transition submission fails
    pub async fn send_contact_request<S: Signer<IdentityPublicKey>, F, Fut, G, Gut, H, Hut>(
        &self,
        input: SendContactRequestInput<S>,
        ecdh_provider: EcdhProvider<F, Fut, G, Gut>,
        get_extended_public_key: H,
    ) -> Result<SendContactRequestResult, Error>
    where
        F: FnOnce(&IdentityPublicKey, u32) -> Fut,
        Fut: std::future::Future<Output = Result<SecretKey, Error>>,
        G: FnOnce(&PublicKey) -> Gut,
        Gut: std::future::Future<Output = Result<[u8; 32], Error>>,
        H: FnOnce(u32) -> Hut,
        Hut: std::future::Future<Output = Result<Vec<u8>, Error>>,
    {
        let prepared = self
            .prepare_contact_request(input.contact_request.shielded_address.is_some())
            .await?;
        self.submit_contact_request(prepared, input, ecdh_provider, get_extended_public_key)
            .await
    }

    /// Send a contact request like [`Sdk::send_contact_request`], building and submitting
    /// the document as the `contactRequest` type of `prepared`, the snapshot the caller
    /// chose `input`'s signing key by.
    pub async fn submit_contact_request<S: Signer<IdentityPublicKey>, F, Fut, G, Gut, H, Hut>(
        &self,
        prepared: PreparedContactRequest,
        input: SendContactRequestInput<S>,
        ecdh_provider: EcdhProvider<F, Fut, G, Gut>,
        get_extended_public_key: H,
    ) -> Result<SendContactRequestResult, Error>
    where
        F: FnOnce(&IdentityPublicKey, u32) -> Fut,
        Fut: std::future::Future<Output = Result<SecretKey, Error>>,
        G: FnOnce(&PublicKey) -> Gut,
        Gut: std::future::Future<Output = Result<[u8; 32], Error>>,
        H: FnOnce(u32) -> Hut,
        Hut: std::future::Future<Output = Result<Vec<u8>, Error>>,
    {
        let SignedContactRequest {
            state_transition,
            recipient_id,
            account_reference,
        } = self
            .sign_prepared_contact_request(prepared, input, ecdh_provider, get_extended_public_key)
            .await?;

        // Submit the document to the platform, and wait for the proof of the
        // same signed transition.
        let broadcast = state_transition.broadcast(self, None).await;
        let platform_document = wait_after_broadcast(
            broadcast,
            <Document as Waitable>::wait_for_response(self, state_transition, None),
        )
        .await?;

        // Return the result with recipient ID and account reference we saved earlier
        Ok(SendContactRequestResult {
            document: platform_document,
            recipient_id,
            account_reference,
        })
    }

    /// Sign a contact request without broadcasting it: what
    /// [`Self::send_contact_request`] submits, for a caller that keeps the
    /// signed transition so that a retry broadcasts the same bytes again
    /// rather than signing a second request.
    ///
    /// The transition takes the sender's next identity contract nonce for
    /// DashPay, as a send does.
    pub async fn sign_contact_request<S: Signer<IdentityPublicKey>, F, Fut, G, Gut, H, Hut>(
        &self,
        input: SendContactRequestInput<S>,
        ecdh_provider: EcdhProvider<F, Fut, G, Gut>,
        get_extended_public_key: H,
    ) -> Result<SignedContactRequest, Error>
    where
        F: FnOnce(&IdentityPublicKey, u32) -> Fut,
        Fut: std::future::Future<Output = Result<SecretKey, Error>>,
        G: FnOnce(&PublicKey) -> Gut,
        Gut: std::future::Future<Output = Result<[u8; 32], Error>>,
        H: FnOnce(u32) -> Hut,
        Hut: std::future::Future<Output = Result<Vec<u8>, Error>>,
    {
        let prepared = self
            .prepare_contact_request(input.contact_request.shielded_address.is_some())
            .await?;
        self.sign_prepared_contact_request(prepared, input, ecdh_provider, get_extended_public_key)
            .await
    }

    /// Sign a contact request like [`Sdk::sign_contact_request`], building the document as
    /// the `contactRequest` type of `prepared`, the snapshot the caller chose `input`'s
    /// signing key by.
    pub async fn sign_prepared_contact_request<
        S: Signer<IdentityPublicKey>,
        F,
        Fut,
        G,
        Gut,
        H,
        Hut,
    >(
        &self,
        prepared: PreparedContactRequest,
        input: SendContactRequestInput<S>,
        ecdh_provider: EcdhProvider<F, Fut, G, Gut>,
        get_extended_public_key: H,
    ) -> Result<SignedContactRequest, Error>
    where
        F: FnOnce(&IdentityPublicKey, u32) -> Fut,
        Fut: std::future::Future<Output = Result<SecretKey, Error>>,
        G: FnOnce(&PublicKey) -> Gut,
        Gut: std::future::Future<Output = Result<[u8; 32], Error>>,
        H: FnOnce(u32) -> Hut,
        Hut: std::future::Future<Output = Result<Vec<u8>, Error>>,
    {
        // Save values we need before moving contact_request
        let recipient_id = input.contact_request.recipient.id();
        let account_reference = input.contact_request.account_reference;

        // Create the contact request document (handles ECDH encryption internally)
        let result = self
            .build_contact_request(
                &prepared,
                input.contact_request,
                ecdh_provider,
                get_extended_public_key,
            )
            .await?;

        // Reuse the entropy that derived result.id during creation. Platform
        // consensus recomputes the document id from this entropy and rejects the
        // create transition unless it matches result.id, so a freshly generated
        // entropy here would always be rejected (InvalidDocumentTransitionIdError).
        let entropy = result.entropy;

        // Create the document from the result
        let document = Document::V0(DocumentV0 {
            contract_version: None,
            id: result.id,
            owner_id: result.owner_id,
            properties: result.properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
            moderated_at: None,
            moderated_by: None,
        });

        let state_transition = prepare_document_state_transition(
            &document,
            self,
            prepared.contact_request_document_type,
            Some(entropy.0),
            input.identity_public_key,
            None, // token payment info
            &input.signer,
            None, // settings
        )
        .await?;

        Ok(SignedContactRequest {
            state_transition,
            recipient_id,
            account_reference,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::DataContract;
    use dpp::secp256k1;
    use dpp::secp256k1::rand::{self, RngCore};
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;
    use platform_encryption::decrypt_shielded_address;
    use std::sync::Arc;

    fn dashpay(platform_version: &PlatformVersion) -> DataContract {
        load_system_data_contract(SystemDataContract::Dashpay, platform_version)
            .expect("the DashPay contract loads")
    }

    #[test]
    fn should_encrypt_a_shielded_address_for_a_dashpay_v3_contact_request() {
        let contract = dashpay(PlatformVersion::latest());
        let contact_request = contract
            .document_type_for_name("contactRequest")
            .expect("contactRequest");
        let shared_key = [7u8; 32];
        let address = [3u8; SHIELDED_ADDRESS_LEN];
        let mut rng = StdRng::seed_from_u64(1);

        let first = encrypted_shielded_address(contact_request, &shared_key, &address, &mut rng)
            .expect("DashPay v3 carries the address");
        let second = encrypted_shielded_address(contact_request, &shared_key, &address, &mut rng)
            .expect("DashPay v3 carries the address");

        // What the contract accepts for the property: an IV and three AES blocks.
        assert_eq!(first.len(), 64);
        assert_eq!(
            decrypt_shielded_address(&shared_key, &first).expect("the field decrypts"),
            address
        );
        // Each call draws its own IV, so the same address never encrypts the same way twice.
        assert_ne!(first[..16], second[..16]);
        assert_ne!(first, second);
    }

    #[test]
    fn should_tell_whether_a_dashpay_contract_gives_contact_requests_a_shielded_address() {
        let before = dashpay(PlatformVersion::get(14).expect("protocol version 14"));
        assert!(!dashpay_contact_requests_carry_shielded_address(&before));
        assert!(dashpay_contact_requests_carry_shielded_address(&dashpay(
            PlatformVersion::latest()
        )));
    }

    #[test]
    fn should_refuse_a_shielded_address_where_contact_requests_have_no_such_property() {
        let contract = dashpay(PlatformVersion::get(14).expect("protocol version 14"));
        let contact_request = contract
            .document_type_for_name("contactRequest")
            .expect("contactRequest");
        let mut rng = StdRng::seed_from_u64(1);

        assert!(encrypted_shielded_address(
            contact_request,
            &[7u8; 32],
            &[3u8; SHIELDED_ADDRESS_LEN],
            &mut rng
        )
        .is_err());
    }

    #[test]
    fn test_ecdh_encryption_produces_correct_size() {
        // Test that ECDH encryption produces the correct output sizes
        let (secret1, _public1) = secp256k1::generate_keypair(&mut rand::rng());
        let (_secret2, public2) = secp256k1::generate_keypair(&mut rand::rng());

        // Derive shared key
        let shared_key = derive_shared_key_ecdh(&secret1, &public2);

        // Generate random IVs
        let mut xpub_iv = [0u8; 16];
        let mut label_iv = [0u8; 16];
        rand::rng().fill_bytes(&mut xpub_iv);
        rand::rng().fill_bytes(&mut label_iv);

        // Test extended public key encryption: the DIP-15 compact plaintext is
        // 69 bytes (parentFingerprint ‖ chainCode ‖ pubKey) → 96 bytes with IV
        // + PKCS7 padding. (A 78-byte BIP32 xpub would also pad to 96, but the
        // contract + reference clients require exactly the 69-byte compact.)
        let xpub_data = vec![0x04; COMPACT_XPUB_LEN];
        let encrypted_xpub = encrypt_extended_public_key(&shared_key, &xpub_iv, &xpub_data);
        assert_eq!(
            encrypted_xpub.len(),
            96,
            "Encrypted xpub should be 96 bytes (16-byte IV + 80 bytes encrypted data)"
        );

        // Test account label encryption (various sizes -> 48-80 bytes with IV + PKCS7 padding)
        let label = "My DashPay Account";
        let encrypted_label = encrypt_account_label(&shared_key, &label_iv, label);
        assert!(
            encrypted_label.len() >= 48 && encrypted_label.len() <= 80,
            "Encrypted label should be 48-80 bytes, got {}",
            encrypted_label.len()
        );
    }

    #[test]
    fn test_auto_accept_proof_validation() {
        // Test that auto accept proof must be 38-102 bytes if provided
        let invalid_sizes = vec![0, 37, 103, 200];
        let valid_sizes = vec![38, 70, 102];

        for size in invalid_sizes {
            let proof = vec![0u8; size];
            assert!(
                proof.len() < 38 || proof.len() > 102,
                "Size {} should be invalid",
                size
            );
        }

        for size in valid_sizes {
            let proof = vec![0u8; size];
            assert!(
                proof.len() >= 38 && proof.len() <= 102,
                "Size {} should be valid",
                size
            );
        }
    }

    #[test]
    fn contact_request_result_entropy_derives_returned_id() {
        // Regression for G2 entropy mismatch: the document id returned by
        // create_contact_request must be derivable from the entropy carried in
        // ContactRequestResult. send_contact_request reuses ContactRequestResult::entropy
        // when broadcasting, and platform consensus rejects the create transition
        // (InvalidDocumentTransitionIdError) unless
        //   generate_document_id_v0(contract, owner, "contactRequest", entropy) == base.id.
        //
        // Without the `entropy` field on ContactRequestResult,
        // send_contact_request would generate fresh entropy E2 != E1 and this
        // invariant could not even be expressed. This test pins it.
        let mut rng = StdRng::seed_from_u64(0x6732_4732); // deterministic, no network
        let entropy = Bytes32::new(rng.random());

        let contract_id = Identifier::from([1u8; 32]);
        let owner_id = Identifier::from([2u8; 32]);

        let id = Document::generate_document_id_v0(
            &contract_id,
            &owner_id,
            "contactRequest",
            entropy.as_slice(),
        );

        let result = ContactRequestResult {
            id,
            owner_id,
            properties: BTreeMap::new(),
            entropy,
        };

        // The entropy that send_contact_request will broadcast must regenerate the
        // exact id that was returned at creation time.
        let regenerated = Document::generate_document_id_v0(
            &contract_id,
            &result.owner_id,
            "contactRequest",
            result.entropy.as_slice(),
        );
        assert_eq!(
            regenerated, result.id,
            "entropy carried in ContactRequestResult must derive the returned document id"
        );
    }

    #[test]
    fn recipient_key_purpose_accepts_decryption_and_encryption() {
        // G15: the recipient-key assertion must accept DECRYPTION (our
        // original convention / newest cohort) OR ENCRYPTION (the dominant
        // mobile cohort, whose identities have no DECRYPTION key and reference
        // their ENCRYPTION key for recipientKeyIndex). Accepting only
        // DECRYPTION would make sending to a mobile recipient error with
        // "Recipient key ... is not a decryption key".
        assert!(
            recipient_key_purpose_is_valid(Purpose::DECRYPTION),
            "DECRYPTION recipient key must remain valid"
        );
        assert!(
            recipient_key_purpose_is_valid(Purpose::ENCRYPTION),
            "ENCRYPTION recipient key (mobile cohort) must be accepted"
        );
    }

    #[test]
    fn mint_side_still_refuses_authentication_and_transfer() {
        // What we CREATE stays strict: reusing a signing or fund-authorizing
        // key for ECDH is poor key separation, and no new document needs to.
        // Widening the receive-side acceptance below must never leak into the
        // key we pick for our own outgoing requests.
        assert!(!recipient_key_purpose_is_valid(Purpose::AUTHENTICATION));
        assert!(!recipient_key_purpose_is_valid(Purpose::TRANSFER));
    }

    #[test]
    fn sender_key_purpose_is_unchanged_encryption_only() {
        // Sender side stays strict: only ENCRYPTION (per the task, the
        // sender-side assertion is unchanged).
        assert!(sender_key_purpose_is_valid(Purpose::ENCRYPTION));
        assert!(!sender_key_purpose_is_valid(Purpose::DECRYPTION));
        assert!(!sender_key_purpose_is_valid(Purpose::AUTHENTICATION));
    }

    #[test]
    fn receive_side_accepts_the_legacy_dashj_cohort() {
        // Regression guard for the mainnet legacy cohort: inbound requests
        // minted by the Android/dashj client reference the recipient's
        // AUTHENTICATION (key ids 0-2) or TRANSFER (key id 3) key. Rejecting
        // them made every pre-iOS contact permanently unpayable — the document
        // is immutable, so no user action could ever fix it.
        for purpose in [
            Purpose::DECRYPTION,
            Purpose::ENCRYPTION,
            Purpose::AUTHENTICATION,
            Purpose::TRANSFER,
        ] {
            assert!(
                recipient_key_purpose_is_acceptable_on_receive(purpose),
                "{purpose:?} recipient key must be accepted from an on-chain document"
            );
        }
        // The sender side of the same legacy documents pairs AUTHENTICATION
        // with AUTHENTICATION; ENCRYPTION remains the modern convention.
        assert!(sender_key_purpose_is_acceptable_on_receive(
            Purpose::ENCRYPTION
        ));
        assert!(sender_key_purpose_is_acceptable_on_receive(
            Purpose::AUTHENTICATION
        ));
    }

    #[test]
    fn receive_side_still_refuses_node_operational_purposes() {
        // Not observed on chain for DashPay — widening is evidence-driven, so
        // these stay out until something real needs them. A rejection here is
        // a skip-and-retry purpose mismatch, never a permanently broken
        // channel, so a later widening can still recover those contacts.
        for purpose in [Purpose::SYSTEM, Purpose::VOTING, Purpose::OWNER] {
            assert!(!recipient_key_purpose_is_acceptable_on_receive(purpose));
            assert!(!sender_key_purpose_is_acceptable_on_receive(purpose));
        }
        // TRANSFER is accepted for the recipient (legacy key id 3) but has
        // never been seen on the sender side.
        assert!(!sender_key_purpose_is_acceptable_on_receive(
            Purpose::TRANSFER
        ));
    }

    #[test]
    fn test_ecdh_shared_secret_symmetry() {
        // Test that both parties derive the same shared secret
        let (secret_alice, public_alice) = secp256k1::generate_keypair(&mut rand::rng());
        let (secret_bob, public_bob) = secp256k1::generate_keypair(&mut rand::rng());

        // Alice derives shared secret using her private key and Bob's public key
        let shared_alice = derive_shared_key_ecdh(&secret_alice, &public_bob);

        // Bob derives shared secret using his private key and Alice's public key
        let shared_bob = derive_shared_key_ecdh(&secret_bob, &public_alice);

        // Both should derive the same shared secret
        assert_eq!(
            shared_alice, shared_bob,
            "Both parties should derive the same shared secret"
        );
    }

    /// A context provider holding `contract`, if any, for every id and protocol version, and
    /// otherwise the contract last registered with it.
    #[derive(Default)]
    struct DashPayProvider {
        contract: Option<Arc<DataContract>>,
        registered: std::sync::Mutex<Option<Arc<DataContract>>>,
    }

    impl dash_context_provider::ContextProvider for DashPayProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &dpp::version::PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, dash_context_provider::ContextProviderError>
        {
            Ok(self
                .contract
                .clone()
                .or_else(|| self.registered.lock().expect("the lock").clone()))
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<
            Option<dpp::data_contract::TokenConfiguration>,
            dash_context_provider::ContextProviderError,
        > {
            Ok(None)
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], dash_context_provider::ContextProviderError> {
            Err(dash_context_provider::ContextProviderError::InvalidQuorum(
                "no quorum in this test".to_string(),
            ))
        }

        fn get_platform_activation_height(
            &self,
        ) -> Result<dpp::prelude::CoreBlockHeight, dash_context_provider::ContextProviderError>
        {
            Ok(1)
        }

        fn register_data_contract(&self, contract: Arc<DataContract>) {
            *self.registered.lock().expect("the lock") = Some(contract);
        }
    }

    /// The latest DashPay contract with its `contactRequest` type requiring a CRITICAL key,
    /// where the bundled one requires HIGH.
    fn dashpay_with_critical_contact_request() -> DataContract {
        use crate::platform::system_data_contract::tests::with_document_security_level;
        use dpp::identity::SecurityLevel;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use dpp::version::PlatformVersion;

        with_document_security_level(
            load_system_data_contract(SystemDataContract::Dashpay, PlatformVersion::latest())
                .expect("the DashPay contract loads"),
            "contactRequest",
            SecurityLevel::CRITICAL,
        )
    }

    #[tokio::test]
    async fn should_prepare_the_signing_requirement_from_the_dashpay_contract_the_provider_holds() {
        use dpp::identity::SecurityLevel;
        use dpp::version::PlatformVersion;

        // No network expectation: a fetch would fail the test
        let held = dashpay_with_critical_contact_request();
        let sdk = crate::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .with_context_provider(DashPayProvider {
                contract: Some(Arc::new(held.clone())),
                ..Default::default()
            })
            .build()
            .expect("the mock SDK builds");

        let prepared = sdk
            .prepare_contact_request(false)
            .await
            .expect("the contact request is prepared");

        assert_eq!(
            prepared.signing_security_level_requirement(),
            SecurityLevel::CRITICAL,
            "the key is judged by the contract the document is built against"
        );
        assert_eq!(prepared.dashpay_contract_id(), held.id());
    }

    #[tokio::test]
    async fn should_prepare_the_signing_requirement_from_the_fetched_dashpay_contract() {
        use dpp::identity::SecurityLevel;
        use dpp::version::PlatformVersion;

        let mut sdk = crate::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .with_context_provider(DashPayProvider::default())
            .build()
            .expect("the mock SDK builds");
        let stored = dashpay_with_critical_contact_request();
        sdk.mock()
            .expect_fetch(stored.id(), Some(stored.clone()))
            .await
            .expect("the DashPay fetch is expected");

        let prepared = sdk
            .prepare_contact_request(false)
            .await
            .expect("the contact request is prepared");

        assert_eq!(
            prepared.signing_security_level_requirement(),
            SecurityLevel::CRITICAL
        );
        assert_eq!(prepared.dashpay_contract_id(), stored.id());
    }

    /// The DashPay contract the proof confirming `prepared`'s request is verified with: what
    /// the SDK's context provider answers at the SDK's version, the lookup
    /// `Waitable::wait_for_response` hands Drive's state-transition proof check.
    fn confirmation_contract(
        sdk: &crate::Sdk,
        prepared: &PreparedContactRequest,
    ) -> Option<Arc<DataContract>> {
        use dash_context_provider::ContextProvider;

        sdk.context_provider()
            .expect("a context provider")
            .get_data_contract(&prepared.dashpay_contract_id(), sdk.version())
            .expect("the provider answers")
    }

    #[tokio::test]
    async fn should_register_the_fetched_dashpay_contract_for_the_confirmation_proof() {
        use dpp::version::PlatformVersion;

        let mut sdk = crate::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .with_context_provider(DashPayProvider::default())
            .build()
            .expect("the mock SDK builds");
        let stored = dashpay_with_critical_contact_request();
        sdk.mock()
            .expect_fetch(stored.id(), Some(stored.clone()))
            .await
            .expect("the DashPay fetch is expected");

        let prepared = sdk
            .prepare_contact_request(false)
            .await
            .expect("the contact request is prepared");

        let resolved =
            confirmation_contract(&sdk, &prepared).expect("the fetched contract is registered");
        assert_eq!(*resolved, stored);
    }

    /// The trusted HTTP provider the Swift and Kotlin wallets run (directly or under the
    /// adaptive one) confirms a request against the DashPay contract it was built from, whether
    /// it bundles DashPay or the SDK fetched it.
    #[tokio::test]
    async fn should_confirm_against_the_contract_the_request_was_built_from_on_the_trusted_provider(
    ) {
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
        use dpp::version::PlatformVersion;
        use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
        use std::num::NonZeroUsize;

        let mut sdk = crate::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("the mock SDK builds");
        sdk.set_context_provider(
            TrustedHttpContextProvider::new_with_url(
                sdk.network,
                "https://127.0.0.1:1".to_string(),
                NonZeroUsize::new(8).expect("non-zero"),
            )
            .expect("the trusted provider builds"),
        )
        .expect("the provider serves the SDK's network");
        let stored = dashpay_with_critical_contact_request();
        sdk.mock()
            .expect_fetch(stored.id(), Some(stored.clone()))
            .await
            .expect("the DashPay fetch is expected");

        let prepared = sdk
            .prepare_contact_request(false)
            .await
            .expect("the contact request is prepared");

        let resolved = confirmation_contract(&sdk, &prepared)
            .expect("the confirmation resolves the DashPay contract");
        assert_eq!(
            resolved
                .document_type_for_name(CONTACT_REQUEST)
                .expect("the contactRequest type")
                .security_level_requirement(),
            prepared.signing_security_level_requirement()
        );
    }

    /// The SDK's protocol version and whether the contract it resolves for a contact
    /// request gives the request a shielded address property.
    #[cfg(feature = "mocks")]
    async fn resolved_schema(sdk: &Sdk, with_shielded_address: bool) -> (u32, bool) {
        let contract = sdk
            .dashpay_contract_for_contact_request(with_shielded_address)
            .await
            .expect("the DashPay contract");
        let carries_address = contract
            .document_type_for_name("contactRequest")
            .expect("contactRequest")
            .properties()
            .contains_key(ENCRYPTED_SHIELDED_ADDRESS);
        (sdk.protocol_version_number(), carries_address)
    }

    /// An SDK seeded below protocol version 15 on a network running the latest version:
    /// asked for a request with a shielded address, it learns the network's version first,
    /// so the contract it resolves has the property for the address.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_learn_the_network_version_before_resolving_the_schema_of_a_shielded_address() {
        let directory = tempfile::tempdir().expect("a mock dump directory");
        let mut sdk: Sdk = crate::SdkBuilder::new_mock()
            .with_initial_version(PlatformVersion::get(14).expect("protocol version 14"))
            .with_dump_dir(directory.path())
            .build()
            .expect("a mock sdk");
        crate::sdk::test::expect_epoch_refresh(&mut sdk).await;
        let current = dashpay(PlatformVersion::latest());
        sdk.mock()
            .expect_fetch(current.id(), Some(current.clone()))
            .await
            .expect("the DashPay fetch is expected");

        assert_eq!(
            resolved_schema(&sdk, true).await,
            (PlatformVersion::latest().protocol_version, true)
        );
    }

    /// A request without an address resolves the contract as before. No answer to a
    /// version lookup is staged here, so the call succeeds only if it makes none.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_resolve_the_schema_of_a_request_without_an_address_at_the_seeded_version() {
        let seeded = PlatformVersion::get(14).expect("protocol version 14");
        let directory = tempfile::tempdir().expect("a mock dump directory");
        let mut sdk: Sdk = crate::SdkBuilder::new_mock()
            .with_initial_version(seeded)
            .with_dump_dir(directory.path())
            .build()
            .expect("a mock sdk");
        let held = dashpay(seeded);
        sdk.mock()
            .expect_fetch(held.id(), Some(held.clone()))
            .await
            .expect("the DashPay fetch is expected");

        let (_, carries_address) = resolved_schema(&sdk, false).await;
        assert!(!carries_address, "the contract resolved is the one held");
    }

    /// The SDK-side ECDH branch, never taken: the tests hand the shared key over.
    #[cfg(feature = "mocks")]
    type NoPrivateKey = fn(&IdentityPublicKey, u32) -> std::future::Ready<Result<SecretKey, Error>>;

    /// The shared key the tests' ECDH callback answers with.
    #[cfg(feature = "mocks")]
    const SHARED_KEY: [u8; 32] = [7; 32];

    /// A request from an identity with ENCRYPTION key 1 to one with DECRYPTION key 2,
    /// carrying `shielded_address`.
    #[cfg(feature = "mocks")]
    fn contact_request_input(
        shielded_address: Option<[u8; SHIELDED_ADDRESS_LEN]>,
    ) -> ContactRequestInput {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::v0::IdentityV0;
        use dpp::identity::{KeyType, SecurityLevel};
        use dpp::platform_value::BinaryData;

        let identity = |id: u8, key_id: u32, purpose: Purpose| {
            let secret = SecretKey::from_secret_bytes([id; 32]).expect("a valid secret key");
            let key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: key_id,
                purpose,
                security_level: SecurityLevel::MEDIUM,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(PublicKey::from_secret_key(&secret).serialize().to_vec()),
                disabled_at: None,
            });
            Identity::V0(IdentityV0 {
                id: Identifier::from([id; 32]),
                public_keys: BTreeMap::from([(key_id, key)]),
                balance: 0,
                revision: 0,
            })
        };
        ContactRequestInput {
            sender_identity: identity(1, 1, Purpose::ENCRYPTION),
            recipient: RecipientIdentity::Identity(identity(2, 2, Purpose::DECRYPTION)),
            sender_key_index: 1,
            recipient_key_index: 2,
            account_reference: 5,
            account_label: None,
            auto_accept_proof: None,
            shielded_address,
        }
    }

    /// `create_contact_request` for `input`, and whether it asked the host for the shared
    /// secret and for the extended public key.
    #[cfg(feature = "mocks")]
    async fn created_contact_request(
        sdk: &Sdk,
        input: ContactRequestInput,
    ) -> (Result<ContactRequestResult, Error>, bool, bool) {
        use std::sync::atomic::{AtomicBool, Ordering};

        let (asked_secret, asked_xpub) = (AtomicBool::new(false), AtomicBool::new(false));
        let created = sdk
            .create_contact_request(
                input,
                EcdhProvider::<NoPrivateKey, _, _, _>::ClientSide {
                    get_shared_secret: |_: &PublicKey| {
                        asked_secret.store(true, Ordering::SeqCst);
                        std::future::ready(Ok(SHARED_KEY))
                    },
                },
                |_| {
                    asked_xpub.store(true, Ordering::SeqCst);
                    std::future::ready(Ok(vec![4u8; COMPACT_XPUB_LEN]))
                },
            )
            .await;
        (
            created,
            asked_secret.load(Ordering::SeqCst),
            asked_xpub.load(Ordering::SeqCst),
        )
    }

    /// A mock SDK seeded at protocol version 14 that holds `contract` as the DashPay it
    /// resolves, with its dump directory.
    #[cfg(feature = "mocks")]
    async fn sdk_at_14_resolving(contract: &DataContract) -> (Sdk, tempfile::TempDir) {
        sdk_resolving(
            PlatformVersion::get(14).expect("protocol version 14"),
            contract,
        )
        .await
    }

    /// A mock SDK seeded at `version` that holds `contract` as the DashPay it resolves,
    /// with its dump directory.
    #[cfg(feature = "mocks")]
    async fn sdk_resolving(
        version: &'static PlatformVersion,
        contract: &DataContract,
    ) -> (Sdk, tempfile::TempDir) {
        let directory = tempfile::tempdir().expect("a mock dump directory");
        let mut sdk: Sdk = crate::SdkBuilder::new_mock()
            .with_initial_version(version)
            .with_dump_dir(directory.path())
            .build()
            .expect("a mock sdk");
        sdk.mock()
            .expect_fetch(contract.id(), Some(contract.clone()))
            .await
            .expect("the DashPay fetch is expected");
        (sdk, directory)
    }

    /// The public creation path puts the address on the request. On a network that carries
    /// one, the document's `encryptedShieldedAddress` decrypts under the request's shared
    /// key to the address given, and its IV is not the extended public key's.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_put_the_encrypted_shielded_address_on_a_created_contact_request() {
        let (mut sdk, _directory) = sdk_at_14_resolving(&dashpay(PlatformVersion::latest())).await;
        crate::sdk::test::expect_epoch_refresh(&mut sdk).await;
        let address = [3u8; SHIELDED_ADDRESS_LEN];

        let (created, asked_secret, asked_xpub) =
            created_contact_request(&sdk, contact_request_input(Some(address))).await;
        let created = created.expect("the request is created");

        assert!(
            asked_secret && asked_xpub,
            "the host answered for the request"
        );
        let bytes = |name: &str| {
            created
                .properties
                .get(name)
                .unwrap_or_else(|| panic!("the request carries {name}"))
                .to_binary_bytes()
                .expect("bytes")
        };
        let encrypted = bytes(ENCRYPTED_SHIELDED_ADDRESS);
        assert_eq!(
            decrypt_shielded_address(&SHARED_KEY, &encrypted).expect("the field decrypts"),
            address
        );
        assert_ne!(encrypted[..16], bytes("encryptedPublicKey")[..16]);
    }

    /// A request with an address on a network whose version the SDK cannot raise to 15 (no
    /// answer to the version lookup is staged) is refused before the host is asked for the
    /// shared secret or the extended public key.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_refuse_a_shielded_address_before_asking_the_host_for_keys() {
        let seeded = PlatformVersion::get(14).expect("protocol version 14");
        let (sdk, _directory) = sdk_at_14_resolving(&dashpay(seeded)).await;

        let (created, asked_secret, asked_xpub) = created_contact_request(
            &sdk,
            contact_request_input(Some([3u8; SHIELDED_ADDRESS_LEN])),
        )
        .await;

        assert!(created.is_err(), "the request is refused");
        assert!(!asked_secret, "the shared secret was not asked for");
        assert!(!asked_xpub, "the extended public key was not asked for");
    }

    /// An SDK at a protocol version that carries the address, resolving a DashPay contract
    /// whose contact requests have no property for it: the version gate passes, and the
    /// schema refuses the address before the host is asked for the shared secret or the
    /// extended public key.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_refuse_an_address_the_schema_lacks_before_asking_the_host_for_keys() {
        let without_property = dashpay(PlatformVersion::get(14).expect("protocol version 14"));
        let (sdk, _directory) = sdk_resolving(PlatformVersion::latest(), &without_property).await;

        let (created, asked_secret, asked_xpub) = created_contact_request(
            &sdk,
            contact_request_input(Some([3u8; SHIELDED_ADDRESS_LEN])),
        )
        .await;

        let refusal = created.expect_err("the request is refused").to_string();
        assert!(refusal.contains(ENCRYPTED_SHIELDED_ADDRESS), "{refusal}");
        assert!(!asked_secret, "the shared secret was not asked for");
        assert!(!asked_xpub, "the extended public key was not asked for");
    }

    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_refuse_preparing_an_address_the_schema_lacks() {
        let without_property = dashpay(PlatformVersion::get(14).expect("protocol version 14"));
        let (sdk, _directory) = sdk_resolving(PlatformVersion::latest(), &without_property).await;

        let refusal = sdk
            .prepare_contact_request(true)
            .await
            .expect_err("preparing an unsupported shielded address is refused")
            .to_string();
        assert!(refusal.contains(ENCRYPTED_SHIELDED_ADDRESS), "{refusal}");
    }

    /// A request sent with a shielded address the prepare call did not announce, or without
    /// one it did, is refused as an invalid request before the host is asked for any key.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_refuse_a_request_whose_address_does_not_match_its_preparation() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let latest = PlatformVersion::latest();
        for (prepared_with, address) in [(false, Some([3u8; SHIELDED_ADDRESS_LEN])), (true, None)] {
            let (sdk, _directory) = sdk_resolving(latest, &dashpay(latest)).await;
            let prepared = sdk
                .prepare_contact_request(prepared_with)
                .await
                .expect("the contact request is prepared");
            assert_eq!(prepared.with_shielded_address(), prepared_with);

            let asked = AtomicBool::new(false);
            let built = sdk
                .build_contact_request(
                    &prepared,
                    contact_request_input(address),
                    EcdhProvider::<NoPrivateKey, _, _, _>::ClientSide {
                        get_shared_secret: |_: &PublicKey| {
                            asked.store(true, Ordering::SeqCst);
                            std::future::ready(Ok(SHARED_KEY))
                        },
                    },
                    |_| {
                        asked.store(true, Ordering::SeqCst);
                        std::future::ready(Ok(vec![4u8; COMPACT_XPUB_LEN]))
                    },
                )
                .await;

            assert!(
                matches!(built, Err(Error::InvalidRequest(_))),
                "prepared with an address: {prepared_with}, got {:?}",
                built.as_ref().err()
            );
            assert!(!asked.load(Ordering::SeqCst), "the host was not asked");
        }
    }

    /// The same SDK creates a request without an address, as before: the host answers for
    /// it and the document has no `encryptedShieldedAddress`.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_create_a_contact_request_without_an_address_below_protocol_version_15() {
        let seeded = PlatformVersion::get(14).expect("protocol version 14");
        let (sdk, _directory) = sdk_at_14_resolving(&dashpay(seeded)).await;

        let (created, asked_secret, asked_xpub) =
            created_contact_request(&sdk, contact_request_input(None)).await;
        let created = created.expect("the request is created");

        assert!(
            asked_secret && asked_xpub,
            "the host answered for the request"
        );
        assert!(!created.properties.contains_key(ENCRYPTED_SHIELDED_ADDRESS));
    }
}
