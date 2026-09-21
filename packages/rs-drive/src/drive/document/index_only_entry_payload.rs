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

#[cfg(any(feature = "server", feature = "verify"))]
use crate::error::drive::DriveError;
#[cfg(any(feature = "server", feature = "verify"))]
use crate::error::Error;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::{DocumentPropertyType, DocumentTypeRef};
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::document::{Document, DocumentV0Getters};
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::platform_value::Value;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::version::PlatformVersion;
#[cfg(any(feature = "server", feature = "verify"))]
use std::collections::BTreeMap;

/// The bytes of length frame each payload property carries.
pub const INDEX_ONLY_ENTRY_PAYLOAD_LENGTH_FRAME: u32 = 2;

/// The most bytes `document_type`'s entry payload can encode to: the sum
/// of every payload property's declared bound plus its length frame. Zero
/// on a type without an entry payload. The parser guarantees every payload
/// property is bounded, so an unbounded one here is a corrupted type.
#[cfg(any(feature = "server", feature = "verify"))]
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
#[cfg(any(feature = "server", feature = "verify"))]
pub fn index_only_item_estimated_value_size(
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<u32, Error> {
    Ok(
        crate::drive::document::INDEX_ONLY_ITEM_ESTIMATED_VALUE_SIZE.saturating_add(
            index_only_entry_payload_max_size(document_type, platform_version)?,
        ),
    )
}

/// The bytes one entry payload value contributes: raw bytes for a byte
/// array, UTF-8 for a string, and the tree-key encoding's fixed
/// order-preserving widths for the numeric kinds. Unlike a key, a payload
/// value is not capped at 255 bytes — the parser bounds it by the field
/// value limit instead. Also what the row commitment hashes for a payload
/// property, so the commitment and the stored value agree byte for byte.
#[cfg(any(feature = "server", feature = "verify"))]
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
#[cfg(any(feature = "server", feature = "verify"))]
pub fn encode_index_only_entry_payload(
    document: &Document,
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, Error> {
    let _ = platform_version;
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
#[cfg(any(feature = "server", feature = "verify"))]
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
