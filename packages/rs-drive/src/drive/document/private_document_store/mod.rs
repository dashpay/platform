//! The store of a private document type (protocol version 15). A private type's entries are
//! fixed-size encrypted blobs the chain never reads, appended by an anonymous transition paid
//! from the shielded pool. They live in a grovedb `PrivateDocumentStore` at the type's key
//! `[0]`, where every other type keeps its primary-key tree, and the type has no index trees.

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::data_contract::document_type::PrivateDocumentConfig;
use dpp::version::PlatformVersion;
use v0::insert_empty_private_document_store_operation_v0;

mod v0;

/// The bytes Drive writes before each entry a client appends: the block height (u64, little
/// endian) and the block time in milliseconds (u64, little endian). They give the recipient a
/// time it can trust, which nothing written inside a ciphertext could be, and leak nothing:
/// the block's height and time are public already. The size of a layout, not a tunable
/// number: a different header is a different entry format, with its own append generation.
pub const PRIVATE_DOCUMENT_ENTRY_HEADER_SIZE: u32 = 16;

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

/// The operation creating the empty store of a private type under its document type tree,
/// `type_path`, sized from `platform_version`'s private document store limits. Only a protocol
/// version admitting private types selects a version of it.
pub fn insert_empty_private_document_store_operation(
    type_path: [&[u8]; 4],
    private_document: PrivateDocumentConfig,
    storage_flags: Option<&StorageFlags>,
    platform_version: &PlatformVersion,
) -> Result<LowLevelDriveOperation, Error> {
    match platform_version
        .drive
        .methods
        .document
        .insert_empty_private_document_store_operation
    {
        Some(0) => insert_empty_private_document_store_operation_v0(
            type_path,
            private_document,
            storage_flags,
            platform_version,
        ),
        Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
            method: "insert_empty_private_document_store_operation".to_string(),
            known_versions: vec![0],
            received: version,
        })),
        None => Err(Error::Drive(DriveError::VersionNotActive {
            method: "insert_empty_private_document_store_operation".to_string(),
            known_versions: vec![0],
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::v0::private_document_store_chunk_power_v0;
    use super::*;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
    use grovedb::Element;
    use crate::drive::{Drive, RootTree};
    use crate::error::query::QuerySyntaxError;
    use crate::query::DriveDocumentQuery;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::data_contract::DataContract;
    use dpp::fee::fee_result::FeeResult;
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

    /// Registers `contract` on `drive`, or estimates its registration when `apply` is false.
    fn register(drive: &Drive, contract: &DataContract, apply: bool) -> FeeResult {
        drive
            .apply_contract(
                contract,
                BlockInfo::default(),
                apply,
                None,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to register the contract, or to estimate it")
    }

    fn assert_the_estimate_covers(estimated: &FeeResult, applied: &FeeResult) {
        assert!(
            estimated.processing_fee >= applied.processing_fee,
            "estimated processing {} under the applied {}",
            estimated.processing_fee,
            applied.processing_fee
        );
        assert!(
            estimated.storage_fee >= applied.storage_fee,
            "estimated storage {} under the applied {}",
            estimated.storage_fee,
            applied.storage_fee
        );
    }

    /// The fee of registering a contract with a private type, or of an update adding one, is
    /// estimated without the store existing yet, as check_tx estimates it, and the estimate
    /// covers what creating the store costs.
    #[test]
    fn should_estimate_a_registration_creating_a_store_at_its_applied_fees_or_more() {
        let estimated = register(
            &setup_drive_with_initial_state_structure(None),
            &contract(1, true),
            false,
        );
        let applied = register(
            &setup_drive_with_initial_state_structure(None),
            &contract(1, true),
            true,
        );
        assert_the_estimate_covers(&estimated, &applied);
    }

    #[test]
    fn should_estimate_an_update_creating_a_store_at_its_applied_fees_or_more() {
        let estimating_drive = setup_drive_with_initial_state_structure(None);
        let applying_drive = setup_drive_with_initial_state_structure(None);
        register(&estimating_drive, &contract(1, false), true);
        register(&applying_drive, &contract(1, false), true);

        let estimated = register(&estimating_drive, &contract(2, true), false);
        let applied = register(&applying_drive, &contract(2, true), true);
        assert_the_estimate_covers(&estimated, &applied);
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
        let platform_version = PlatformVersion::latest();
        // 16 + 585 = 601 bytes: 872 entries fit, 2^9 = 512 is the largest power below
        assert_eq!(
            private_document_store_chunk_power_v0(601, platform_version).unwrap(),
            9
        );
        // 16 + 4096 = 4112 bytes: 127 entries fit, 2^6 = 64
        assert_eq!(
            private_document_store_chunk_power_v0(4112, platform_version).unwrap(),
            6
        );
    }

    #[test]
    fn should_clamp_the_chunk_power() {
        let platform_version = PlatformVersion::latest();
        let limits = &platform_version.system_limits;
        let max_chunk_power = limits.private_document_store_max_chunk_power.unwrap();
        let min_chunk_power = limits.private_document_store_min_chunk_power.unwrap();
        assert_eq!(
            private_document_store_chunk_power_v0(17, platform_version).unwrap(),
            max_chunk_power
        );
        assert_eq!(
            private_document_store_chunk_power_v0(u16::MAX as u32, platform_version).unwrap(),
            min_chunk_power
        );
        assert_eq!(
            private_document_store_chunk_power_v0(0, platform_version).unwrap(),
            max_chunk_power
        );
    }

    /// Only a protocol version admitting private types selects a version of the store's
    /// creation: protocol version 14 creates no store.
    #[test]
    fn should_create_a_store_only_at_a_protocol_version_admitting_private_types() {
        let protocol_version_14 = PlatformVersion::get(14).expect("protocol version 14");
        assert!(protocol_version_14
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .parse_private_document
            .is_none());
        let secret_type_path: [&[u8]; 4] = [
            &[RootTree::DataContractDocuments as u8],
            &CONTRACT_ID,
            &[1],
            b"secret",
        ];
        let private_document = contract(1, true)
            .document_type_for_name("secret")
            .expect("the secret type")
            .private_document()
            .expect("a private type");
        assert!(matches!(
            insert_empty_private_document_store_operation(
                secret_type_path,
                private_document,
                None,
                protocol_version_14,
            ),
            Err(Error::Drive(DriveError::VersionNotActive { .. }))
        ));
        assert!(insert_empty_private_document_store_operation(
            secret_type_path,
            private_document,
            None,
            PlatformVersion::latest(),
        )
        .is_ok());
    }
}
