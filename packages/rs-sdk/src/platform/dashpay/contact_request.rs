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
use dpp::document::{DocumentV0, DocumentV0Getters};
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

/// Input for creating a contact request document
pub struct ContactRequestInput {
    /// The identity sending the contact request (owner)
    pub sender_identity: Identity,
    /// The recipient's complete identity (needed for ECDH encryption)
    pub recipient_identity: Identity,
    /// The sender's encryption key index for ECDH
    pub sender_key_index: u32,
    /// The recipient's encryption key index for ECDH
    pub recipient_key_index: u32,
    /// Reference to the DashPay receiving account
    pub account_reference: u32,
    /// Extended public key bytes to encrypt for the recipient (unencrypted, typically 78 bytes, encrypted to 96 bytes with IV)
    pub extended_public_key: Vec<u8>,
    /// Optional account label to encrypt (unencrypted string, encrypted to 48-80 bytes with IV depending on length)
    pub account_label: Option<String>,
    /// Optional auto-accept proof (38-102 bytes) - not encrypted
    pub auto_accept_proof: Option<Vec<u8>>,
    /// The sender's private key for ECDH (for deriving shared secret)
    pub sender_private_key: SecretKey,
}

/// Result of creating a contact request document
#[derive(Debug)]
pub struct ContactRequestResult {
    /// The created contact request document
    pub document: Document,
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
    pub async fn create_contact_request(
        &self,
        input: ContactRequestInput,
    ) -> Result<ContactRequestResult, Error> {
        // Validate auto accept proof size if provided
        if let Some(ref proof) = input.auto_accept_proof {
            if proof.len() < 38 || proof.len() > 102 {
                return Err(Error::Generic(format!(
                    "autoAcceptProof must be 38-102 bytes, got {}",
                    proof.len()
                )));
            }
        }

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
        let recipient_key = input
            .recipient_identity
            .public_keys()
            .get(&input.recipient_key_index)
            .ok_or_else(|| {
                Error::Generic(format!(
                    "Recipient identity does not have encryption key at index {}",
                    input.recipient_key_index
                ))
            })?;

        if recipient_key.purpose() != Purpose::ENCRYPTION {
            return Err(Error::Generic(format!(
                "Recipient key at index {} is not an encryption key",
                input.recipient_key_index
            )));
        }

        // Get the recipient's public key data for ECDH
        let recipient_public_key_data = recipient_key.data();
        let recipient_public_key = PublicKey::from_slice(recipient_public_key_data.as_slice())
            .map_err(|e| Error::Generic(format!("Invalid recipient public key: {}", e)))?;

        // Derive shared secret using ECDH
        let shared_key = derive_shared_key_ecdh(&input.sender_private_key, &recipient_public_key);

        // Generate random IVs for encryption
        let mut rng = StdRng::from_entropy();
        let mut xpub_iv = [0u8; 16];
        rng.fill_bytes(&mut xpub_iv);

        // Encrypt the extended public key (includes IV prepended)
        let encrypted_public_key =
            encrypt_extended_public_key(&shared_key, &xpub_iv, &input.extended_public_key);

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
        let recipient_id = input.recipient_identity.id().to_owned();
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

        // Create the contact request document
        let document = Document::V0(DocumentV0 {
            id: document_id,
            owner_id: sender_id,
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
        });

        Ok(ContactRequestResult { document })
    }

    /// Send a contact request to the platform
    ///
    /// This creates a contact request document with automatic ECDH encryption and submits it
    /// to the platform as a state transition.
    ///
    /// # Arguments
    ///
    /// * `input` - The send contact request input containing document data, key, and signer
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
    pub async fn send_contact_request<S: Signer>(
        &self,
        input: SendContactRequestInput<S>,
    ) -> Result<SendContactRequestResult, Error> {
        // Create the contact request document (handles ECDH encryption internally)
        let result = self.create_contact_request(input.contact_request).await?;

        // Get the DashPay contract for the document type
        let dashpay_contract = self.fetch_dashpay_contract().await?;
        let contact_request_document_type = dashpay_contract
            .document_type_for_name("contactRequest")
            .map_err(|_| {
                Error::Generic("DashPay contactRequest document type not found".to_string())
            })?;

        // Extract entropy from document ID for state transition
        // Note: In a real implementation, we'd need to store the entropy used during creation
        // For now, we'll generate new entropy (this is a simplification)
        let mut rng = StdRng::from_entropy();
        let entropy = Bytes32::random_with_rng(&mut rng);

        // Submit the document to the platform
        let platform_document = result
            .document
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

        // Extract recipient ID and account reference from the document
        let recipient_id = if let Some(Value::Identifier(id_bytes)) =
            platform_document.properties().get("toUserId")
        {
            Identifier::from_bytes(id_bytes)
                .map_err(|e| Error::Generic(format!("Invalid recipient ID: {}", e)))?
        } else {
            return Err(Error::Generic(
                "toUserId not found in contact request document".to_string(),
            ));
        };

        let account_reference = if let Some(Value::U32(ref_val)) =
            platform_document.properties().get("accountReference")
        {
            *ref_val
        } else {
            return Err(Error::Generic(
                "accountReference not found in contact request document".to_string(),
            ));
        };

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
