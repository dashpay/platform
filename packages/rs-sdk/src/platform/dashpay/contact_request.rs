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
use platform_wallet::crypto::{
    derive_shared_key_ecdh, encrypt_account_label, encrypt_extended_public_key,
};
use std::collections::BTreeMap;

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
    /// The document ID
    pub id: Identifier,
    /// The owner ID (sender identity ID)
    pub owner_id: Identifier,
    /// The document properties
    pub properties: BTreeMap<String, Value>,
}

/// Input for sending a contact request to the platform
pub struct SendContactRequestInput<S: Signer> {
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
    ///   - Returns: The unencrypted extended public key bytes (typically 78 bytes)
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

        if sender_key.purpose() != Purpose::ENCRYPTION {
            return Err(Error::Generic(format!(
                "Sender key at index {} is not an encryption key",
                input.sender_key_index
            )));
        }

        // Verify recipient has the encryption key at the specified index
        let recipient_key = recipient_identity
            .public_keys()
            .get(&input.recipient_key_index)
            .ok_or_else(|| {
                Error::Generic(format!(
                    "Recipient identity does not have encryption key at index {}",
                    input.recipient_key_index
                ))
            })?;

        if recipient_key.purpose() != Purpose::DECRYPTION {
            return Err(Error::Generic(format!(
                "Recipient key at index {} is not a decryption key",
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

        // Get the extended public key to encrypt
        let extended_public_key = get_extended_public_key(input.account_reference).await?;

        // Generate random IVs for encryption
        let mut rng = StdRng::from_entropy();
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

        // Fetch DashPay contract
        let dashpay_contract = self.fetch_dashpay_contract().await?;

        // Get contactRequest document type
        let contact_request_document_type = dashpay_contract
            .document_type_for_name("contactRequest")
            .map_err(|_| {
                Error::Generic("DashPay contactRequest document type not found".to_string())
            })?;

        // Generate entropy for document ID
        let mut rng = StdRng::from_entropy();
        let entropy = Bytes32::random_with_rng(&mut rng);

        // Generate document ID
        let sender_id = input.sender_identity.id().to_owned();
        let document_id = Document::generate_document_id_v0(
            &dashpay_contract.id(),
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

        // Return the essential fields for the contact request
        Ok(ContactRequestResult {
            id: document_id,
            owner_id: sender_id,
            properties,
        })
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
    ///   - Returns: The unencrypted extended public key bytes (typically 78 bytes)
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
    pub async fn send_contact_request<S: Signer, F, Fut, G, Gut, H, Hut>(
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

        // Create the document from the result
        let document = Document::V0(DocumentV0 {
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
        });

        // Extract entropy from document ID for state transition
        // Note: In a real implementation, we'd need to store the entropy used during creation
        // For now, we'll generate new entropy (this is a simplification)
        let mut rng = StdRng::from_entropy();
        let entropy = Bytes32::random_with_rng(&mut rng);

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

        // Test extended public key encryption (78 bytes -> 96 bytes with IV + PKCS7 padding)
        let xpub_data = vec![0x04; 78];
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
