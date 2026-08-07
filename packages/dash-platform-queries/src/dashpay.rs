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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;

    fn dashpay_contract() -> DataContract {
        load_system_data_contract(SystemDataContract::Dashpay, PlatformVersion::latest())
            .expect("should load DashPay system contract")
    }

    fn valid_params() -> ContactRequestDocumentParams {
        ContactRequestDocumentParams {
            sender_id: Identifier::from([2u8; 32]),
            recipient_id: Identifier::from([3u8; 32]),
            sender_key_index: 1,
            recipient_key_index: 2,
            account_reference: 7,
            encrypted_public_key: vec![0xAA; 96],
            encrypted_account_label: None,
            auto_accept_proof: None,
            entropy: [5u8; 32],
        }
    }

    #[test]
    fn entropy_derives_built_document_id() {
        // Mirror of rs-sdk's contact_request_result_entropy_derives_returned_id:
        // the id the builder returns must be exactly what consensus recomputes
        // from the entropy attached to the create transition.
        let contract = dashpay_contract();
        let params = valid_params();
        let entropy = params.entropy;
        let sender_id = params.sender_id;

        let (id, _) =
            build_contact_request_document(&contract, params).expect("valid params must build");

        assert_eq!(
            id,
            Document::generate_document_id_v0(
                &contract.id(),
                &sender_id,
                "contactRequest",
                entropy.as_slice()
            ),
            "built document id must derive from the supplied entropy"
        );
    }

    #[test]
    fn builds_expected_property_map() {
        let contract = dashpay_contract();
        let mut params = valid_params();
        params.encrypted_account_label = Some(vec![0xBB; 48]);
        params.auto_accept_proof = Some(vec![0xCC; 38]);

        let (_, properties) =
            build_contact_request_document(&contract, params).expect("valid params must build");

        assert_eq!(
            properties,
            BTreeMap::from([
                (
                    "toUserId".to_string(),
                    Value::Identifier(Identifier::from([3u8; 32]).to_buffer())
                ),
                (
                    "encryptedPublicKey".to_string(),
                    Value::Bytes(vec![0xAA; 96])
                ),
                ("senderKeyIndex".to_string(), Value::U32(1)),
                ("recipientKeyIndex".to_string(), Value::U32(2)),
                ("accountReference".to_string(), Value::U32(7)),
                (
                    "encryptedAccountLabel".to_string(),
                    Value::Bytes(vec![0xBB; 48])
                ),
                ("autoAcceptProof".to_string(), Value::Bytes(vec![0xCC; 38])),
            ])
        );
    }

    #[test]
    fn optional_fields_are_omitted_when_absent() {
        let contract = dashpay_contract();
        let (_, properties) = build_contact_request_document(&contract, valid_params())
            .expect("valid params must build");

        assert_eq!(properties.len(), 5);
        assert!(!properties.contains_key("encryptedAccountLabel"));
        assert!(!properties.contains_key("autoAcceptProof"));
    }

    #[test]
    fn rejects_wrong_encrypted_public_key_size() {
        let contract = dashpay_contract();
        for bad_len in [0, 95, 97] {
            let mut params = valid_params();
            params.encrypted_public_key = vec![0xAA; bad_len];
            assert!(
                matches!(
                    build_contact_request_document(&contract, params),
                    Err(Error::InvalidInput(_))
                ),
                "encrypted public key of {bad_len} bytes must be rejected"
            );
        }
    }

    #[test]
    fn rejects_out_of_range_auto_accept_proof() {
        let contract = dashpay_contract();
        for bad_len in [0, 37, 103] {
            let mut params = valid_params();
            params.auto_accept_proof = Some(vec![0xCC; bad_len]);
            assert!(
                matches!(
                    build_contact_request_document(&contract, params),
                    Err(Error::InvalidInput(_))
                ),
                "auto accept proof of {bad_len} bytes must be rejected"
            );
        }
        for good_len in [38, 70, 102] {
            let mut params = valid_params();
            params.auto_accept_proof = Some(vec![0xCC; good_len]);
            assert!(
                build_contact_request_document(&contract, params).is_ok(),
                "auto accept proof of {good_len} bytes must be accepted"
            );
        }
    }

    #[test]
    fn rejects_out_of_range_encrypted_account_label() {
        let contract = dashpay_contract();
        for bad_len in [0, 47, 81] {
            let mut params = valid_params();
            params.encrypted_account_label = Some(vec![0xBB; bad_len]);
            assert!(
                matches!(
                    build_contact_request_document(&contract, params),
                    Err(Error::InvalidInput(_))
                ),
                "encrypted account label of {bad_len} bytes must be rejected"
            );
        }
    }
}
