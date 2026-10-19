//! The indexOnly **row commitment**: the payload every indexOnly terminal
//! item stores, binding the independently stored index projections of one
//! document back into one logical row.

#[cfg(any(feature = "server", feature = "verify"))]
use crate::drive::document::encode_index_only_entry_payload_value;
#[cfg(any(feature = "server", feature = "verify"))]
use crate::error::drive::DriveError;
#[cfg(any(feature = "server", feature = "verify"))]
use crate::error::Error;
#[cfg(feature = "server")]
use crate::util::type_constants::DEFAULT_HASH_SIZE_U8;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::MAX_INDEX_SIZE;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::{
    index_only_row_commits_created_at, DocumentPropertyType, DocumentTypeRef,
};
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::document::document_methods::DocumentMethodsV0;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::document::{Document, DocumentV0Getters};
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::platform_value::btreemap_extensions::BTreeValueMapPathHelper;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::platform_value::Value;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::util::hash::hash_double;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::version::PlatformVersion;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::ProtocolError;

/// The byte length of an indexOnly terminal item's payload: the row
/// commitment below.
pub const INDEX_ONLY_ROW_COMMITMENT_SIZE: u32 = 32;

/// The value size fee estimation claims for one indexOnly entry item: the
/// commitment plus 32 bytes of padding, because the estimation layers
/// under-count each entry's chain (the serialized item envelope: enum tag,
/// length prefix, flags option; plus the per-entry share of parent-tree
/// aggregate bytes) and estimation must UPPER-bound the applied fee. The
/// padding is per entry, so it scales with a time-range index's bucket
/// fan-out. Sum-bearing entries add the 10-byte sum-item worst case on top.
/// One definition for the entry-insert terminal, the preallocated-tree
/// estimate and the delete-side commitment-probe estimate, so the three
/// cannot drift.
pub const INDEX_ONLY_ITEM_ESTIMATED_VALUE_SIZE: u32 = INDEX_ONLY_ROW_COMMITMENT_SIZE + 32;

/// The **row commitment** stored as every indexOnly terminal item's payload:
/// `hash_double(owner ‖ (name ‖ raw index bytes)* ‖ [$createdAt bytes])`
/// over the document's PRESENT properties in sorted-name order. A required
/// property must be present (an absence there is an internal error); an
/// optional property — a skip property of a skipIfAbsent index, the only
/// kind of optional property the parser admits — simply contributes nothing
/// when absent, its name included.
///
/// This is what binds the independently stored index projections of one
/// document back into one logical row: a delete recomputes the commitment
/// from its submitted values and every probed entry must carry it, so a
/// values tuple spliced from two different creates — even two creates by
/// the same owner — fails the comparison on whichever entry belongs to the
/// other row. Without it, entry existence alone cannot distinguish "one
/// document's projections" from "several documents' projections that
/// happen to coexist". Committing the exact present-set extends that to
/// absence games: a delete carrying a different absence pattern than the
/// create (dropping a skip property, or inventing one) hashes differently and
/// fails every probe, so a skipped index can neither be force-pruned nor
/// left with an orphan entry.
///
/// The variable present-set stays unambiguous under this framing: absent
/// means the property's NAME is not emitted at all, while a present empty
/// value emits `name ‖ 00000000`. Parsing the preimage left-to-right is
/// deterministic because a property name (schema-validated to
/// `[a-zA-Z0-9-_]`, dotted at flattening joints) never contains a 0x00
/// byte, while every length prefix starts with one (index-borne values are
/// capped far below 2^24 bytes), and the one name-less tail segment starts
/// with `$` — so equal preimages imply equal (owner, present-set, values,
/// createdAt). Pinned by the absence tests in the e2e battery.
///
/// A typed array (protocol version 14, which an indexOnly type holds only
/// as the array an index enters a document under once per element) is
/// committed as its whole list, in the document's order: the raw bytes are
/// `(u32 length ‖ element key)*`, each element keyed as its entries are
/// ([`typed_array_commitment_bytes`]). A delete carrying the list reordered,
/// shortened, lengthened or changed hashes differently and fails every probe.
/// An empty or null list is committed as an absent one: the three write the
/// same entries (none under the array's indexes), so a delete may carry
/// any of them.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn index_only_row_commitment(
    document: &Document,
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<[u8; 32], Error> {
    index_only_row_commitment_with_preimage_size(document, document_type, platform_version)
        .map(|(commitment, _)| commitment)
}

/// [`index_only_row_commitment`] plus the byte length of the hashed
/// preimage — the length is what fee accounting sizes the double-SHA256
/// by (`FunctionOp::new_with_byte_count`), so a validation path that
/// computes the commitment can bill the hash it just performed.
#[cfg(any(feature = "server", feature = "verify"))]
pub fn index_only_row_commitment_with_preimage_size(
    document: &Document,
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<([u8; 32], u16), Error> {
    let owner_id = Some(document.owner_id().to_buffer());

    let mut preimage: Vec<u8> = Vec::with_capacity(128);
    preimage.extend_from_slice(document.owner_id().as_bytes());

    let mut properties: Vec<_> = document_type
        .flattened_properties()
        .iter()
        .filter(|(_, property)| !matches!(property.property_type, DocumentPropertyType::Object(_)))
        .collect();
    properties.sort_by_key(|(name, _)| *name);

    for (property_name, property) in properties {
        let raw = if matches!(property.property_type, DocumentPropertyType::TypedArray(_)) {
            // The whole list. An empty one is committed as an absent one,
            // required or not: the array's indexes skip both, and the schema
            // enforces `required`
            match typed_array_commitment_bytes(
                document,
                document_type,
                property_name,
                platform_version,
            )? {
                Some(list) => Some(list),
                None => continue,
            }
        } else if document_type
            .entry_payload()
            .contains(property_name.as_str())
        {
            // An entry payload property is a value, not a key: it is committed
            // through the payload encoding, which carries no 255-byte key cap
            // (the parser bounds it by the field value limit instead).
            match document.properties().get(property_name.as_str()) {
                Some(value) => Some(encode_index_only_entry_payload_value(
                    &property.property_type,
                    value,
                )?),
                None => None,
            }
        } else {
            document.get_raw_for_document_type(
                property_name,
                document_type,
                owner_id,
                platform_version,
            )?
        };
        let Some(raw) = raw else {
            if document_type
                .required_fields()
                .contains(property_name.as_str())
            {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "indexOnly row commitment requires every required property value; the \
                     parser requires them and the transitions carry them",
                )));
            }
            // An optional property (a skipIfAbsent skip property) contributes
            // nothing when absent — not even its name — so the commitment
            // pins the exact present-set.
            continue;
        };
        preimage.extend_from_slice(property_name.as_bytes());
        preimage.extend_from_slice(&(raw.len() as u32).to_be_bytes());
        preimage.extend_from_slice(&raw);
    }

    // `$createdAt` is committed unless only indexes whose entries outlive a
    // delete involve it: a delete then carries no timestamp, and the entries
    // it checks and removes are keyed by none (see
    // `index_only_row_commits_created_at`, shared with the delete's
    // construction and validation). A create's document carries the block
    // time either way.
    if index_only_row_commits_created_at(
        document_type.required_fields(),
        document_type.index_structure(),
    ) {
        if let Some(created_at) = document.created_at() {
            preimage.extend_from_slice(b"$createdAt");
            preimage.extend_from_slice(&created_at.to_be_bytes());
        }
    }

    // Index-bearing properties are bounded far below 64 KiB; saturate
    // instead of failing if a pathological preimage ever exceeds u16.
    let preimage_size = preimage.len().try_into().unwrap_or(u16::MAX);

    Ok((hash_double(preimage), preimage_size))
}

/// The raw bytes the row commitment binds a typed array of `document` by
/// ([`index_only_row_commitment`]): `(u32 length ‖ element key)*` over its
/// elements in the document's order, each element keyed as an index naming
/// the array enters the document under it (`serialize_value_for_key`, the
/// encoding the walkers and probes key its entries with), or `None` for an
/// absent, null or empty list.
#[cfg(any(feature = "server", feature = "verify"))]
fn typed_array_commitment_bytes(
    document: &Document,
    document_type: DocumentTypeRef,
    property_name: &str,
    platform_version: &PlatformVersion,
) -> Result<Option<Vec<u8>>, Error> {
    let elements = match document
        .properties()
        .get_optional_at_path(property_name)
        .map_err(ProtocolError::ValueError)?
    {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::Array(elements)) if elements.is_empty() => return Ok(None),
        Some(Value::Array(elements)) => elements,
        Some(_) => {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "an indexOnly document's typed array holds no list: the schema validates \
                 the values before they are committed",
            )))
        }
    };
    let mut list = Vec::new();
    for element in elements {
        let key =
            document_type.serialize_value_for_key(property_name, element, platform_version)?;
        // An element key is at most 255 bytes (the index key cap)
        list.extend_from_slice(&(key.len() as u32).to_be_bytes());
        list.extend_from_slice(&key);
    }
    Ok(Some(list))
}

/// The widest preimage [`index_only_row_commitment`] can hash for a document
/// of `document_type`, saturating at `u16::MAX` as the measured size does:
/// the owner, every property framed by its name and a `u32` length (a key of
/// at most 255 bytes, a typed array's `maxItems` framed element keys, an
/// entry payload value its declared bound), and the `$createdAt` tail. An
/// insert's dry run bills each entry's commitment hash by it, so the estimate
/// upper-bounds the hash the apply bills by the real preimage, whatever
/// values the dry run's document holds.
#[cfg(feature = "server")]
pub fn index_only_row_commitment_max_preimage_size(
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> u16 {
    // `serialize_value_for_key` refuses a key over 255 bytes
    let key_bound = |property_type: &DocumentPropertyType| -> u32 {
        property_type
            .max_byte_size(platform_version)
            .ok()
            .flatten()
            .map_or(MAX_INDEX_SIZE as u32, |width| {
                u32::from(width).min(MAX_INDEX_SIZE as u32)
            })
    };
    let mut total: u32 = DEFAULT_HASH_SIZE_U8 as u32;
    for (property_name, property) in document_type.flattened_properties() {
        let raw = match &property.property_type {
            DocumentPropertyType::Object(_) => continue,
            DocumentPropertyType::TypedArray(typed_array) => u32::from(typed_array.max_items)
                .saturating_mul(4 + key_bound(&typed_array.item_type)),
            property_type
                if document_type
                    .entry_payload()
                    .contains(property_name.as_str()) =>
            {
                property_type
                    .max_byte_size(platform_version)
                    .ok()
                    .flatten()
                    .map_or(u32::from(u16::MAX), u32::from)
            }
            property_type => key_bound(property_type),
        };
        total = total
            .saturating_add(property_name.len() as u32)
            .saturating_add(4)
            .saturating_add(raw);
    }
    // `$createdAt` and its 8 bytes
    total = total.saturating_add(18);
    u16::try_from(total).unwrap_or(u16::MAX)
}
