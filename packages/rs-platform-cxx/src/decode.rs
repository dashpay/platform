// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! DPP decoders for C++ identity and document value types, built on the real
//! rs-dpp deserializers.

use std::collections::BTreeMap;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::platform_value::Value;
use dpp::serialization::PlatformDeserializable;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use platform_version::version::PlatformVersion;

use crate::types::{ContactRequest, DpnsName, IdentityInfo, KeyInfo, Profile};

/// Flattens a dpp key into the FFI form.
pub(crate) fn key_info(key: &IdentityPublicKey) -> KeyInfo {
    KeyInfo {
        id: key.id(),
        purpose: key.purpose() as u8,
        security_level: key.security_level() as u8,
        key_type: key.key_type() as u8,
        read_only: key.read_only(),
        data: key.data().to_vec(),
        disabled_at: key.disabled_at(),
    }
}

/// Flattens a dpp identity into the FFI form.
pub(crate) fn identity_info(identity: &Identity) -> IdentityInfo {
    IdentityInfo {
        id: identity.id().to_buffer(),
        balance: identity.balance(),
        revision: identity.revision(),
        keys: identity.public_keys().values().map(key_info).collect(),
    }
}

pub fn decode_identity(bytes: &[u8]) -> Result<IdentityInfo, String> {
    let identity =
        Identity::deserialize_from_bytes(bytes).map_err(|e| format!("bad identity: {e}"))?;
    Ok(identity_info(&identity))
}

pub fn decode_identity_public_key(bytes: &[u8]) -> Result<KeyInfo, String> {
    let key = IdentityPublicKey::deserialize_from_bytes(bytes)
        .map_err(|e| format!("bad identity public key: {e}"))?;
    Ok(key_info(&key))
}

fn decode_document(
    bytes: &[u8],
    contract: SystemDataContract,
    document_type_name: &str,
) -> Result<Document, String> {
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    let version = PlatformVersion::latest();
    let contract = load_system_data_contract(contract, version)
        .map_err(|e| format!("unable to load system data contract: {e}"))?;
    let document_type = contract
        .document_type_for_name(document_type_name)
        .map_err(|e| format!("unknown document type {document_type_name}: {e}"))?;
    Document::from_bytes(bytes, document_type, version)
        .map_err(|e| format!("bad {document_type_name} document: {e}"))
}

fn get_str(properties: &BTreeMap<String, Value>, name: &str) -> String {
    match properties.get(name) {
        Some(Value::Text(text)) => text.clone(),
        _ => String::new(),
    }
}

fn get_bytes(properties: &BTreeMap<String, Value>, name: &str) -> Vec<u8> {
    match properties.get(name) {
        Some(Value::Bytes(bytes)) => bytes.clone(),
        Some(Value::Bytes20(bytes)) => bytes.to_vec(),
        Some(Value::Bytes32(bytes)) => bytes.to_vec(),
        Some(Value::Bytes36(bytes)) => bytes.to_vec(),
        Some(Value::Identifier(id)) => id.to_vec(),
        _ => Vec::new(),
    }
}

fn get_u32(properties: &BTreeMap<String, Value>, name: &str) -> Result<u32, String> {
    let Some(value) = properties.get(name) else {
        return Ok(0);
    };
    value
        .clone()
        .into_integer::<u32>()
        .map_err(|e| format!("property {name} is not a u32: {e}"))
}

fn get_identifier(value: Option<&Value>) -> Option<[u8; 32]> {
    match value {
        Some(Value::Identifier(id)) => Some(*id),
        Some(Value::Bytes32(bytes)) => Some(*bytes),
        Some(Value::Bytes(bytes)) => bytes.as_slice().try_into().ok(),
        _ => None,
    }
}

pub fn decode_dpns_domain(doc_bytes: &[u8]) -> Result<DpnsName, String> {
    let document = decode_document(doc_bytes, SystemDataContract::DPNS, "domain")?;
    let properties = document.properties();
    let identity = properties
        .get("records")
        .and_then(|records| match records {
            Value::Map(map) => map
                .iter()
                .find(|(key, _)| matches!(key, Value::Text(text) if text == "identity"))
                .map(|(_, value)| value),
            _ => None,
        })
        .and_then(|value| get_identifier(Some(value)))
        .ok_or("DPNS domain document has no records.identity")?;
    Ok(DpnsName {
        label: get_str(properties, "label"),
        normalized_label: get_str(properties, "normalizedLabel"),
        parent_domain: get_str(properties, "normalizedParentDomainName"),
        identity,
        document_id: document.id().to_buffer(),
        owner_id: document.owner_id().to_buffer(),
    })
}

pub fn decode_dashpay_profile(doc_bytes: &[u8]) -> Result<Profile, String> {
    let document = decode_document(doc_bytes, SystemDataContract::Dashpay, "profile")?;
    let properties = document.properties();
    Ok(Profile {
        document_id: document.id().to_buffer(),
        owner_id: document.owner_id().to_buffer(),
        display_name: get_str(properties, "displayName"),
        public_message: get_str(properties, "publicMessage"),
        avatar_url: get_str(properties, "avatarUrl"),
        avatar_hash: get_bytes(properties, "avatarHash"),
        avatar_fingerprint: get_bytes(properties, "avatarFingerprint"),
        created_at: document.created_at().unwrap_or(0),
        updated_at: document.updated_at().unwrap_or(0),
        revision: document.revision().unwrap_or(0),
    })
}

pub fn decode_contact_request(doc_bytes: &[u8]) -> Result<ContactRequest, String> {
    let document = decode_document(doc_bytes, SystemDataContract::Dashpay, "contactRequest")?;
    let properties = document.properties();
    let to_user_id = get_identifier(properties.get("toUserId"))
        .ok_or("contact request document has no toUserId")?;
    Ok(ContactRequest {
        document_id: document.id().to_buffer(),
        owner_id: document.owner_id().to_buffer(),
        to_user_id,
        encrypted_public_key: get_bytes(properties, "encryptedPublicKey"),
        sender_key_index: get_u32(properties, "senderKeyIndex")?,
        recipient_key_index: get_u32(properties, "recipientKeyIndex")?,
        account_reference: get_u32(properties, "accountReference")?,
        encrypted_account_label: get_bytes(properties, "encryptedAccountLabel"),
        core_height_created_at: document.created_at_core_block_height().unwrap_or(0),
        created_at: document.created_at().unwrap_or(0),
    })
}
