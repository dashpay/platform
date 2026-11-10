//! The private document envelope (`platform-encryption`) and a private type's payload (dpp) fit
//! together: both count the envelope's bytes alike, and a type whose entries take its minimum
//! size carries its largest payload, sealed and opened, in exactly one entry.

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::{
    decode_private_document_payload, encode_private_document_payload,
    minimum_private_document_entry_size, PRIVATE_DOCUMENT_ENVELOPE_V1_BODY_HEADER_BYTES,
    PRIVATE_DOCUMENT_ENVELOPE_V1_OVERHEAD_BYTES, PRIVATE_DOCUMENT_ENVELOPE_V1_WRAPPER_BYTES,
};
use dpp::data_contract::DataContract;
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::secp256k1::{PublicKey, SecretKey};
use dpp::version::PlatformVersion;
use platform_encryption::{
    open_private_document_entry, private_document_outgoing_viewing_key,
    seal_private_document_entry, PrivateDocumentRecipient, PrivateDocumentSender,
    PrivateDocumentTarget, PRIVATE_DOCUMENT_ENVELOPE_BODY_HEADER_LEN,
    PRIVATE_DOCUMENT_ENVELOPE_OVERHEAD_LEN, PRIVATE_DOCUMENT_ENVELOPE_WRAPPER_LEN,
};
use std::collections::BTreeMap;

/// A contract with the private type `request`, shaped as DashPay's `privateContactRequest`, of
/// `entry_size`-byte entries. Unlike DashPay, it keeps sized integer types on, so
/// `accountReference` is a u32: its largest payload is 286 bytes, not DashPay's 290.
fn contract(entry_size: u32) -> DataContract {
    let id = Identifier::from([7; 32]).to_string(Encoding::Base58);
    let owner_id = Identifier::from([8; 32]).to_string(Encoding::Base58);
    DataContract::from_value(
        platform_value!({
            "$formatVersion": "1",
            "id": id,
            "ownerId": owner_id,
            "version": 1,
            "documentSchemas": {
                "request": {
                    "type": "object",
                    "private": { "entrySize": entry_size },
                    "documentsMutable": false,
                    "canBeDeleted": false,
                    "properties": {
                        "accountReference": {
                            "type": "integer", "minimum": 0, "maximum": 4294967295u64,
                            "position": 0
                        },
                        "extendedPublicKey": {
                            "type": "array", "byteArray": true,
                            "minItems": 69, "maxItems": 69, "position": 1
                        },
                        "accountLabel": { "type": "string", "maxBytes": 63, "position": 2 },
                        "autoAcceptProof": {
                            "type": "array", "byteArray": true,
                            "minItems": 38, "maxItems": 102, "position": 3
                        },
                        "shieldedAddress": {
                            "type": "array", "byteArray": true,
                            "minItems": 43, "maxItems": 43, "position": 4
                        }
                    },
                    "required": ["accountReference", "extendedPublicKey"],
                    "additionalProperties": false
                }
            }
        }),
        true,
        PlatformVersion::latest(),
    )
    .expect("the contract registers")
}

#[test]
fn should_count_the_envelope_bytes_alike_in_both_crates() {
    assert_eq!(
        PRIVATE_DOCUMENT_ENVELOPE_WRAPPER_LEN as u32,
        PRIVATE_DOCUMENT_ENVELOPE_V1_WRAPPER_BYTES
    );
    assert_eq!(
        PRIVATE_DOCUMENT_ENVELOPE_BODY_HEADER_LEN as u32,
        PRIVATE_DOCUMENT_ENVELOPE_V1_BODY_HEADER_BYTES
    );
    assert_eq!(
        PRIVATE_DOCUMENT_ENVELOPE_OVERHEAD_LEN as u32,
        PRIVATE_DOCUMENT_ENVELOPE_V1_OVERHEAD_BYTES
    );
}

/// The largest payload of a type whose entries take its minimum size is sealed into one entry
/// of exactly that size, opens for its recipient with its sender checked, and decodes back.
#[test]
fn should_carry_the_largest_payload_in_an_entry_of_the_minimum_size() {
    let platform_version = PlatformVersion::latest();
    let minimum = minimum_private_document_entry_size(
        contract(4096)
            .document_type_for_name("request")
            .expect("the type"),
    )
    .expect("bounded");
    assert_eq!(minimum, 286 + 299);
    let contract = contract(minimum);
    let request = contract
        .document_type_for_name("request")
        .expect("the type");
    let entry_size = request.private_document().expect("private").entry_size() as usize;

    let properties = BTreeMap::from([
        ("accountReference".to_string(), Value::U32(u32::MAX)),
        ("extendedPublicKey".to_string(), Value::Bytes(vec![1; 69])),
        ("accountLabel".to_string(), Value::Text("a".repeat(63))),
        ("autoAcceptProof".to_string(), Value::Bytes(vec![2; 102])),
        ("shieldedAddress".to_string(), Value::Bytes(vec![3; 43])),
    ]);
    let payload = encode_private_document_payload(request, properties.clone()).expect("encodes");
    assert_eq!(
        payload.len(),
        entry_size - PRIVATE_DOCUMENT_ENVELOPE_OVERHEAD_LEN
    );

    let sender_key = SecretKey::from_secret_bytes([1; 32]).expect("a key");
    let recipient_key = SecretKey::from_secret_bytes([2; 32]).expect("a key");
    let target = PrivateDocumentTarget {
        contract_id: contract.id().to_buffer(),
        document_type_name: "request",
    };
    let entry = seal_private_document_entry(
        &target,
        entry_size,
        &PrivateDocumentRecipient {
            identity_id: [0xB0; 32],
            public_key: &PublicKey::from_secret_key(&recipient_key),
        },
        &PrivateDocumentSender::DeniableMac {
            identity_id: [0xA0; 32],
            key_id: 4,
            secret_key: &sender_key,
        },
        &payload,
        &private_document_outgoing_viewing_key(&[0x0F; 32]),
        &[9; 32],
    )
    .expect("the largest payload fits");
    assert_eq!(entry.len(), entry_size);

    let opened = open_private_document_entry(&entry, &target, &[0xB0; 32], &[recipient_key])
        .expect("well formed")
        .expect("for the recipient");
    opened
        .verify_sender(
            &PublicKey::from_secret_key(&sender_key),
            Some(&recipient_key),
        )
        .expect("from the sender");
    let decoded = decode_private_document_payload(request, opened.payload(), platform_version)
        .expect("decodes");
    assert_eq!(
        decoded, properties,
        "every property reads back as it was sealed"
    );
}
