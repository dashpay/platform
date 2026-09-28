use crate::drive::Drive;

use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::{
    UniquenessOfDataRequestUpdateType, UniquenessOfDataRequestV1,
};
use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::{
    DriveDocumentQuery, InternalClauses, ResolvedTimeRange, WhereClause, WhereOperator,
};
use dpp::consensus::state::document::duplicate_unique_index_error::DuplicateUniqueIndexError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::{property_names, DocumentV0Getters};
use dpp::platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use dpp::platform_value::{platform_value, Value};
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
    /// This method checks if a given data, within the context of its associated contract and
    /// document type, is unique. If an index is not flagged as unique, it is considered non-problematic.
    /// If all required fields for uniqueness are present and the data is found to be unique,
    /// it returns a successful validation result.
    ///
    /// # Arguments
    ///
    /// * `request`: The data and related metadata to be checked for uniqueness.
    /// * `transaction`: The transaction associated with this check.
    /// * `platform_version`: The version of the platform being used.
    ///
    /// # Returns
    ///
    /// A `Result<SimpleConsensusValidationResult, Error>`, which either:
    ///
    /// * Contains a validation result indicating if the data is unique or not, or
    /// * An error that occurred during the operation.
    #[inline(always)]
    pub(super) fn validate_uniqueness_of_data_v2(
        &self,
        request: UniquenessOfDataRequestV1,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let UniquenessOfDataRequestV1 {
            contract,
            document_type,
            owner_id,
            creator_id,
            document_id,
            created_at,
            updated_at,
            transferred_at,
            created_at_block_height,
            updated_at_block_height,
            transferred_at_block_height,
            created_at_core_block_height,
            updated_at_core_block_height,
            transferred_at_core_block_height,
            data,
            update_type,
        } = request;

        let validation_results = document_type
            .indexes()
            .values()
            .filter_map(|index| {
                if !index.unique {
                    // if an index is not unique there is no issue
                    None
                } else {
                    // A path running through a value that is not an object:
                    // passed on rather than read as an absent value, which
                    // would skip the index
                    let mut path_error = None;
                    let (mut where_queries, allow_original) = match &update_type {
                        UniquenessOfDataRequestUpdateType::NewDocument => {
                            let where_queries = index
                                .properties
                                .iter()
                                .filter_map(|property| {
                                    let value = match property.name.as_str() {
                                        property_names::OWNER_ID => {
                                            platform_value!(owner_id)
                                        }
                                        property_names::CREATOR_ID => {
                                            platform_value!(creator_id?)
                                        }
                                        property_names::CREATED_AT => {
                                            platform_value!(created_at?)
                                        }
                                        property_names::UPDATED_AT => {
                                            platform_value!(updated_at?)
                                        }
                                        property_names::TRANSFERRED_AT => {
                                            platform_value!(transferred_at?)
                                        }
                                        property_names::CREATED_AT_BLOCK_HEIGHT => {
                                            platform_value!(created_at_block_height?)
                                        }
                                        property_names::UPDATED_AT_BLOCK_HEIGHT => {
                                            platform_value!(updated_at_block_height?)
                                        }
                                        property_names::TRANSFERRED_AT_BLOCK_HEIGHT => {
                                            platform_value!(transferred_at_block_height?)
                                        }
                                        property_names::CREATED_AT_CORE_BLOCK_HEIGHT => {
                                            platform_value!(created_at_core_block_height?)
                                        }
                                        property_names::UPDATED_AT_CORE_BLOCK_HEIGHT => {
                                            platform_value!(updated_at_core_block_height?)
                                        }
                                        property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT => {
                                            platform_value!(transferred_at_core_block_height?)
                                        }
                                        _ => {
                                            match data.get_optional_at_path(property.name.as_str())
                                            {
                                                Ok(value) => value?.clone(),
                                                Err(error) => {
                                                    path_error = Some(error);
                                                    return None;
                                                }
                                            }
                                        }
                                    };
                                    Some((
                                        property.name.clone(),
                                        WhereClause {
                                            field: property.name.clone(),
                                            operator: WhereOperator::Equal,
                                            value,
                                        },
                                    ))
                                })
                                .collect::<BTreeMap<String, WhereClause>>();
                            (where_queries, false)
                        }
                        UniquenessOfDataRequestUpdateType::ChangedDocument {
                            changed_owner_id,
                            changed_updated_at,
                            changed_transferred_at,
                            changed_updated_at_block_height,
                            changed_transferred_at_block_height,
                            changed_updated_at_core_block_height,
                            changed_transferred_at_core_block_height,
                            changed_data_values,
                        } => {
                            let mut allow_original = true;
                            let mut exit_early = false;
                            let where_queries = index
                                .properties
                                .iter()
                                .filter_map(|property| {
                                    let value = match property.name.as_str() {
                                        property_names::OWNER_ID => {
                                            if *changed_owner_id {
                                                allow_original = false;
                                            }
                                            platform_value!(owner_id)
                                        }
                                        property_names::CREATOR_ID => {
                                            if let Some(creator_id) = creator_id {
                                                platform_value!(creator_id)
                                            } else {
                                                exit_early = true;
                                                return None;
                                            }
                                        }
                                        property_names::CREATED_AT => {
                                            if let Some(created_at) = created_at {
                                                platform_value!(created_at)
                                            } else {
                                                exit_early = true;
                                                return None;
                                            }
                                        }
                                        property_names::UPDATED_AT => {
                                            if *changed_updated_at {
                                                allow_original = false;
                                            }
                                            if let Some(updated_at) = updated_at {
                                                platform_value!(updated_at)
                                            } else {
                                                exit_early = true;
                                                return None;
                                            }
                                        }
                                        property_names::TRANSFERRED_AT => {
                                            if *changed_transferred_at {
                                                allow_original = false;
                                            }
                                            if let Some(transferred_at) = transferred_at {
                                                platform_value!(transferred_at)
                                            } else {
                                                exit_early = true;
                                                return None;
                                            }
                                        }
                                        property_names::CREATED_AT_BLOCK_HEIGHT => {
                                            if let Some(created_at_block_height) =
                                                created_at_block_height
                                            {
                                                platform_value!(created_at_block_height)
                                            } else {
                                                exit_early = true;
                                                return None;
                                            }
                                        }
                                        property_names::UPDATED_AT_BLOCK_HEIGHT => {
                                            if *changed_updated_at_block_height {
                                                allow_original = false;
                                            }
                                            if let Some(updated_at_block_height) =
                                                updated_at_block_height
                                            {
                                                platform_value!(updated_at_block_height)
                                            } else {
                                                exit_early = true;
                                                return None;
                                            }
                                        }
                                        property_names::TRANSFERRED_AT_BLOCK_HEIGHT => {
                                            if *changed_transferred_at_block_height {
                                                allow_original = false;
                                            }
                                            if let Some(transferred_at_block_height) =
                                                transferred_at_block_height
                                            {
                                                platform_value!(transferred_at_block_height)
                                            } else {
                                                exit_early = true;
                                                return None;
                                            }
                                        }
                                        property_names::CREATED_AT_CORE_BLOCK_HEIGHT => {
                                            if let Some(created_at_core_block_height) =
                                                created_at_core_block_height
                                            {
                                                platform_value!(created_at_core_block_height)
                                            } else {
                                                exit_early = true;
                                                return None;
                                            }
                                        }
                                        property_names::UPDATED_AT_CORE_BLOCK_HEIGHT => {
                                            if *changed_updated_at_core_block_height {
                                                allow_original = false;
                                            }
                                            if let Some(updated_at_core_block_height) =
                                                updated_at_core_block_height
                                            {
                                                platform_value!(updated_at_core_block_height)
                                            } else {
                                                exit_early = true;
                                                return None;
                                            }
                                        }
                                        property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT => {
                                            if *changed_transferred_at_core_block_height {
                                                allow_original = false;
                                            }
                                            if let Some(transferred_at_core_block_height) =
                                                transferred_at_core_block_height
                                            {
                                                platform_value!(transferred_at_core_block_height)
                                            } else {
                                                exit_early = true;
                                                return None;
                                            }
                                        }
                                        _ => {
                                            let value = match data
                                                .get_optional_at_path(property.name.as_str())
                                            {
                                                Ok(value) => value,
                                                Err(error) => {
                                                    path_error = Some(error);
                                                    return None;
                                                }
                                            };
                                            if let Some(value) = value {
                                                // If the property is not none then the uniqueness should exist.
                                                // `changed_data_values` holds top-level names, so a
                                                // nested property never matches here and keeps
                                                // `allow_original`: marking it changed whenever its
                                                // object changed would refuse an edit of a sibling
                                                // field, since the query would then find this
                                                // document under its own unchanged value. Keeping
                                                // it is safe: the query can only return this
                                                // document when its indexed values did not change.
                                                if changed_data_values.get(&property.name).is_some()
                                                {
                                                    allow_original = false;
                                                }
                                                value.clone()
                                            } else {
                                                // If any of the index is null then the uniqueness no longer exists
                                                exit_early = true;
                                                return None;
                                            }
                                        }
                                    };
                                    Some((
                                        property.name.clone(),
                                        WhereClause {
                                            field: property.name.clone(),
                                            operator: WhereOperator::Equal,
                                            value,
                                        },
                                    ))
                                })
                                .collect::<BTreeMap<String, WhereClause>>();
                            if exit_early && path_error.is_none() {
                                return None;
                            } else {
                                (where_queries, allow_original)
                            }
                        }
                    };

                    if let Some(error) = path_error {
                        return Some(Err(Error::Protocol(Box::new(ProtocolError::ValueError(
                            error,
                        )))));
                    }

                    if where_queries.len() < index.properties.len() {
                        // there are empty fields, which means that the index is no longer unique
                        None
                    } else {
                        // A unique time-range index stores bucket *starts*
                        // under its first property, never raw timestamps, so
                        // probing it with the candidate document's own
                        // timestamp would look in a key that no document ever
                        // occupies and report every duplicate as unique.
                        // Rewrite the source equality to the containing
                        // bucket, and record the rewrite as provenance so
                        // index selection admits the bucketed index (see
                        // `index_admissible_for_resolved_time_range`): this is
                        // a legitimate internal producer of a resolved bucket
                        // equality — the value is derived deterministically
                        // from the candidate document's own timestamp through
                        // the contract's transform, so every node computes the
                        // identical clause.
                        //
                        // On the `ChangedDocument` path this stays correct
                        // without any old-vs-new bucket tracking (which the
                        // request has no field for — note the arm carries no
                        // `changed_created_at` flag): a unique time-range
                        // index is validated to bucket `$createdAt`, which is
                        // immutable across updates, so the bucket component of
                        // the tuple never moves and `allow_original` keeps its
                        // meaning — the tuple changed exactly when one of the
                        // index's other properties changed.
                        let mut resolved_time_ranges = Vec::new();
                        if let Some(transform) = &index.time_range {
                            let Some(clause) = where_queries.get_mut(transform.source.as_str())
                            else {
                                // Unreachable: the transform's source is
                                // validated to be the index's first property,
                                // and the count check above established that
                                // every property produced a clause.
                                return Some(Err(Error::Drive(
                                    DriveError::CorruptedCodeExecution(
                                        "a time-range index's source must be one of its \
                                         properties",
                                    ),
                                )));
                            };
                            // The clause value was built by `platform_value!`
                            // from an `Option<TimestampMillis>`, so it is a
                            // `U64`; `I64` is accepted defensively because a
                            // non-system-timestamp source would arrive through
                            // the document data map. Anything else is
                            // unreachable under a validated contract and must
                            // fail loudly: silently skipping this index's
                            // check would let the collision surface later as
                            // a corrupted-index insert error, because the
                            // write path stores a non-timestamp value under
                            // its raw key rather than dropping it.
                            let timestamp =
                                match &clause.value {
                                    Value::U64(timestamp) => *timestamp,
                                    Value::I64(timestamp) => match u64::try_from(*timestamp) {
                                        Ok(timestamp) => timestamp,
                                        Err(_) => return Some(Err(Error::Drive(
                                            DriveError::CorruptedCodeExecution(
                                                "a unique time-range index's source value must \
                                                 be a millisecond timestamp",
                                            ),
                                        ))),
                                    },
                                    _ => {
                                        return Some(Err(Error::Drive(
                                            DriveError::CorruptedCodeExecution(
                                                "a unique time-range index's source value must be \
                                             a millisecond timestamp",
                                            ),
                                        )))
                                    }
                                };
                            // A validated unique time-range index has overlap
                            // factor 1 (range == step), so any real timestamp
                            // yields exactly one containing bucket. An empty
                            // result means the timestamp falls in the
                            // sub-`step` epoch sliver before the grid's phase
                            // anchor: such documents produce no index entries
                            // at all, so they cannot collide with anything
                            // under this index and the whole check is skipped
                            // for it.
                            let bucket_start = *transform.containing_buckets(timestamp).first()?;
                            clause.value = platform_value!(bucket_start);
                            resolved_time_ranges.push(ResolvedTimeRange {
                                transform: transform.clone(),
                            });
                        }

                        let query = DriveDocumentQuery {
                            contract,
                            document_type,
                            internal_clauses: InternalClauses {
                                primary_key_in_clause: None,
                                primary_key_equal_clause: None,
                                in_clauses: Vec::new(),
                                range_clause: None,
                                equal_clauses: where_queries,
                            },
                            offset: None,
                            limit: Some(1),
                            order_by: Default::default(),
                            start_at: None,
                            start_at_included: false,
                            block_time_ms: None,
                            resolved_time_ranges,
                            sub_queries: vec![],
                        };

                        // todo: deal with cost of this operation
                        let query_result = self.query_documents(
                            query,
                            None,
                            false,
                            transaction,
                            Some(platform_version.protocol_version),
                        );
                        match query_result {
                            Ok(query_outcome) => {
                                let documents = query_outcome.documents_owned();
                                let would_be_unique = documents.is_empty()
                                    || (allow_original
                                        && documents.len() == 1
                                        && documents[0].id() == document_id);
                                if would_be_unique {
                                    Some(Ok(SimpleConsensusValidationResult::default()))
                                } else {
                                    Some(Ok(SimpleConsensusValidationResult::new_with_error(
                                        StateError::DuplicateUniqueIndexError(
                                            DuplicateUniqueIndexError::new(
                                                document_id,
                                                index.property_names(),
                                            ),
                                        )
                                        .into(),
                                    )))
                                }
                            }
                            Err(e) => Some(Err(e)),
                        }
                    }
                }
            })
            .collect::<Result<Vec<SimpleConsensusValidationResult>, Error>>()?;

        Ok(SimpleConsensusValidationResult::merge_many_errors(
            validation_results,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::{
        UniquenessOfDataRequest, UniquenessOfDataRequestV0,
    };
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::{Document, DocumentV0};
    use dpp::identifier::Identifier;
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
