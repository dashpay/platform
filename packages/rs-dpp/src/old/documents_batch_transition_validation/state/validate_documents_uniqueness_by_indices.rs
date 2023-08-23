use std::convert::TryInto;

use futures::future::join_all;
use itertools::Itertools;
use platform_value::platform_value;
use platform_value::Value;
use serde_json::json;

use crate::consensus::state::document::duplicate_unique_index_error::DuplicateUniqueIndexError;
use crate::consensus::state::state_error::StateError;
use crate::document::Document;
use crate::validation::SimpleConsensusValidationResult;
use crate::{
    document::document_transition::{DocumentTransition, DocumentTransitionExt},
    prelude::{DataContract, Identifier},
    state_repository::StateRepositoryLike,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
    util::json_schema::{Index, JsonSchemaExt},
    ProtocolError,
};

struct QueryDefinition<'a> {
    document_type: &'a str,
    where_query: Vec<Value>,
    index_definition: &'a Index,
    document_transition: &'a DocumentTransition,
}

pub async fn validate_documents_uniqueness_by_indices<SR>(
    state_repository: &SR,
    owner_id: &Identifier,
    document_transitions: impl Iterator<Item = &DocumentTransition>,
    data_contract: &DataContract,
    execution_context: &StateTransitionExecutionContext,
) -> Result<SimpleConsensusValidationResult, ProtocolError>
where
    SR: StateRepositoryLike,
{
    let mut validation_result = SimpleConsensusValidationResult::default();

    if execution_context.is_dry_run() {
        return Ok(validation_result);
    }

    // 1. Prepare fetchDocuments queries from indexed properties
    for t in document_transitions {
        let transition = t;
        let document_schema =
            data_contract.get_document_schema(&transition.base().document_type_name)?;
        let document_indices = document_schema.get_indices::<Vec<_>>()?;
        if document_indices.is_empty() {
            continue;
        }

        // 2. Fetch Document by indexed properties
        let document_index_queries =
            generate_document_index_queries(&document_indices, transition, owner_id);
        let (futures, futures_meta): (Vec<_>, Vec<_>) = document_index_queries
            .filter(|query| !query.where_query.is_empty())
            .map(|query| {
                (
                    state_repository.fetch_documents(
                        &data_contract.id(),
                        query.document_type,
                        platform_value!( { "where": query.where_query}),
                        Some(execution_context),
                    ),
                    (query.index_definition, query.document_transition),
                )
            })
            .unzip();
        let results = join_all(futures).await;

        // 3. Create errors if duplicates found
        let result = validate_uniqueness(futures_meta, results)?;
        validation_result.merge(result);
    }

    Ok(validation_result)
}

fn generate_document_index_queries<'a>(
    indices: &'a [Index],
    transition: &'a DocumentTransition,
    owner_id: &'a Identifier,
) -> impl Iterator<Item = QueryDefinition<'a>> {
    indices
        .iter()
        .filter(|index| index.unique)
        .map(move |index| {
            let where_query = build_query_for_index_definition(index, transition, owner_id);
            QueryDefinition {
                document_type: &transition.base().document_type_name,
                index_definition: index,
                document_transition: transition,
                where_query,
            }
        })
}

fn build_query_for_index_definition(
    index_definition: &Index,
    transition: &DocumentTransition,
    owner_id: &Identifier,
) -> Vec<Value> {
    let mut query = vec![];
    for index_property in index_definition.properties.iter() {
        let property_name = &index_property.name;

        match property_name.as_str() {
            "$ownerId" => {
                let id = owner_id.to_buffer();
                query.push(platform_value!([property_name, "==", id]))
            }
            "$createdAt" => {
                if transition.base().action == Action::Create {
                    if let Some(transition_create) = transition.as_transition_create() {
                        if let Some(created_at) = transition_create.created_at.map(|v| json!(v)) {
                            query.push(platform_value!([property_name, "==", created_at]));
                        }
                    }
                }
            }
            "$updatedAt" => {
                if transition.base().action == Action::Create {
                    if let Some(updated_at) = transition.get_created_at().map(|v| json!(v)) {
                        query.push(platform_value!([property_name, "==", updated_at]))
                    }
                }
            }

            _ => {
                if let Some(value) = transition.get_dynamic_property(property_name) {
                    match value {
                        Value::Identifier(internal) => {
                            query.push(platform_value!([property_name, "==", internal]))
                        }
                        _ => query.push(platform_value!([property_name, "==", value])),
                    }
                }
            }
        }
    }
    query
}

fn validate_uniqueness<'a>(
    futures_meta: Vec<(&'a Index, &'a DocumentTransition)>,
    results: Vec<
        Result<Vec<impl TryInto<Document, Error = impl Into<ProtocolError>>>, anyhow::Error>,
    >,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    let mut validation_result = SimpleConsensusValidationResult::default();
    for (i, result) in results.into_iter().enumerate() {
        let documents: Vec<Document> = result?
            .into_iter()
            .map(|d| d.try_into().map_err(Into::<ProtocolError>::into))
            .try_collect()?;

        let only_origin_document =
            documents.len() == 1 && documents[0].id == futures_meta[i].1.base().id;
        if documents.is_empty() || only_origin_document {
            continue;
        }

        validation_result.add_error(StateError::DuplicateUniqueIndexError(
            DuplicateUniqueIndexError::new(
                futures_meta[i].1.base().id,
                futures_meta[i]
                    .0
                    .properties
                    .iter()
                    .map(|property| property.name.to_owned())
                    .collect_vec(),
            ),
        ))
    }
    Ok(validation_result)
}
