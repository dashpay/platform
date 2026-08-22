//! Transport-free DashPay contact request document assembly.
//!
//! The Sdk-bound DashPay surface (recipient fetching, ECDH, encryption,
//! broadcasting) lives in `dash-sdk`; this module is the pure DIP-15
//! `contactRequest` document assembly it shares with embedders. All crypto
//! material arrives here as bytes — key derivation and encryption stay with
//! the caller.

use crate::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use std::collections::BTreeMap;

/// Already-derived crypto material and metadata for a DIP-15
/// `contactRequest` document.
///
/// Everything here is plain data: the ECDH/encryption that produced
/// `encrypted_public_key` and `encrypted_account_label`, and the randomness
/// that produced `entropy`, happen in the caller (`dash-sdk` or an
/// embedder).
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
        return Err(Error::InvalidInput(format!(
            "autoAcceptProof must be 38-102 bytes, got {}",
            proof.len()
        )));
    }
    Ok(())
}

/// Build the id and property map of a DIP-15 `contactRequest` document from
/// already-derived crypto material.
///
/// This is the pure document-assembly half of `dash-sdk`'s
/// `create_contact_request`: the document id derives from
/// `params.entropy`, and the property map carries exactly the fields the
/// DashPay contract defines (`toUserId`, `encryptedPublicKey`,
/// `senderKeyIndex`, `recipientKeyIndex`, `accountReference`, plus the
/// optional `encryptedAccountLabel` and `autoAcceptProof`).
///
/// Returns `(document_id, properties)`.
pub fn build_contact_request_document(
    contract: &DataContract,
    params: ContactRequestDocumentParams,
) -> Result<(Identifier, BTreeMap<String, Value>), Error> {
    if let Some(ref proof) = params.auto_accept_proof {
        validate_auto_accept_proof(proof)?;
    }

    // Validate encrypted public key size (must be exactly 96 bytes: 16-byte IV + 80-byte encrypted data)
    if params.encrypted_public_key.len() != 96 {
        return Err(Error::InvalidInput(format!(
            "Encrypted public key size mismatch: expected 96 bytes, got {}",
            params.encrypted_public_key.len()
        )));
    }

    // Validate encrypted label size (48-80 bytes: 16-byte IV + 32-64 byte encrypted data)
    if let Some(ref label) = params.encrypted_account_label {
        if label.len() < 48 || label.len() > 80 {
            return Err(Error::InvalidInput(format!(
                "Encrypted account label size out of range: expected 48-80 bytes, got {}",
                label.len()
            )));
        }
    }

    let contact_request_document_type =
        contract
            .document_type_for_name("contactRequest")
            .map_err(|_| {
                Error::InvalidInput("DashPay contactRequest document type not found".to_string())
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

    Ok((document_id, properties))
}
