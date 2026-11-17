//! Whether a document's entry in a unique index sits under the one key that index holds for
//! its values, or under the document's id.

use crate::error::Error;
use crate::util::object_size_info::{DocumentInfo, DocumentInfoV0Methods};
use dpp::data_contract::document_type::{DocumentTypeRef, Index};
use dpp::version::PlatformVersion;

/// Whether the entry of `document_info` in the unique `index` sits under the one key the index
/// holds for its values, the key a contest's award takes. `index` is one the insert walkers key
/// by single values, as they key a contested index (no typed array, no bucketing).
///
/// The walkers read each level's key with the call made here (`level_keys`), and a level whose
/// key is empty puts the entry under the document's id instead (`any_fields_null`), where no
/// other document's entry meets it: a value the document lacks, or one that encodes to no
/// bytes, such as an empty byte array.
pub fn unique_index_entry_is_shared(
    document_info: &DocumentInfo,
    index: &Index,
    document_type: DocumentTypeRef,
    owner_id: Option<[u8; 32]>,
    platform_version: &PlatformVersion,
) -> Result<bool, Error> {
    for property in &index.properties {
        let key = document_info
            .get_raw_for_document_type(
                &property.name,
                document_type,
                owner_id,
                None,
                platform_version,
            )?
            .unwrap_or_default();
        if key.is_empty() {
            return Ok(false);
        }
    }
    Ok(true)
}
