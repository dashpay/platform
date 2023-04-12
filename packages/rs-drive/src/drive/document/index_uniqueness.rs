// MIT LICENSE
//
// Copyright (c) 2023 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Check Index uniqueness for documents.
//!
//! This module implements functions in Drive relevant to checking if a document validates all
//! uniqueness constraints.
//!

use crate::contract::Contract;
use crate::drive::query::QueryDocumentsOutcome;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::{DriveQuery, InternalClauses, WhereClause, WhereOperator};
use dpp::data_contract::document_type::{DocumentType, Index};
use dpp::document::document_transition::{
    DocumentCreateTransitionAction, DocumentReplaceTransitionAction, DocumentTransitionExt,
};
use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::{DocumentTransition, TimestampMillis};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::StateError;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

struct UniquenessOfDataRequest<'a> {
    contract: &'a Contract,
    document_type: &'a DocumentType,
    owner_id: &'a Identifier,
    document_id: &'a Identifier,
    allow_original: bool,
    created_at: &'a Option<TimestampMillis>,
    updated_at: &'a Option<TimestampMillis>,
    data: &'a BTreeMap<String, Value>,
}

impl Drive {
    /// Validate that a document would be unique in the state
    pub fn validate_document_uniqueness(
        &self,
        contract: &Contract,
        document_type: &DocumentType,
        document: &Document,
        owner_id: &Identifier,
        allow_original: bool,
        transaction: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let request = UniquenessOfDataRequest {
            contract,
            document_type,
            owner_id,
            document_id: &document.id,
            allow_original,
            created_at: &document.created_at,
            updated_at: &document.updated_at,
            data: &document.properties,
        };
        self.validate_uniqueness_of_data(request, transaction)
    }

    /// Validate that a document create transition action would be unique in the state
    pub fn validate_document_create_transition_action_uniqueness(
        &self,
        contract: &Contract,
        document_type: &DocumentType,
        document_create_transition: &DocumentCreateTransitionAction,
        owner_id: &Identifier,
        transaction: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let request = UniquenessOfDataRequest {
            contract,
            document_type,
            owner_id,
            document_id: &document_create_transition.base.id,
            allow_original: false,
            created_at: &document_create_transition.created_at,
            updated_at: &document_create_transition.updated_at,
            data: &document_create_transition.data,
        };
        self.validate_uniqueness_of_data(request, transaction)
    }

    /// Validate that a document replace transition action would be unique in the state
    pub fn validate_document_replace_transition_action_uniqueness(
        &self,
        contract: &Contract,
        document_type: &DocumentType,
        document_replace_transition: &DocumentReplaceTransitionAction,
        owner_id: &Identifier,
        transaction: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let request = UniquenessOfDataRequest {
            contract,
            document_type,
            owner_id,
            document_id: &document_replace_transition.base.id,
            allow_original: true,
            created_at: &document_replace_transition.created_at,
            updated_at: &document_replace_transition.updated_at,
            data: &document_replace_transition.data,
        };
        self.validate_uniqueness_of_data(request, transaction)
    }

    /// Internal method validating uniqueness
    fn validate_uniqueness_of_data(
        &self,
        request: UniquenessOfDataRequest,
        transaction: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let UniquenessOfDataRequest {
            contract,
            document_type,
            owner_id,
            document_id,
            allow_original,
            created_at,
            updated_at,
            data,
        } = request;

        let validation_results = document_type
            .indices
            .iter()
            .filter_map(|index| {
                if !index.unique {
                    // if a index is not unique there is no issue
                    None
                } else {
                    let where_queries = index
                        .properties
                        .iter()
                        .filter_map(|property| {
                            let value = match property.name.as_str() {
                                "$ownerId" => {
                                    platform_value!(*owner_id)
                                }
                                "$createdAt" => {
                                    if let Some(created_at) = created_at {
                                        platform_value!(*created_at)
                                    } else {
                                        return None;
                                    }
                                }
                                "$updatedAt" => {
                                    if let Some(updated_at) = updated_at {
                                        platform_value!(*updated_at)
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

                    if where_queries.len() < index.properties.len() {
                        // there are empty fields, which means that the index is no longer unique
                        None
                    } else {
                        let query = DriveQuery {
                            contract: &contract,
                            document_type: &document_type,
                            internal_clauses: InternalClauses {
                                primary_key_in_clause: None,
                                primary_key_equal_clause: None,
                                in_clause: None,
                                range_clause: None,
                                equal_clauses: where_queries,
                            },
                            offset: 0,
                            limit: 0,
                            order_by: Default::default(),
                            start_at: None,
                            start_at_included: false,
                            block_time: None,
                        };

                        let query_result = self.query_documents(query, None, transaction);
                        match query_result {
                            Ok(query_outcome) => {
                                let documents = query_outcome.documents;
                                let would_be_unique = documents.is_empty()
                                    || (allow_original
                                        && documents.len() == 1
                                        && documents[0].id == document_id);
                                if would_be_unique {
                                    Some(Ok(SimpleConsensusValidationResult::default()))
                                } else {
                                    Some(Ok(SimpleConsensusValidationResult::new_with_error(
                                        StateError::DuplicateUniqueIndexError {
                                            document_id: *document_id,
                                            duplicating_properties: index.fields(),
                                        }
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
