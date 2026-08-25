//! Contact request creation and state transition helpers
//!
//! Implements DIP-15 DashPay contact request functionality

use crate::platform::transition::put_document::PutDocument;
use crate::platform::Document;
use crate::{Error, Sdk};
use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use dpp::dashcore::secp256k1::rand::{RngCore, SeedableRng};
use dpp::dashcore::secp256k1::{PublicKey, SecretKey};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::DocumentV0;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::Purpose;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::platform_value::{Bytes32, Value};
use dpp::prelude::Identifier;
use platform_encryption::{
    derive_shared_key_ecdh, encrypt_account_label, encrypt_extended_public_key, COMPACT_XPUB_LEN,
};
use std::collections::BTreeMap;

use dpp::data_contract::DataContract;

/// Already-derived crypto material and metadata for a DIP-15
/// `contactRequest` document.
///
/// Everything here is plain data: the ECDH/encryption that produced
/// `encrypted_public_key` and `encrypted_account_label`, and the randomness
/// that produced `entropy`, happen in the caller.
#[derive(Debug, Clone)]
pub struct ContactRequestDocumentParams {
    /// The sender's identity id (the document owner)
    pub sender_id: Identifier,
    /// The recipient's identity id (`toUserId`)
    pub recipient_id: Identifier,
    /// The sender's encryption key index used for ECDH
    pub sender_key_index: u32,
    /// The recipient's key index used for ECDH
    pub recipient_key_index: u32,
    /// Reference to the DashPay receiving account
    pub account_reference: u32,
    /// ECDH-encrypted extended public key: exactly 96 bytes
    /// (16-byte IV + 80 bytes of encrypted DIP-15 compact xpub)
    pub encrypted_public_key: Vec<u8>,
    /// Optional encrypted account label: 48-80 bytes
    /// (16-byte IV + 32-64 bytes of encrypted data)
    pub encrypted_account_label: Option<Vec<u8>>,
    /// Optional auto-accept proof (38-102 bytes) - not encrypted
    pub auto_accept_proof: Option<Vec<u8>>,
    /// The entropy that derives the document id; the same entropy must be
    /// attached to the create transition, or platform consensus rejects it
    /// with `InvalidDocumentTransitionIdError`.
    pub entropy: [u8; 32],
}

/// Validate the size of a DIP-15 `autoAcceptProof` (38-102 bytes).
pub fn validate_auto_accept_proof(proof: &[u8]) -> Result<(), Error> {
    if proof.len() < 38 || proof.len() > 102 {
        return Err(Error::Generic(format!(
            "autoAcceptProof must be 38-102 bytes, got {}",
            proof.len()
        )));
    }
    Ok(())
}

/// Build a DIP-15 `contactRequest` document from already-derived crypto
/// material.
///
/// This is the pure document-assembly half of [`Sdk::create_contact_request`]:
/// the document id derives from `params.entropy`, the owner is
/// `params.sender_id`, and the properties are exactly the fields the DashPay
/// contract defines. Broadcast the returned document with the same
/// `params.entropy` attached to the create transition, or platform consensus
/// rejects it with `InvalidDocumentTransitionIdError`.
///
/// Returns the assembled `contactRequest` [`Document`].
pub fn build_contact_request_document(
    contract: &DataContract,
    params: ContactRequestDocumentParams,
) -> Result<Document, Error> {
    if let Some(ref proof) = params.auto_accept_proof {
        validate_auto_accept_proof(proof)?;
    }

    // Validate encrypted public key size (must be exactly 96 bytes: 16-byte IV + 80-byte encrypted data)
    if params.encrypted_public_key.len() != 96 {
        return Err(Error::Generic(format!(
            "Encrypted public key size mismatch: expected 96 bytes, got {}",
            params.encrypted_public_key.len()
        )));
    }

    // Validate encrypted label size (48-80 bytes: 16-byte IV + 32-64 byte encrypted data)
    if let Some(ref label) = params.encrypted_account_label {
        if label.len() < 48 || label.len() > 80 {
            return Err(Error::Generic(format!(
                "Encrypted account label size out of range: expected 48-80 bytes, got {}",
                label.len()
            )));
        }
    }

    let contact_request_document_type =
        contract
            .document_type_for_name("contactRequest")
            .map_err(|_| {
                Error::Generic("DashPay contactRequest document type not found".to_string())
            })?;

    let document_id = Document::generate_document_id_v0(
        &contract.id(),
        &params.sender_id,
        contact_request_document_type.name(),
        params.entropy.as_slice(),
    );

    let mut properties = BTreeMap::new();
    properties.insert(
        "toUserId".to_string(),
        Value::Identifier(params.recipient_id.to_buffer()),
    );
    properties.insert(
        "encryptedPublicKey".to_string(),
        Value::Bytes(params.encrypted_public_key),
    );
    properties.insert(
        "senderKeyIndex".to_string(),
        Value::U32(params.sender_key_index),
    );
    properties.insert(
        "recipientKeyIndex".to_string(),
        Value::U32(params.recipient_key_index),
    );
    properties.insert(
        "accountReference".to_string(),
        Value::U32(params.account_reference),
    );

    if let Some(label) = params.encrypted_account_label {
        properties.insert("encryptedAccountLabel".to_string(), Value::Bytes(label));
    }
    if let Some(proof) = params.auto_accept_proof {
        properties.insert("autoAcceptProof".to_string(), Value::Bytes(proof));
    }

    Ok(Document::V0(DocumentV0 {
        contract_version: None,
        id: document_id,
        owner_id: params.sender_id,
        properties,
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
    }))
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
}

/// Result of creating a contact request document
#[derive(Debug)]
pub struct ContactRequestResult {
    /// The assembled `contactRequest` document, not yet submitted to the
    /// platform. Its id derives from `entropy` and its owner is the sender.
    pub document: Document,
    /// The entropy used to derive the document id.
    ///
    /// This must be reused when broadcasting the document so that the
    /// document id computed at creation matches the id platform consensus
    /// recomputes from the entropy (otherwise the create transition is
    /// rejected with `InvalidDocumentTransitionIdError`).
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

/// Whether `purpose` is acceptable for the `senderKeyIndex` key of a contact
/// request **we are about to mint**. The sender always references its own
/// ENCRYPTION key.
///
/// Mint-side only — see [`sender_key_purpose_is_acceptable_on_receive`] for
/// what we accept from documents already on chain.
fn sender_key_purpose_is_valid(purpose: Purpose) -> bool {
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
        // Validate auto accept proof size if provided. The builder
        // validates again, but checking here first keeps the failure local —
        // before the recipient fetch and ECDH work below.
        if let Some(ref proof) = input.auto_accept_proof {
            validate_auto_accept_proof(proof)?;
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
        let mut rng = StdRng::from_entropy();
        let mut xpub_iv = [0u8; 16];
        rng.fill_bytes(&mut xpub_iv);

        // Encrypt the extended public key (includes IV prepended). The
        // builder rejects any ciphertext that isn't exactly 96 bytes
        // (16-byte IV + 80-byte encrypted data).
        let encrypted_public_key =
            encrypt_extended_public_key(&shared_key, &xpub_iv, &extended_public_key);

        // Encrypt the account label if provided (includes IV prepended). The
        // builder rejects any ciphertext outside 48-80 bytes
        // (16-byte IV + 32-64 byte encrypted data).
        let encrypted_account_label = input.account_label.as_ref().map(|label| {
            let mut label_iv = [0u8; 16];
            rng.fill_bytes(&mut label_iv);
            encrypt_account_label(&shared_key, &label_iv, label)
        });

        // Fetch DashPay contract
        let dashpay_contract = self.fetch_dashpay_contract().await?;

        // Generate entropy for document ID
        let mut rng = StdRng::from_entropy();
        let entropy = Bytes32::random_with_rng(&mut rng);

        // Assemble the document in the pure builder above, keeping document
        // assembly separate from this networked flow.
        let document = build_contact_request_document(
            &dashpay_contract,
            ContactRequestDocumentParams {
                sender_id: input.sender_identity.id().to_owned(),
                recipient_id: recipient_identity.id().to_owned(),
                sender_key_index: input.sender_key_index,
                recipient_key_index: input.recipient_key_index,
                account_reference: input.account_reference,
                encrypted_public_key,
                encrypted_account_label,
                auto_accept_proof: input.auto_accept_proof,
                entropy: entropy.0,
            },
        )?;

        // Return the assembled document together with the entropy that
        // derived its id so the broadcast path can reuse it.
        Ok(ContactRequestResult { document, entropy })
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
        // Save values we need before moving contact_request
        let recipient_id = input.contact_request.recipient.id();
        let account_reference = input.contact_request.account_reference;

        // Create the contact request document (handles ECDH encryption internally)
        let result = self
            .create_contact_request(
                input.contact_request,
                ecdh_provider,
                get_extended_public_key,
            )
            .await?;

        // Get the DashPay contract for the document type
        let dashpay_contract = self.fetch_dashpay_contract().await?;
        let contact_request_document_type = dashpay_contract
            .document_type_for_name("contactRequest")
            .map_err(|_| {
                Error::Generic("DashPay contactRequest document type not found".to_string())
            })?;

        // Reuse the entropy that derived the document id during creation.
        // Platform consensus recomputes the id from this entropy and rejects
        // the create transition unless it matches, so a freshly generated
        // entropy here would always be rejected (InvalidDocumentTransitionIdError).
        let entropy = result.entropy;
        let document = result.document;

        // Submit the document to the platform
        let platform_document = document
            .put_to_platform_and_wait_for_response(
                self,
                contact_request_document_type.to_owned_document_type(),
                Some(entropy.0),
                input.identity_public_key,
                None, // token payment info
                &input.signer,
                None, // settings
            )
            .await?;

        // Return the result with recipient ID and account reference we saved earlier
        Ok(SendContactRequestResult {
            document: platform_document,
            recipient_id,
            account_reference,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::dashcore::secp256k1::rand::{self, RngCore};
    use dpp::dashcore::secp256k1::Secp256k1;
    use dpp::document::DocumentV0Getters;

    #[test]
    fn test_ecdh_encryption_produces_correct_size() {
        // Test that ECDH encryption produces the correct output sizes
        let secp = Secp256k1::new();
        let (secret1, _public1) = secp.generate_keypair(&mut rand::thread_rng());
        let (_secret2, public2) = secp.generate_keypair(&mut rand::thread_rng());

        // Derive shared key
        let shared_key = derive_shared_key_ecdh(&secret1, &public2);

        // Generate random IVs
        let mut xpub_iv = [0u8; 16];
        let mut label_iv = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut xpub_iv);
        rand::thread_rng().fill_bytes(&mut label_iv);

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
        let entropy = Bytes32::random_with_rng(&mut rng);

        let contract_id = Identifier::from([1u8; 32]);
        let owner_id = Identifier::from([2u8; 32]);

        let id = Document::generate_document_id_v0(
            &contract_id,
            &owner_id,
            "contactRequest",
            entropy.as_slice(),
        );

        let result = ContactRequestResult {
            document: Document::V0(DocumentV0 {
                id,
                owner_id,
                ..Default::default()
            }),
            entropy,
        };

        // The entropy that send_contact_request will broadcast must regenerate the
        // exact id that was returned at creation time.
        let regenerated = Document::generate_document_id_v0(
            &contract_id,
            &result.document.owner_id(),
            "contactRequest",
            result.entropy.as_slice(),
        );
        assert_eq!(
            regenerated,
            result.document.id(),
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
        let secp = Secp256k1::new();
        let (secret_alice, public_alice) = secp.generate_keypair(&mut rand::thread_rng());
        let (secret_bob, public_bob) = secp.generate_keypair(&mut rand::thread_rng());

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
}
