//! The **entry payload** of an indexOnly document type: the value slot.
//!
//! A document type may list top-level properties under `entryPayload`.
//! They sit in no index; instead every entry's item carries them after the
//! 32-byte row commitment, so an entry reads `Item(commitment ‖ payload)`
//! (or `ItemWithSumItem(commitment ‖ payload, amount)` under a summable
//! index). The commitment still hashes them, so the delete probes and the
//! executed-transition verifier keep comparing the first 32 bytes only,
//! and the payload is recovered by decoding the proved element.
//!
//! Layout: for every payload property in property-name order (the order
//! the parsed `BTreeSet` iterates), a big-endian `u16` length followed by
//! the value's bytes: raw bytes for byte arrays, UTF-8 for strings, and
//! tree-key encoding for other scalars. Length framing preserves empty
//! values without the tree-key encoding's empty/null sentinels. The parser
//! bounds every payload property and caps their sum, which is what lets
//! the length frame be two bytes and fee estimation size the entry value
//! by the bounds.

use crate::drive::document::INDEX_ONLY_ITEM_ESTIMATED_VALUE_SIZE;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{DocumentPropertyType, DocumentTypeRef};
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

/// The bytes of length frame each payload property carries.
pub const INDEX_ONLY_ENTRY_PAYLOAD_LENGTH_FRAME: u32 = 2;

/// The most bytes `document_type`'s entry payload can encode to: the sum
/// of every payload property's declared bound plus its length frame. Zero
/// on a type without an entry payload. The parser guarantees every payload
/// property is bounded, so an unbounded one here is a corrupted type.
pub fn index_only_entry_payload_max_size(
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<u32, Error> {
    let mut total: u32 = 0;
    for property_name in document_type.entry_payload() {
        let property = document_type
            .properties()
            .get(property_name)
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "an entryPayload property must be a top-level property of its document type: \
                 the contract parser enforces it",
            )))?;
        let max_width = property
            .property_type
            .max_byte_size(platform_version)
            .map_err(|e| Error::Protocol(Box::new(e)))?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "an entryPayload property must be bounded: the contract parser enforces it",
            )))?;
        total = total
            .saturating_add(u32::from(max_width))
            .saturating_add(INDEX_ONLY_ENTRY_PAYLOAD_LENGTH_FRAME);
    }
    Ok(total)
}

/// The value size fee estimation claims for one of `document_type`'s entry
/// items: the padded commitment estimate plus the payload bound. Every
/// estimating site (the entry-insert terminal, the preallocated-tree
/// estimate, the delete walkers and the delete-side probe estimate) sizes
/// the item through this one function so they cannot drift.
pub fn index_only_item_estimated_value_size(
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<u32, Error> {
    Ok(
        INDEX_ONLY_ITEM_ESTIMATED_VALUE_SIZE.saturating_add(index_only_entry_payload_max_size(
            document_type,
            platform_version,
        )?),
    )
}

/// The bytes one entry payload value contributes: raw bytes for a byte
/// array, UTF-8 for a string, and the tree-key encoding's fixed
/// order-preserving widths for the numeric kinds. Unlike a key, a payload
/// value is not capped at 255 bytes — the parser bounds it by the field
/// value limit instead. Also what the row commitment hashes for a payload
/// property, so the commitment and the stored value agree byte for byte.
pub fn encode_index_only_entry_payload_value(
    property_type: &DocumentPropertyType,
    value: &Value,
) -> Result<Vec<u8>, Error> {
    // Payload lengths already distinguish empty values; the tree-key
    // string sentinel would conflate "" and "\0" in both the stored
    // payload and its row commitment.
    if let (DocumentPropertyType::String(_), Value::Text(text)) = (property_type, value) {
        return Ok(text.as_bytes().to_vec());
    }
    property_type
        .encode_value_for_tree_keys(value)
        .map_err(|e| Error::Protocol(Box::new(e)))
}

/// Encode `document`'s entry payload — every `entryPayload` property in
/// name order, length-framed in its payload encoding. Empty on a type
/// without an entry payload. A missing payload property is a corrupted
/// document: the parser requires every payload property.
pub fn encode_index_only_entry_payload(
    document: &Document,
    document_type: DocumentTypeRef,
) -> Result<Vec<u8>, Error> {
    let mut payload = Vec::new();
    for property_name in document_type.entry_payload() {
        let property = document_type
            .properties()
            .get(property_name)
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "an entryPayload property must be a top-level property of its document type",
            )))?;
        let value = document
            .properties()
            .get(property_name)
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "an indexOnly document must carry every entryPayload property: the parser \
                 requires them",
            )))?;
        let encoded = encode_index_only_entry_payload_value(&property.property_type, value)?;
        let length = u16::try_from(encoded.len()).map_err(|_| {
            Error::Drive(DriveError::CorruptedCodeExecution(
                "an entryPayload value exceeds its bound: schema validation admits no such \
                 document",
            ))
        })?;
        payload.extend_from_slice(&length.to_be_bytes());
        payload.extend_from_slice(&encoded);
    }
    Ok(payload)
}

/// Decode an entry item's payload (the bytes after the 32-byte row
/// commitment) back into `document_type`'s entry payload properties. Fails
/// closed on any framing mismatch: a truncated, overlong or misframed
/// payload is a corrupted entry, never a partial document.
pub fn decode_index_only_entry_payload(
    document_type: DocumentTypeRef,
    payload: &[u8],
) -> Result<BTreeMap<String, Value>, Error> {
    let corrupted =
        |message: &'static str| Error::Drive(DriveError::CorruptedCodeExecution(message));
    let mut properties = BTreeMap::new();
    let mut cursor = 0usize;
    for property_name in document_type.entry_payload() {
        let property = document_type
            .properties()
            .get(property_name)
            .ok_or(corrupted(
                "an entryPayload property must be a top-level property of its document type",
            ))?;
        let frame = payload
            .get(cursor..cursor + INDEX_ONLY_ENTRY_PAYLOAD_LENGTH_FRAME as usize)
            .ok_or(corrupted(
                "indexOnly entry payload is truncated before a property's length frame",
            ))?;
        let length = usize::from(u16::from_be_bytes([frame[0], frame[1]]));
        cursor += INDEX_ONLY_ENTRY_PAYLOAD_LENGTH_FRAME as usize;
        let encoded = payload.get(cursor..cursor + length).ok_or(corrupted(
            "indexOnly entry payload is truncated inside a property",
        ))?;
        cursor += length;
        let value = match &property.property_type {
            // These are required values, so an empty frame is an empty
            // byte array or string, never the tree-key null sentinel.
            DocumentPropertyType::ByteArray(_) => Value::Bytes(encoded.to_vec()),
            DocumentPropertyType::String(_) => Value::Text(
                String::from_utf8(encoded.to_vec())
                    .map_err(|_| corrupted("indexOnly entry payload contains invalid UTF-8"))?,
            ),
            // Every other payload type encodes to a fixed width, so an
            // empty frame is a corrupted entry; the tree-key decoder would
            // read it as the null sentinel instead.
            _ if encoded.is_empty() => {
                return Err(corrupted(
                    "indexOnly entry payload carries an empty frame for a fixed-width property",
                ));
            }
            property_type => property_type
                .decode_value_for_tree_keys(encoded)
                .map_err(|e| Error::Protocol(Box::new(e)))?,
        };
        properties.insert(property_name.clone(), value);
    }
    if cursor != payload.len() {
        return Err(corrupted(
            "indexOnly entry payload carries bytes past its last property",
        ));
    }
    Ok(properties)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::data_contract::document_type::DocumentType;
    use dpp::platform_value::platform_value;
    use dpp::platform_value::Identifier;
    use std::collections::BTreeMap;

    /// An indexOnly type whose value slot holds a fixed-width integer and
    /// a bounded string, in that (name) order.
    fn payload_type() -> DocumentType {
        let platform_version = PlatformVersion::latest();
        let config = DataContractConfig::default_for_version(platform_version).expect("config");
        let schema = platform_value!({
            "type": "object",
            "indexOnly": true,
            "documentsMutable": false,
            "canBeDeleted": true,
            "indices": [{ "name": "byOwner", "terminal": "$ownerId" }],
            "entryPayload": ["count", "text"],
            "properties": {
                "count": { "type": "integer", "position": 0 },
                "text": { "type": "string", "maxLength": 8, "position": 1 }
            },
            "required": ["count", "text"],
            "additionalProperties": false
        });
        DocumentType::try_from_schema(
            Identifier::random(),
            1,
            config.version(),
            "entry",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("the payload type parses")
    }

    fn frame(bytes: &[u8]) -> Vec<u8> {
        let mut framed = (bytes.len() as u16).to_be_bytes().to_vec();
        framed.extend_from_slice(bytes);
        framed
    }

    fn count_frame(value: i64) -> Vec<u8> {
        frame(
            &DocumentPropertyType::I64
                .encode_value_for_tree_keys(&Value::I64(value))
                .expect("i64 key"),
        )
    }

    fn decode_error(payload: &[u8]) -> String {
        decode_index_only_entry_payload(payload_type().as_ref(), payload)
            .expect_err("a malformed payload must be refused")
            .to_string()
    }

    #[test]
    fn should_round_trip_a_documents_payload() {
        let document_type = payload_type();
        let platform_version = PlatformVersion::latest();
        let document = document_type
            .random_document(Some(7), platform_version)
            .expect("random document");
        let encoded = encode_index_only_entry_payload(&document, document_type.as_ref())
            .expect("payload encodes");
        let decoded = decode_index_only_entry_payload(document_type.as_ref(), &encoded)
            .expect("payload decodes");
        for name in ["count", "text"] {
            assert_eq!(decoded.get(name), document.properties().get(name), "{name}");
        }
        assert_eq!(decoded.len(), 2);
    }

    #[test]
    fn should_decode_a_well_framed_payload() {
        let mut payload = count_frame(-5);
        payload.extend(frame(b"abc"));
        let decoded =
            decode_index_only_entry_payload(payload_type().as_ref(), &payload).expect("decodes");
        assert_eq!(decoded.get("count"), Some(&Value::I64(-5)));
        assert_eq!(decoded.get("text"), Some(&Value::Text("abc".to_string())));
    }

    #[test]
    fn should_refuse_a_truncated_length_frame() {
        let mut payload = count_frame(1);
        payload.push(0);
        assert!(decode_error(&payload).contains("truncated before a property's length frame"));
    }

    #[test]
    fn should_refuse_a_frame_longer_than_the_payload() {
        let mut payload = count_frame(1);
        payload.extend(frame(b"abc"));
        payload.pop();
        assert!(decode_error(&payload).contains("truncated inside a property"));
    }

    #[test]
    fn should_refuse_bytes_past_the_last_property() {
        let mut payload = count_frame(1);
        payload.extend(frame(b"abc"));
        payload.push(0);
        assert!(decode_error(&payload).contains("past its last property"));
    }

    #[test]
    fn should_refuse_invalid_utf8_in_a_string_payload() {
        let mut payload = count_frame(1);
        payload.extend(frame(&[0xFF, 0xFE]));
        assert!(decode_error(&payload).contains("invalid UTF-8"));
    }

    #[test]
    fn should_refuse_an_empty_frame_for_a_fixed_width_property() {
        let mut payload = frame(&[]);
        payload.extend(frame(b"abc"));
        assert!(decode_error(&payload).contains("empty frame for a fixed-width property"));
    }
}
