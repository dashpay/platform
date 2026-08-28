//! The indexOnly **row commitment**: the payload every indexOnly terminal
//! item stores, binding the independently stored index projections of one
//! document back into one logical row.

#[cfg(any(feature = "server", feature = "verify"))]
use crate::error::drive::DriveError;
#[cfg(any(feature = "server", feature = "verify"))]
use crate::error::Error;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::{DocumentPropertyType, DocumentTypeRef};
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::document::document_methods::DocumentMethodsV0;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::document::{Document, DocumentV0Getters};
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::util::hash::hash_double;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::version::PlatformVersion;

/// The byte length of an indexOnly terminal item's payload: the row
/// commitment below.
pub const INDEX_ONLY_ROW_COMMITMENT_SIZE: u32 = 32;

/// The **row commitment** stored as every indexOnly terminal item's payload:
/// `hash_double(owner ‖ (name ‖ raw index bytes)* ‖ [$createdAt bytes])`
/// over ALL of the document's properties in sorted-name order.
///
/// This is what binds the independently stored index projections of one
/// document back into one logical row: a delete recomputes the commitment
/// from its submitted values and every probed entry must carry it, so a
/// values tuple spliced from two different creates — even two creates by
/// the same owner — fails the comparison on whichever entry belongs to the
/// other row. Without it, entry existence alone cannot distinguish "one
/// document's projections" from "several documents' projections that
/// happen to coexist".
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

    let mut property_names: Vec<&String> = document_type
        .flattened_properties()
        .iter()
        .filter(|(_, property)| !matches!(property.property_type, DocumentPropertyType::Object(_)))
        .map(|(name, _)| name)
        .collect();
    property_names.sort();

    for property_name in property_names {
        let raw = document
            .get_raw_for_document_type(property_name, document_type, owner_id, platform_version)?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "indexOnly row commitment requires every property value; the parser \
                 requires them all and the transitions carry them all",
            )))?;
        preimage.extend_from_slice(property_name.as_bytes());
        preimage.extend_from_slice(&(raw.len() as u32).to_be_bytes());
        preimage.extend_from_slice(&raw);
    }

    if let Some(created_at) = document.created_at() {
        preimage.extend_from_slice(b"$createdAt");
        preimage.extend_from_slice(&created_at.to_be_bytes());
    }

    // Index-bearing properties are bounded far below 64 KiB; saturate
    // instead of failing if a pathological preimage ever exceeds u16.
    let preimage_size = preimage.len().try_into().unwrap_or(u16::MAX);

    Ok((hash_double(preimage), preimage_size))
}
