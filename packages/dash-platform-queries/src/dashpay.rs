//! Transport-free DashPay document assembly.
//!
//! The Sdk-bound DashPay surface (ECDH, encryption, fetching the recipient,
//! broadcasting) lives in `dash-sdk`; this is the pure DIP-15 document
//! assembly it shares with embedders that hold the encrypted material
//! themselves. Field size bounds come from the DashPay contract schema, so
//! the builder cannot drift from what the contract accepts.

use crate::dpns_usernames::new_document;
use crate::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::errors::DataContractError;
use dpp::document::Document;
use dpp::platform_value::Value;
use dpp::prelude::{DataContract, Identifier};
use dpp::system_data_contracts::dashpay_contract::v1::document_types::contact_request;
use std::collections::BTreeMap;

/// Inputs of a DIP-15 `contactRequest` document. The encrypted fields are
/// supplied already encrypted (ECDH, AES-CBC with a fresh IV prepended);
/// see `dash-sdk`'s `create_contact_request` for the encryption itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactRequestDocumentParams {
    /// Identity sending the request; becomes the document owner.
    pub sender_id: Identifier,
    /// Identity receiving the request (`toUserId`).
    pub recipient_id: Identifier,
    /// Index of the sender's encryption key used for ECDH.
    pub sender_key_index: u32,
    /// Index of the recipient's key used for ECDH.
    pub recipient_key_index: u32,
    /// DashPay receiving-account reference.
    pub account_reference: u32,
    /// Encrypted DIP-15 compact extended public key (IV ‖ ciphertext).
    pub encrypted_public_key: Vec<u8>,
    /// Encrypted account label (IV ‖ ciphertext), if any.
    pub encrypted_account_label: Option<Vec<u8>>,
    /// Unencrypted auto-accept proof, if any.
    pub auto_accept_proof: Option<Vec<u8>>,
    /// Entropy the document id derives from; reuse it on the create
    /// transition.
    pub entropy: [u8; 32],
}

/// Assemble a `contactRequest` document, checking every byte-array field
/// against the size bounds the contract schema declares for it.
pub fn build_contact_request_document(
    contract: &DataContract,
    params: ContactRequestDocumentParams,
) -> Result<Document, Error> {
    let document_type = contract.document_type_for_name(contact_request::NAME)?;

    check_byte_field(
        document_type,
        "encryptedPublicKey",
        &params.encrypted_public_key,
    )?;
    if let Some(label) = &params.encrypted_account_label {
        check_byte_field(document_type, "encryptedAccountLabel", label)?;
    }
    if let Some(proof) = &params.auto_accept_proof {
        check_byte_field(document_type, "autoAcceptProof", proof)?;
    }

    let mut properties = BTreeMap::from([
        (
            contact_request::properties::TO_USER_ID.to_string(),
            Value::Identifier(params.recipient_id.to_buffer()),
        ),
        (
            "encryptedPublicKey".to_string(),
            Value::Bytes(params.encrypted_public_key),
        ),
        (
            "senderKeyIndex".to_string(),
            Value::U32(params.sender_key_index),
        ),
        (
            "recipientKeyIndex".to_string(),
            Value::U32(params.recipient_key_index),
        ),
        (
            "accountReference".to_string(),
            Value::U32(params.account_reference),
        ),
    ]);
    if let Some(label) = params.encrypted_account_label {
        properties.insert("encryptedAccountLabel".to_string(), Value::Bytes(label));
    }
    if let Some(proof) = params.auto_accept_proof {
        properties.insert("autoAcceptProof".to_string(), Value::Bytes(proof));
    }

    Ok(new_document(
        contract,
        document_type.name(),
        params.sender_id,
        params.entropy,
        properties,
    ))
}

/// Check `bytes` against the `minItems`/`maxItems` the contract declares for
/// the byte-array property `field`.
fn check_byte_field(
    document_type: DocumentTypeRef,
    field: &str,
    bytes: &[u8],
) -> Result<(), Error> {
    let property = document_type.properties().get(field).ok_or_else(|| {
        DataContractError::DocumentTypeFieldNotFound(format!(
            "{} has no property {field}",
            document_type.name()
        ))
    })?;
    // Only `Array`/`VariableTypeArray` report no size, and dpp refuses those
    // at contract creation ("only byte arrays are supported now"), so every
    // property reachable here is sized and the defaults never apply.
    let min = property.property_type.min_size().unwrap_or(0) as usize;
    let max = property.property_type.max_size().unwrap_or(u16::MAX) as usize;
    if bytes.len() < min || bytes.len() > max {
        return Err(Error::Config(format!(
            "{field} must be {min}-{max} bytes, got {}",
            bytes.len()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::document::DocumentV0Getters;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;

    fn params(encrypted_public_key: Vec<u8>) -> ContactRequestDocumentParams {
        ContactRequestDocumentParams {
            sender_id: Identifier::from([1u8; 32]),
            recipient_id: Identifier::from([2u8; 32]),
            sender_key_index: 1,
            recipient_key_index: 2,
            account_reference: 3,
            encrypted_public_key,
            encrypted_account_label: None,
            auto_accept_proof: None,
            entropy: [7u8; 32],
        }
    }

    fn contract() -> DataContract {
        load_system_data_contract(SystemDataContract::Dashpay, PlatformVersion::latest())
            .expect("dashpay contract")
    }

    #[test]
    fn should_derive_the_document_id_from_the_entropy() {
        let contract = contract();
        let document = build_contact_request_document(&contract, params(vec![0u8; 96]))
            .expect("valid contact request");
        assert_eq!(
            document.id(),
            Document::generate_document_id_v0(
                &contract.id(),
                &Identifier::from([1u8; 32]),
                contact_request::NAME,
                &[7u8; 32]
            )
        );
        assert_eq!(document.owner_id(), Identifier::from([1u8; 32]));
        assert_eq!(
            document.get("toUserId"),
            Some(&Value::Identifier([2u8; 32]))
        );
    }

    #[test]
    fn should_enforce_the_contract_byte_bounds() {
        let contract = contract();
        build_contact_request_document(&contract, params(vec![0u8; 95]))
            .expect_err("encryptedPublicKey below the schema's 96 bytes");
        let mut with_label = params(vec![0u8; 96]);
        with_label.encrypted_account_label = Some(vec![0u8; 81]);
        build_contact_request_document(&contract, with_label)
            .expect_err("encryptedAccountLabel above the schema's 80 bytes");
        let mut with_proof = params(vec![0u8; 96]);
        with_proof.auto_accept_proof = Some(vec![0u8; 37]);
        build_contact_request_document(&contract, with_proof)
            .expect_err("autoAcceptProof below the schema's 38 bytes");
    }

    /// The optional fields are the ones a wire-format slip would silently
    /// drop, so pin every property the document carries when both are set.
    #[test]
    fn should_carry_every_property_when_the_optional_fields_are_present() {
        let contract = contract();
        let mut with_optionals = params(vec![1u8; 96]);
        with_optionals.encrypted_account_label = Some(vec![2u8; 64]);
        with_optionals.auto_accept_proof = Some(vec![3u8; 40]);

        let document = build_contact_request_document(&contract, with_optionals)
            .expect("valid contact request");

        assert_eq!(
            document.get("toUserId"),
            Some(&Value::Identifier([2u8; 32]))
        );
        assert_eq!(
            document.get("encryptedPublicKey"),
            Some(&Value::Bytes(vec![1u8; 96]))
        );
        assert_eq!(document.get("senderKeyIndex"), Some(&Value::U32(1)));
        assert_eq!(document.get("recipientKeyIndex"), Some(&Value::U32(2)));
        assert_eq!(document.get("accountReference"), Some(&Value::U32(3)));
        assert_eq!(
            document.get("encryptedAccountLabel"),
            Some(&Value::Bytes(vec![2u8; 64]))
        );
        assert_eq!(
            document.get("autoAcceptProof"),
            Some(&Value::Bytes(vec![3u8; 40]))
        );
    }

    /// A contract that declares no `contactRequest` surfaces dpp's
    /// `DataContractError` through our `Error`, rather than panicking.
    #[test]
    fn should_refuse_a_contract_without_a_contact_request_type() {
        let dpns = load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("dpns contract");
        let error = build_contact_request_document(&dpns, params(vec![0u8; 96]))
            .expect_err("dpns declares no contactRequest document type");
        assert!(
            matches!(error, Error::Protocol(_)),
            "unexpected error: {error:?}"
        );
    }

    #[test]
    fn should_refuse_a_field_the_document_type_does_not_declare() {
        let contract = contract();
        let document_type = contract
            .document_type_for_name(contact_request::NAME)
            .expect("contactRequest document type");
        let error = check_byte_field(document_type, "notAProperty", &[])
            .expect_err("contactRequest declares no such property");
        assert!(
            matches!(error, Error::Protocol(_)),
            "unexpected error: {error:?}"
        );
    }
}
