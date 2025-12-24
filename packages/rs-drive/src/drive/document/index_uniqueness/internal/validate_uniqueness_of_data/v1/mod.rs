use crate::drive::Drive;

use crate::drive::document::index_uniqueness::internal::validate_uniqueness_of_data::{
    UniquenessOfDataRequestUpdateType, UniquenessOfDataRequestV1,
};
use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
use crate::error::Error;
use crate::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use dpp::consensus::state::document::duplicate_unique_index_error::DuplicateUniqueIndexError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::{property_names, DocumentV0Getters};
use dpp::platform_value::platform_value;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Validates the uniqueness of data for version 1.
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
    pub(super) fn validate_uniqueness_of_data_v1(
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
                    let (where_queries, allow_original) = match &update_type {
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
                                            if let Some(creator_id) = creator_id {
                                                platform_value!(creator_id)
                                            } else {
                                                return None;
                                            }
                                        }
                                        property_names::CREATED_AT => {
                                            if let Some(created_at) = created_at {
                                                platform_value!(created_at)
                                            } else {
                                                return None;
                                            }
                                        }
                                        property_names::UPDATED_AT => {
                                            if let Some(updated_at) = updated_at {
                                                platform_value!(updated_at)
                                            } else {
                                                return None;
                                            }
                                        }
                                        property_names::TRANSFERRED_AT => {
                                            if let Some(transferred_at) = transferred_at {
                                                platform_value!(transferred_at)
                                            } else {
                                                return None;
                                            }
                                        }
                                        property_names::CREATED_AT_BLOCK_HEIGHT => {
                                            if let Some(created_at_block_height) =
                                                created_at_block_height
                                            {
                                                platform_value!(created_at_block_height)
                                            } else {
                                                return None;
                                            }
                                        }
                                        property_names::UPDATED_AT_BLOCK_HEIGHT => {
                                            if let Some(updated_at_block_height) =
                                                updated_at_block_height
                                            {
                                                platform_value!(updated_at_block_height)
                                            } else {
                                                return None;
                                            }
                                        }
                                        property_names::TRANSFERRED_AT_BLOCK_HEIGHT => {
                                            if let Some(transferred_at_block_height) =
                                                transferred_at_block_height
                                            {
                                                platform_value!(transferred_at_block_height)
                                            } else {
                                                return None;
                                            }
                                        }
                                        property_names::CREATED_AT_CORE_BLOCK_HEIGHT => {
                                            if let Some(created_at_core_block_height) =
                                                created_at_core_block_height
                                            {
                                                platform_value!(created_at_core_block_height)
                                            } else {
                                                return None;
                                            }
                                        }
                                        property_names::UPDATED_AT_CORE_BLOCK_HEIGHT => {
                                            if let Some(updated_at_core_block_height) =
                                                updated_at_core_block_height
                                            {
                                                platform_value!(updated_at_core_block_height)
                                            } else {
                                                return None;
                                            }
                                        }
                                        property_names::TRANSFERRED_AT_CORE_BLOCK_HEIGHT => {
                                            if let Some(transferred_at_core_block_height) =
                                                transferred_at_core_block_height
                                            {
                                                platform_value!(transferred_at_core_block_height)
                                            } else {
                                                return None;
                                            }
                                        }
                                        _ => {
                                            if let Some(value) = data.get(property.name.as_str()) {
                                                value.clone()
                                            } else {
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
                                            if let Some(value) = data.get(property.name.as_str()) {
                                                // If the property is not none then the uniqueness should exist
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
                            if exit_early {
                                return None;
                            } else {
                                (where_queries, allow_original)
                            }
                        }
                    };

                    if where_queries.len() < index.properties.len() {
                        // there are empty fields, which means that the index is no longer unique
                        None
                    } else {
                        let query = DriveDocumentQuery {
                            contract,
                            document_type,
                            internal_clauses: InternalClauses {
                                primary_key_in_clause: None,
                                primary_key_equal_clause: None,
                                in_clause: None,
                                range_clause: None,
                                equal_clauses: where_queries,
                            },
                            offset: None,
                            limit: Some(1),
                            order_by: Default::default(),
                            start_at: None,
                            start_at_included: false,
                            block_time_ms: None,
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
