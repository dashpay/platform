use std::convert::TryInto;

use dpp::document::Document;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::{
    document::document_transition::{Action, DocumentTransition, DocumentTransitionExt},
    prelude::{DataContract, Identifier},
    state_repository::StateRepositoryLike,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
    ProtocolError, StateError,
};
use dpp::data_contract::document_type::Index;
use dpp::data_contract::DriveContractExt;
use dpp::platform_value::{platform_value, Value};
use drive::drive::Drive;
use drive::query::DriveQuery;

struct QueryDefinition<'a> {
    query: DriveQuery<'a>,
    index_definition: &'a Index,
    document_transition: &'a DocumentTransition,
}

pub fn validate_documents_uniqueness_by_indices(
    drive: &Drive,
    owner_id: &Identifier,
    document_transitions: impl Iterator<Item = &DocumentTransition>,
    data_contract: &DataContract,
    execution_context: &StateTransitionExecutionContext,
) -> Result<SimpleConsensusValidationResult, ProtocolError>
{
    let mut validation_result = SimpleConsensusValidationResult::default();

    if execution_context.is_dry_run() {
        return Ok(validation_result);
    }

    // 1. Prepare fetchDocuments queries from indexed properties
    for document_transition in document_transitions {

        let document_type = data_contract.document_type_for_name(&document_transition.base().document_type_name)?;
        let document_indices = document_type.indices.as_slice();

        // 2. Generate queries to search for duplicates
        let document_index_queries =
            generate_document_index_queries(document_indices, document_transition, owner_id);
        let (results, futures_meta) : (Vec<_>, Vec<_>) = document_index_queries
            .filter(|query| !query.where_query.is_empty())
            .map(|query| {
                (
                    drive.query_documents(
                        &data_contract.id,
                        query.document_type,
                        platform_value!( { "where": query.where_query}),
                        Some(execution_context),
                    ),
                    (query.index_definition, query.document_transition),
                )
            }).unzip();

        // 3. Create errors if duplicates found
        let result = validate_uniqueness(futures_meta, results)?;
        validation_result.merge(result);
    }

    Ok(validation_result)
}

