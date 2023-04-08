use std::{
    collections::hash_map::{Entry, HashMap},
    convert::TryInto,
};
use std::collections::BTreeMap;

use dpp::document::{Document, ExtendedDocument};
use dpp::{get_from_transition, ProtocolError};
use dpp::block_time_window::validation_result;
use dpp::consensus::basic::BasicError;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::data_contract::DriveContractExt;
use dpp::platform_value::{Identifier, platform_value, Value};
use dpp::prelude::DocumentTransition;
use dpp::state_repository::StateRepositoryLike;
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::platform_value::string_encoding::Encoding;
use dpp::validation::ConsensusValidationResult;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::query::{DriveQuery, InternalClauses, WhereClause, WhereOperator};
use crate::error::Error;

pub fn fetch_documents(
    drive: &Drive,
    document_transitions: &[&DocumentTransition],
    transaction: TransactionArg
) -> Result<ConsensusValidationResult<Vec<Document>>, Error> {
    let mut transitions_by_contracts_and_types: BTreeMap<(&Identifier, &String), Vec<&DocumentTransition>> =
        BTreeMap::new();

    for document_transition in document_transitions {
        let document_type = get_from_transition!(document_transition, document_type_name);
        let data_contract_id = get_from_transition!(document_transition, data_contract_id);

        match transitions_by_contracts_and_types.entry((data_contract_id, document_type)) {
            Entry::Vacant(v) => {
                v.insert(vec![document_transition]);
            }
            Entry::Occupied(mut o) => o.get_mut().push(document_transition),
        }
    }

    let validation_results_of_documents = transitions_by_contracts_and_types.into_iter()
        .map(|((contract_id, document_type_name), transitions)| {
            let ids: Vec<Value> = transitions
                .iter()
                .map(|dt| Value::Identifier(get_from_transition!(dt, id).to_buffer()))
                .collect();

            //todo: deal with fee result
            let (_, contract_fetch_info) = drive.get_contract_with_fetch_info(
                contract_id.to_buffer(),
                epoch,
                add_to_cache_if_pulled,
                transaction,
            )?;

            let Some(contract_fetch_info) = contract_fetch_info else {
                return Ok(ConsensusValidationResult::new_with_error(BasicError::DataContractNotPresent { data_contract_id: contract_id.clone()}.into()));
            };

            let contract_fetch_info = contract_fetch_info.clone();

            let Some(document_type) = contract_fetch_info.contract.optional_document_type_for_name(document_type_name) else {
                return Ok(ConsensusValidationResult::new_with_error(BasicError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(document_type_name.to_string(), *contract_id)).into()));
            };


            let drive_query = DriveQuery {
                contract: &contract_fetch_info.contract,
                document_type,
                internal_clauses: InternalClauses {
                    primary_key_in_clause: Some(WhereClause {
                        field: "$id".to_string(),
                        operator: WhereOperator::In,
                        value: Value::Array(ids),
                    }),
                    primary_key_equal_clause: None,
                    in_clause: None,
                    range_clause: None,
                    equal_clauses: Default::default(),
                },
                offset: 0,
                limit: 0,
                order_by: Default::default(),
                start_at: None,
                start_at_included: false,
                block_time: None,
            };

            //todo: deal with cost of this operation
                let documents = drive
                    .query_documents(
                        drive_query,
                        None,
                        Some(tx),
                    )?
                    .documents;

            Ok(ConsensusValidationResult::new_with_data(documents))
    }).collect::<Result<Vec<ConsensusValidationResult<Vec<Document>>>, Error>>()?;

    let validation_result = ConsensusValidationResult::flatten(validation_results_of_documents);

    Ok(validation_result)
}