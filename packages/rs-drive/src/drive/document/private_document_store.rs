//! The store of a private document type (protocol version 15). A private type's entries are
//! fixed-size encrypted blobs the chain never reads, appended by an anonymous transition paid
//! from the shielded pool. They live in a grovedb `PrivateDocumentStore` at the type's key
//! `[0]`, where every other type keeps its primary-key tree, and the type has no index trees.

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::data_contract::document_type::PrivateDocumentConfig;
use grovedb::Element;

/// The bytes Drive writes before each entry a client appends: the block height (u64, little
/// endian) and the block time in milliseconds (u64, little endian). They give the recipient a
/// time it can trust, which nothing written inside a ciphertext could be, and leak nothing:
/// the block's height and time are public already.
pub const PRIVATE_DOCUMENT_ENTRY_HEADER_SIZE: u32 = 16;

/// The size a compacted chunk of a private document store is kept near.
pub const PRIVATE_DOCUMENT_STORE_TARGET_CHUNK_BYTES: u32 = 512 * 1024;

/// The smallest chunk power a private document store takes, whatever its entry size.
pub const PRIVATE_DOCUMENT_STORE_MIN_CHUNK_POWER: u8 = 4;

/// The largest chunk power a private document store takes, whatever its entry size.
pub const PRIVATE_DOCUMENT_STORE_MAX_CHUNK_POWER: u8 = 11;

/// The key of a private type's store under its document type tree: the key every other type
/// keeps its primary-key tree at.
pub const PRIVATE_DOCUMENT_STORE_KEY: u8 = 0;

/// The bytes each stored entry of a private type takes: Drive's header and the client's entry.
pub fn private_document_stored_entry_size(
    private_document: PrivateDocumentConfig,
) -> Result<u32, Error> {
    private_document
        .entry_size()
        .checked_add(PRIVATE_DOCUMENT_ENTRY_HEADER_SIZE)
        .ok_or_else(|| {
            Error::Drive(DriveError::CorruptedDriveState(format!(
                "a private document type declares entries of {} bytes, more than a store holds",
                private_document.entry_size()
            )))
        })
}

/// The chunk power of a store whose stored entries take `stored_entry_size` bytes: the largest
/// that keeps a compacted chunk near [`PRIVATE_DOCUMENT_STORE_TARGET_CHUNK_BYTES`],
/// `clamp(floor(log2(target / stored_entry_size)), min, max)`. A platform rule, not a contract
/// choice: it is committed into the store when the store is created, so a later change to the
/// rule would reach only stores created after it.
pub fn private_document_store_chunk_power(stored_entry_size: u32) -> u8 {
    let entries_per_chunk = PRIVATE_DOCUMENT_STORE_TARGET_CHUNK_BYTES / stored_entry_size.max(1);
    // At most 31 for a u32
    let power = u8::try_from(entries_per_chunk.checked_ilog2().unwrap_or(0)).unwrap_or(u8::MAX);
    power.clamp(
        PRIVATE_DOCUMENT_STORE_MIN_CHUNK_POWER,
        PRIVATE_DOCUMENT_STORE_MAX_CHUNK_POWER,
    )
}

/// The empty store of a private type, as its contract creates it.
pub fn empty_private_document_store(
    private_document: PrivateDocumentConfig,
    storage_flags: Option<&StorageFlags>,
) -> Result<Element, Error> {
    let stored_entry_size = private_document_stored_entry_size(private_document)?;
    Ok(Element::empty_private_document_store_with_flags(
        stored_entry_size,
        private_document_store_chunk_power(stored_entry_size),
        storage_flags.map(StorageFlags::to_element_flags),
    )?)
}

/// The operation creating the empty store of a private type under its document type tree,
/// `type_path`. The store is a single element, inserted directly: grovedb's empty-tree helpers
/// take a tree type, which cannot carry the store's entry size.
pub fn insert_empty_private_document_store_operation(
    type_path: [&[u8]; 4],
    private_document: PrivateDocumentConfig,
    storage_flags: Option<&StorageFlags>,
) -> Result<LowLevelDriveOperation, Error> {
    Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
        type_path.iter().map(|segment| segment.to_vec()).collect(),
        vec![PRIVATE_DOCUMENT_STORE_KEY],
        empty_private_document_store(private_document, storage_flags)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::{Drive, RootTree};
    use crate::error::query::QuerySyntaxError;
    use crate::query::DriveDocumentQuery;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::data_contract::DataContract;
    use dpp::platform_value::string_encoding::Encoding;
    use dpp::platform_value::Identifier;
    use dpp::version::PlatformVersion;
    use grovedb_path::SubtreePath;
    use serde_json::json;

    const CONTRACT_ID: [u8; 32] = [7; 32];

    /// A contract of `version` holding a public `note` type and, when `with_secret`, the private
    /// `secret` type of 585-byte entries.
    fn contract(version: u32, with_secret: bool) -> DataContract {
        let mut document_schemas = json!({
            "note": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 63, "position": 0 }
                },
                "additionalProperties": false
            }
        });
        if with_secret {
            document_schemas["secret"] = json!({
                "type": "object",
                "private": { "entrySize": 585 },
                "documentsMutable": false,
                "canBeDeleted": false,
                "properties": {
                    "accountReference": {
                        "type": "integer", "minimum": 0, "maximum": 4294967295u64, "position": 0
                    }
                },
                "additionalProperties": false
            });
        }
        let contract = json!({
            "$formatVersion": "1",
            "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
            "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
            "version": version,
            "documentSchemas": document_schemas
        });
        DataContract::from_value(
            dpp::platform_value::to_value(contract).expect("the contract converts"),
            true,
            PlatformVersion::latest(),
        )
        .expect("the contract registers")
    }

    /// The element at the key `[0]` of the document type `type_name`.
    fn element_at_key_zero(drive: &Drive, type_name: &str) -> Element {
        let type_path: [&[u8]; 4] = [
            &[RootTree::DataContractDocuments as u8],
            &CONTRACT_ID,
            &[1],
            type_name.as_bytes(),
        ];
        drive
            .grove
            .get_raw(
                SubtreePath::from(&type_path[..]),
                &[PRIVATE_DOCUMENT_STORE_KEY],
                None,
                &PlatformVersion::latest().drive.grove_version,
            )
            .unwrap()
            .expect("expected an element at the type's key [0]")
    }

    fn assert_holds_an_empty_store_of_585_byte_entries(drive: &Drive) {
        let element = element_at_key_zero(drive, "secret");
        assert!(
            matches!(element, Element::PrivateDocumentStore(0, 601, 9, _)),
            "expected an empty store of 16 + 585 byte entries at chunk power 9, got {element:?}"
        );
    }

    #[test]
    fn should_create_the_store_when_a_contract_is_registered() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        drive
            .apply_contract(
                &contract(1, true),
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to register the contract");

        assert_holds_an_empty_store_of_585_byte_entries(&drive);
        // The public type of the same contract keeps its primary-key tree
        assert!(element_at_key_zero(&drive, "note").is_any_tree());
    }

    #[test]
    fn should_create_the_store_of_a_private_type_an_update_adds() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        drive
            .apply_contract(
                &contract(1, false),
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to register the contract");
        drive
            .apply_contract(
                &contract(2, true),
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to update the contract");

        assert_holds_an_empty_store_of_585_byte_entries(&drive);
    }

    /// The fee of registering a contract with a private type, or of an update adding one, is
    /// estimated without the store existing yet, as check_tx estimates it.
    #[test]
    fn should_estimate_a_registration_and_an_update_creating_a_store() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let estimated = drive
            .apply_contract(
                &contract(1, true),
                BlockInfo::default(),
                false,
                None,
                None,
                platform_version,
            )
            .expect("expected to estimate the registration");
        assert!(estimated.processing_fee > 0);

        drive
            .apply_contract(
                &contract(1, false),
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to register the contract");
        drive
            .apply_contract(
                &contract(2, true),
                BlockInfo::default(),
                false,
                None,
                None,
                platform_version,
            )
            .expect("expected to estimate the update");
    }

    #[test]
    fn should_refuse_a_document_query_on_a_private_type() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let contract = contract(1, true);
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
            )
            .expect("expected to register the contract");

        let query = DriveDocumentQuery::from_sql_expr(
            "select * from secret",
            &contract,
            Some(&drive.config),
            platform_version,
        )
        .expect("expected the query to parse");
        let refusal = query
            .construct_path_query(None, platform_version)
            .expect_err("a private type has no documents to query");
        assert!(
            matches!(
                &refusal,
                Error::Query(QuerySyntaxError::Unsupported(message))
                    if message.contains("private document type")
            ),
            "{refusal:?}"
        );
        let refusal = drive
            .query_documents(
                query,
                None,
                false,
                None,
                Some(platform_version.protocol_version),
            )
            .expect_err("a private type has no documents to query");
        assert!(
            matches!(refusal, Error::Query(QuerySyntaxError::Unsupported(_))),
            "{refusal:?}"
        );
    }

    #[test]
    fn should_keep_a_compacted_chunk_near_the_target() {
        // 16 + 585 = 601 bytes: 872 entries fit, 2^9 = 512 is the largest power below
        assert_eq!(private_document_store_chunk_power(601), 9);
        // 16 + 4096 = 4112 bytes: 127 entries fit, 2^6 = 64
        assert_eq!(private_document_store_chunk_power(4112), 6);
    }

    #[test]
    fn should_clamp_the_chunk_power() {
        assert_eq!(
            private_document_store_chunk_power(17),
            PRIVATE_DOCUMENT_STORE_MAX_CHUNK_POWER
        );
        assert_eq!(
            private_document_store_chunk_power(u16::MAX as u32),
            PRIVATE_DOCUMENT_STORE_MIN_CHUNK_POWER
        );
        assert_eq!(
            private_document_store_chunk_power(0),
            PRIVATE_DOCUMENT_STORE_MAX_CHUNK_POWER
        );
    }
}
