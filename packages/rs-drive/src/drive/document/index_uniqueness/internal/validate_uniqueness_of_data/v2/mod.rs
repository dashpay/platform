use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::UniquenessOfDataRequestV1;
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Validates the uniqueness of data for version 2.
    ///
    /// Version 1 with one change: an index property named with a dot, such
    /// as `profile.handle`, is read as a path into nested objects, the way
    /// the insert path reads it. Version 1 looked the whole name up as one
    /// top-level field, found nothing, and skipped the index as incomplete.
    ///
    /// Version 1 reads the document data only to look up the properties of
    /// unique indexes by name, so this resolves those values first, a dotted
    /// name as a path, and hands version 1 a map keyed by the index property
    /// names. A property name holds no dot, so a dotted key hides no field.
    ///
    /// A replace records its changed fields by top-level name, so version 1
    /// never counts a nested property as changed and lets the check find the
    /// document itself. That keeps an edit of a sibling field in the same
    /// object from colliding with the document's own value, and it is safe:
    /// the query can only return the document itself when its indexed values
    /// did not change.
    ///
    /// A path running through a value that is not an object is returned as
    /// an error rather than read as an absent value, which would skip the
    /// index.
    #[inline(always)]
    pub(super) fn validate_uniqueness_of_data_v2(
        &self,
        request: UniquenessOfDataRequestV1,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut index_values = BTreeMap::new();
        for index in request
            .document_type
            .indexes()
            .values()
            .filter(|index| index.unique)
        {
            for property in &index.properties {
                let value = request
                    .data
                    .get_optional_at_path(&property.name)
                    .map_err(|error| Error::Protocol(Box::new(ProtocolError::ValueError(error))))?;
                if let Some(value) = value {
                    index_values.insert(property.name.clone(), value.clone());
                }
            }
        }
        self.validate_uniqueness_of_data_v1(
            UniquenessOfDataRequestV1 {
                data: &index_values,
                ..request
            },
            transaction,
            platform_version,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::{
        UniquenessOfDataRequest, UniquenessOfDataRequestUpdateType, UniquenessOfDataRequestV0,
    };
    use crate::error::drive::DriveError;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::{Document, DocumentV0};
    use dpp::identifier::Identifier;
    use dpp::platform_value::{platform_value, Value};
    use dpp::prelude::DataContract;
    use std::borrow::Cow;
    use std::collections::BTreeSet;

    const ALICE_CARD: [u8; 32] = [0xA1; 32];
    const BOB_CARD: [u8; 32] = [0xB0; 32];

    /// A `card` type whose unique index is on the nested `profile.handle`
    fn build_card_contract(platform_version: &PlatformVersion) -> DataContract {
        DataContractFactory::new(platform_version.protocol_version)
            .expect("expected a contract factory")
            .create_with_value_config(
                Identifier::from([0x0C; 32]),
                0,
                platform_value!({"card": {
                    "type": "object",
                    "documentsMutable": true,
                    "properties": {
                        "profile": {
                            "type": "object",
                            "position": 0,
                            "properties": {
                                "handle": {"type": "string", "maxLength": 63, "position": 0},
                                "bio": {"type": "string", "maxLength": 63, "position": 1}
                            },
                            "required": ["handle"],
                            "additionalProperties": false
                        }
                    },
                    "required": ["profile"],
                    "indices": [
                        {"name": "byHandle", "properties": [{"profile.handle": "asc"}], "unique": true}
                    ],
                    "additionalProperties": false
                }}),
                None,
                None,
            )
            .expect("expected the contract to parse")
            .data_contract_owned()
    }

    fn card_data(handle: &str, bio: &str) -> BTreeMap<String, Value> {
        BTreeMap::from([(
            "profile".to_string(),
            platform_value!({"handle": handle, "bio": bio}),
        )])
    }

    /// A drive holding the card contract and Alice's card under `sam`
    fn setup(platform_version: &'static PlatformVersion) -> (crate::drive::Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = build_card_contract(platform_version);
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply the contract");
        insert_card(&drive, &contract, ALICE_CARD, "sam", platform_version);
        (drive, contract)
    }

    fn insert_card(
        drive: &crate::drive::Drive,
        contract: &DataContract,
        id: [u8; 32],
        handle: &str,
        platform_version: &PlatformVersion,
    ) {
        let document = Document::V0(DocumentV0 {
            id: Identifier::from(id),
            owner_id: Identifier::from(id),
            properties: card_data(handle, "first"),
            revision: Some(1),
            ..Default::default()
        });
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(id),
                    },
                    contract,
                    document_type: contract
                        .document_type_for_name("card")
                        .expect("expected the card type"),
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to add the card");
    }

    fn check(
        drive: &crate::drive::Drive,
        contract: &DataContract,
        document_id: [u8; 32],
        data: &BTreeMap<String, Value>,
        update_type: UniquenessOfDataRequestUpdateType,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let request = UniquenessOfDataRequestV1 {
            contract,
            document_type: contract
                .document_type_for_name("card")
                .expect("expected the card type"),
            owner_id: Identifier::from(document_id),
            creator_id: None,
            document_id: Identifier::from(document_id),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            data,
            update_type,
        };
        drive.validate_uniqueness_of_data(
            UniquenessOfDataRequest::V1(request),
            None,
            platform_version,
        )
    }

    /// A replace whose data changed inside `profile`
    fn profile_changed() -> UniquenessOfDataRequestUpdateType<'static> {
        UniquenessOfDataRequestUpdateType::ChangedDocument {
            changed_owner_id: false,
            changed_updated_at: false,
            changed_transferred_at: false,
            changed_updated_at_block_height: false,
            changed_transferred_at_block_height: false,
            changed_updated_at_core_block_height: false,
            changed_transferred_at_core_block_height: false,
            changed_data_values: Cow::Owned(BTreeSet::from(["profile".to_string()])),
        }
    }

    fn is_duplicate(result: &SimpleConsensusValidationResult) -> bool {
        matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(
                StateError::DuplicateUniqueIndexError(_)
            )]
        )
    }

    #[test]
    fn should_refuse_a_new_document_taking_a_stored_nested_unique_value() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup(platform_version);

        let result = check(
            &drive,
            &contract,
            BOB_CARD,
            &card_data("sam", ""),
            UniquenessOfDataRequestUpdateType::NewDocument,
            platform_version,
        )
        .expect("expected the check to run");
        assert!(is_duplicate(&result), "got {:?}", result.errors);
    }

    #[test]
    fn should_accept_a_new_document_with_a_free_nested_unique_value() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup(platform_version);

        let result = check(
            &drive,
            &contract,
            BOB_CARD,
            &card_data("bob", ""),
            UniquenessOfDataRequestUpdateType::NewDocument,
            platform_version,
        )
        .expect("expected the check to run");
        assert!(result.is_valid(), "got {:?}", result.errors);
    }

    /// Only `profile.bio` changed, but the replace records `profile` as
    /// changed: the check must still find Alice's own card and accept it
    #[test]
    fn should_accept_a_changed_document_keeping_its_nested_value_when_a_sibling_changes() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup(platform_version);

        let result = check(
            &drive,
            &contract,
            ALICE_CARD,
            &card_data("sam", "edited"),
            profile_changed(),
            platform_version,
        )
        .expect("expected the check to run");
        assert!(result.is_valid(), "got {:?}", result.errors);
    }

    #[test]
    fn should_refuse_a_changed_document_moving_into_a_taken_nested_value() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup(platform_version);
        insert_card(&drive, &contract, BOB_CARD, "bob", platform_version);

        let result = check(
            &drive,
            &contract,
            BOB_CARD,
            &card_data("sam", ""),
            profile_changed(),
            platform_version,
        )
        .expect("expected the check to run");
        assert!(is_duplicate(&result), "got {:?}", result.errors);
    }

    /// A path running through a value that is not an object is an error, not
    /// an absent value that would skip the index
    #[test]
    fn should_return_an_error_for_a_path_through_a_value_that_is_not_an_object() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup(platform_version);
        let data = BTreeMap::from([("profile".to_string(), Value::Text("sam".to_string()))]);

        for update_type in [
            UniquenessOfDataRequestUpdateType::NewDocument,
            profile_changed(),
        ] {
            assert!(matches!(
                check(
                    &drive,
                    &contract,
                    BOB_CARD,
                    &data,
                    update_type,
                    platform_version
                ),
                Err(Error::Protocol(error)) if matches!(*error, ProtocolError::ValueError(_))
            ));
        }
    }

    /// Protocol version 13 selects generation 1, which skips the nested index;
    /// the latest selects generation 2, which checks it
    #[test]
    fn should_check_a_nested_unique_index_from_generation_2_only() {
        let platform_version_13 = PlatformVersion::get(13).expect("expected protocol version 13");
        let (drive, contract) = setup(platform_version_13);
        let result = check(
            &drive,
            &contract,
            BOB_CARD,
            &card_data("sam", ""),
            UniquenessOfDataRequestUpdateType::NewDocument,
            platform_version_13,
        )
        .expect("expected the check to run");
        assert!(
            result.is_valid(),
            "generation 1 skips the nested index: {:?}",
            result.errors
        );

        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup(platform_version);
        let result = check(
            &drive,
            &contract,
            BOB_CARD,
            &card_data("sam", ""),
            UniquenessOfDataRequestUpdateType::NewDocument,
            platform_version,
        )
        .expect("expected the check to run");
        assert!(is_duplicate(&result), "got {:?}", result.errors);
    }

    /// Generation 2 takes the V1 request only
    #[test]
    fn should_refuse_a_v0_request_at_generation_2() {
        let platform_version = PlatformVersion::latest();
        let (drive, contract) = setup(platform_version);
        let data = card_data("sam", "");
        let request = UniquenessOfDataRequestV0 {
            contract: &contract,
            document_type: contract
                .document_type_for_name("card")
                .expect("expected the card type"),
            owner_id: Identifier::from(BOB_CARD),
            document_id: Identifier::from(BOB_CARD),
            allow_original: false,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            data: &data,
        };
        assert!(matches!(
            drive.validate_uniqueness_of_data(
                UniquenessOfDataRequest::V0(request),
                None,
                platform_version
            ),
            Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
        ));
    }
}
