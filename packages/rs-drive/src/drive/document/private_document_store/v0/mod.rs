#[cfg(feature = "server")]
use crate::drive::document::private_document_store::{
    private_document_stored_entry_size, PRIVATE_DOCUMENT_STORE_KEY,
};
use crate::error::drive::DriveError;
use crate::error::Error;
#[cfg(feature = "server")]
use crate::fees::op::LowLevelDriveOperation;
#[cfg(feature = "server")]
use crate::util::storage_flags::StorageFlags;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::PrivateDocumentConfig;
use dpp::version::PlatformVersion;
#[cfg(feature = "server")]
use grovedb::Element;

/// The chunk power of a store whose stored entries take `stored_entry_size` bytes: the largest
/// that keeps a compacted chunk near the target of `platform_version`'s system limits,
/// `clamp(floor(log2(target / stored_entry_size)), min, max)`. A platform rule, not a contract
/// choice, committed into the store when the store is created: a store keeps the rule of the
/// protocol version that created it, and new numbers are a new limits table.
pub(super) fn private_document_store_chunk_power_v0(
    stored_entry_size: u32,
    platform_version: &PlatformVersion,
) -> Result<u8, Error> {
    let limits = &platform_version.system_limits;
    let (Some(target_chunk_bytes), Some(min_chunk_power), Some(max_chunk_power)) = (
        limits.private_document_store_target_chunk_bytes,
        limits.private_document_store_min_chunk_power,
        limits.private_document_store_max_chunk_power,
    ) else {
        return Err(Error::Drive(DriveError::CorruptedCodeExecution(
            "a private document store is sized only at a protocol version setting the private \
             document store limits",
        )));
    };
    let entries_per_chunk = target_chunk_bytes / stored_entry_size.max(1);
    // At most 31 for a u32
    let power = u8::try_from(entries_per_chunk.checked_ilog2().unwrap_or(0)).unwrap_or(u8::MAX);
    Ok(power.clamp(min_chunk_power, max_chunk_power))
}

/// The store is a single element, inserted directly: grovedb's empty-tree helpers take a tree
/// type, which cannot carry the store's entry size. It carries the contract's storage flags,
/// like the type's other trees.
#[cfg(feature = "server")]
pub(super) fn insert_empty_private_document_store_operation_v0(
    type_path: [&[u8]; 4],
    private_document: PrivateDocumentConfig,
    storage_flags: Option<&StorageFlags>,
    platform_version: &PlatformVersion,
) -> Result<LowLevelDriveOperation, Error> {
    let stored_entry_size = private_document_stored_entry_size(private_document)?;
    let store = Element::empty_private_document_store_with_flags(
        stored_entry_size,
        private_document_store_chunk_power_v0(stored_entry_size, platform_version)?,
        storage_flags.map(StorageFlags::to_element_flags),
    )?;
    Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
        type_path.iter().map(|segment| segment.to_vec()).collect(),
        vec![PRIVATE_DOCUMENT_STORE_KEY],
        store,
    ))
}
