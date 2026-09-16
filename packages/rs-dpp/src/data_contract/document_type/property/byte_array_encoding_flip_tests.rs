//! Security regression tests for the byteArray on-disk encoding-flip chain-halt
//! vulnerability (Dash Platform v4.0.0-rc.1).
//!
//! BACKGROUND
//! ----------
//! A `byteArray` document property is serialized differently depending on whether
//! its `minItems == maxItems`:
//!
//!   * `min == max` -> stored as RAW bytes, with NO length prefix. The decoder
//!     (`DocumentPropertyType::read_optionally_from`) reads exactly `min` bytes.
//!   * `min != max` (or `maxItems` removed) -> stored as a VARINT length prefix
//!     followed by the bytes. The decoder calls `read_varint_value`, which reads
//!     a varint length then `read_exact`s that many bytes.
//!
//! A `DataContractUpdate` that *widens* `maxItems` (e.g. 32 -> 64) is accepted by
//! schema-compatibility validation, but it FLIPS the on-disk encoding of the
//! property. Documents that were already stored under the old fixed (raw)
//! encoding then become un-decodable under the new variable (varint-prefixed)
//! type: the first stored data byte is reinterpreted as a varint length. If that
//! byte is a varint continuation byte (>= 0x80, e.g. 0xFF) the decoded length is
//! enormous, `read_exact` overruns the buffer, and decoding returns
//! `DataContractError::CorruptedSerialization` => `Err`.
//!
//! These tests PROVE the bug exists today. They are written to PASS by asserting
//! that the failure (`Err`) happens. After the bug is fixed, the asserted `Err`
//! should become `Ok`, so the assertions can be flipped to lock in the fix as a
//! regression test.

use super::*;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::DataContract;
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use crate::document::{Document, DocumentV0};
use platform_value::{platform_value, Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use std::io::BufReader;

const BYTE_ARRAY_FIELD: &str = "data";

/// Builds a single-document-type [`DataContract`] whose `data` property is a
/// `byteArray` with the given `minItems`/`maxItems`. When `max_items` is `None`
/// the `maxItems` keyword is omitted entirely (the "removal" variant of the
/// attack).
fn build_contract_with_byte_array(
    min_items: u32,
    max_items: Option<u32>,
    platform_version: &PlatformVersion,
) -> DataContract {
    let config = DataContractConfig::default_for_version(platform_version).expect("default config");

    let mut byte_array_schema = platform_value!({
        "type": "array",
        "byteArray": true,
        "minItems": min_items,
        "position": 0_u32,
    });

    if let Some(max) = max_items {
        if let Value::Map(map) = &mut byte_array_schema {
            map.push((Value::Text("maxItems".to_string()), Value::U32(max)));
        }
    }

    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            BYTE_ARRAY_FIELD: byte_array_schema,
        },
        "additionalProperties": false,
    });

    let serialization_format = DataContractInSerializationFormatV0 {
        id: Identifier::new([7; 32]),
        config,
        version: 1,
        owner_id: Identifier::new([8; 32]),
        schema_defs: None,
        document_schemas: BTreeMap::from([("doc".to_string(), document_schema)]),
    };

    DataContractV0::try_from_platform_versioned(
        serialization_format.into(),
        true,
        &mut vec![],
        platform_version,
    )
    .expect("should build contract")
    .into()
}

/// Builds a document whose byteArray `data` field is a 32-byte value whose FIRST
/// byte is `0xFF` (a varint continuation byte).
fn build_document_with_ff_prefixed_bytes(_contract: &DataContract) -> Document {
    let mut bytes = [0u8; 32];
    bytes[0] = 0xFF;

    let mut properties = BTreeMap::new();
    // A 32-byte byteArray is canonically represented (and re-loaded) as
    // `Value::Bytes32` by the decoder, so we store it that way to keep the
    // happy-path round-trip comparison exact.
    properties.insert(BYTE_ARRAY_FIELD.to_string(), Value::Bytes32(bytes));

    DocumentV0 {
        contract_version: None,
        id: Identifier::new([1; 32]),
        owner_id: Identifier::new([2; 32]),
        properties,
        revision: Some(1),
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
    }
    .into()
}

// ----------------------------------------------------------------------------
// TEST 1: byteArray encoding flip makes old bytes undecodable
// ----------------------------------------------------------------------------

/// PROPERTY-LEVEL PROOF.
///
/// Encode a 32-byte value (first byte 0xFF) under a fixed `ByteArray{32,32}`
/// type, then attempt to decode it under a widened `ByteArray{32,64}` type.
///
/// * Under the FIXED type the bytes are raw (no length prefix).
/// * Under the WIDENED type the decoder treats the first byte (0xFF) as a varint
///   length, reads a huge length, and `read_exact` overruns -> `Err`.
#[test]
fn byte_array_encoding_flip_property_level_makes_old_bytes_undecodable() {
    let fixed = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
        min_size: Some(32),
        max_size: Some(32),
    });
    let widened = DocumentPropertyType::ByteArray(ByteArrayPropertySizes {
        min_size: Some(32),
        max_size: Some(64),
    });

    let mut value_bytes = [0u8; 32];
    value_bytes[0] = 0xFF;
    let value = Value::Bytes(value_bytes.to_vec());

    // Encoded under the fixed (raw, no length prefix) type.
    let encoded = fixed
        .encode_value_ref_with_size(&value, true)
        .expect("encode under fixed type should succeed");

    // The fixed encoding is exactly the raw 32 bytes (no prefix).
    assert_eq!(
        encoded.len(),
        32,
        "fixed byteArray must be stored raw with no length prefix"
    );
    assert_eq!(encoded[0], 0xFF, "first stored byte is the 0xFF data byte");

    // HAPPY PATH: decoding under the SAME fixed type round-trips perfectly.
    {
        let mut buf = BufReader::new(encoded.as_slice());
        let (decoded, _finished) = fixed
            .read_optionally_from(&mut buf, true)
            .expect("decode under fixed type should succeed");
        let decoded = decoded.expect("value present");
        let decoded_bytes = decoded.into_binary_bytes().expect("decoded value is bytes");
        assert_eq!(
            decoded_bytes, value_bytes,
            "round-trip under the unchanged fixed type must preserve the bytes"
        );
    }

    // BUG: decoding the SAME stored bytes under the WIDENED type fails today,
    // because the 0xFF first byte is misread as a varint length.
    let mut buf = BufReader::new(encoded.as_slice());
    let result = widened.read_optionally_from(&mut buf, true);

    assert!(
        result.is_err(),
        "VULNERABILITY: widening maxItems flips the encoding so old raw bytes \
         become undecodable; expected Err today but got Ok: {:?}. \
         If this assertion now fails because the result is Ok, the bug has been \
         fixed -- flip this assertion to lock in the fix.",
        result
    );

    let err = result.unwrap_err();
    // It is a serialization/corruption-class error coming out of read_varint_value.
    assert!(
        matches!(
            err,
            DataContractError::CorruptedSerialization(_)
                | DataContractError::DecodingContractError(_)
        ),
        "expected a corrupted/decoding serialization error, got: {:?}",
        err
    );
}

/// DOCUMENT-LEVEL PROOF (the form closest to the on-chain path).
///
/// Serialize a full [`Document`] against a contract whose `data` byteArray is
/// fixed `{minItems:32, maxItems:32}`, then call `Document::from_bytes` using a
/// contract whose `data` byteArray was widened to `{minItems:32, maxItems:64}`.
/// This mirrors exactly what happens on chain after a malicious
/// `DataContractUpdate`: the stored (old-encoding) document bytes are decoded
/// against the CURRENT (widened) document type.
#[test]
fn byte_array_encoding_flip_document_level_makes_old_bytes_undecodable() {
    let platform_version = PlatformVersion::latest();

    let fixed_contract = build_contract_with_byte_array(32, Some(32), platform_version);
    let widened_contract = build_contract_with_byte_array(32, Some(64), platform_version);

    let fixed_type = fixed_contract
        .document_type_for_name("doc")
        .expect("fixed doc type");
    let widened_type = widened_contract
        .document_type_for_name("doc")
        .expect("widened doc type");

    let document = build_document_with_ff_prefixed_bytes(&fixed_contract);

    // Stored on chain under the OLD (fixed 32/32) encoding.
    // Fully-qualified call to disambiguate from serde's `Serialize::serialize`.
    let serialized = DocumentPlatformConversionMethodsV0::serialize(
        &document,
        fixed_type,
        &fixed_contract,
        platform_version,
    )
    .expect("serialize under fixed type should succeed");

    // HAPPY PATH: round-trip under the unchanged fixed type works.
    let round_tripped = Document::from_bytes(&serialized, fixed_type, platform_version)
        .expect("round-trip under fixed type should succeed");
    assert_eq!(
        round_tripped, document,
        "round-trip under the unchanged fixed type must preserve the document"
    );

    // BUG: decoding the OLD stored bytes against the WIDENED current type fails.
    let result = Document::from_bytes(&serialized, widened_type, platform_version);

    assert!(
        result.is_err(),
        "VULNERABILITY: a committed DataContractUpdate that widens maxItems from \
         32 to 64 makes previously-stored documents undecodable. Expected Err \
         today but got Ok. If this is now Ok, the bug is fixed -- flip the \
         assertion."
    );
}

/// Control: widening from `{32,32}` to `{32,64}` is the cause. A document whose
/// first stored byte is a NON-continuation byte (< 0x80) may decode to a
/// *different* (wrong) value rather than erroring, which is itself silent data
/// corruption -- but the 0xFF case above is the one that produces the hard
/// `Err` that halts the chain. Here we additionally show that removing
/// `maxItems` entirely triggers the same hard failure.
#[test]
fn byte_array_encoding_flip_via_maxitems_removal_makes_old_bytes_undecodable() {
    let platform_version = PlatformVersion::latest();

    let fixed_contract = build_contract_with_byte_array(32, Some(32), platform_version);
    // maxItems removed entirely -> variable-length (varint-prefixed) encoding.
    let removed_contract = build_contract_with_byte_array(32, None, platform_version);

    let fixed_type = fixed_contract
        .document_type_for_name("doc")
        .expect("fixed doc type");
    let removed_type = removed_contract
        .document_type_for_name("doc")
        .expect("maxItems-removed doc type");

    let document = build_document_with_ff_prefixed_bytes(&fixed_contract);

    let serialized = DocumentPlatformConversionMethodsV0::serialize(
        &document,
        fixed_type,
        &fixed_contract,
        platform_version,
    )
    .expect("serialize under fixed type should succeed");

    let result = Document::from_bytes(&serialized, removed_type, platform_version);

    assert!(
        result.is_err(),
        "VULNERABILITY: removing maxItems also flips the encoding and makes old \
         bytes undecodable. Expected Err today but got Ok."
    );
}
