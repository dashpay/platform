use std::{
    collections::hash_map::{Entry, HashMap},
    convert::TryInto,
};
use std::collections::BTreeMap;

use dpp::document::{Document, ExtendedDocument};
use dpp::{get_from_transition, ProtocolError};
use dpp::platform_value::{Identifier, platform_value, Value};
use dpp::prelude::DocumentTransition;
use dpp::state_repository::StateRepositoryLike;
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use platform_value::platform_value;
use dpp::platform_value::string_encoding::Encoding;
use dpp::validation::ConsensusValidationResult;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::query::{DriveQuery, InternalClauses, StatelessDriveQuery, WhereClause, WhereOperator};
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

    let mut validation_results_of_documents = transitions_by_contracts_and_types.into_iter()
        .map(|((contract_id, document_type_name), transitions)| {
            let ids: Vec<Value> = dts
                .iter()
                .map(|dt| Value::Identifier(get_from_transition!(dt, id).to_buffer()))
                .collect();

            let stateless_drive_query = StatelessDriveQuery {
                contract_id,
                document_type_name,
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

            //todo: deal with fee_result (by putting in epoch)
            let (_, drive_query_fabrication_result) = DriveQuery::new_from_stateless_with_consensus_validation(stateless_drive_query, drive, None, true, transaction)?;

            if !drive_query_fabrication_result.is_valid() {
                return Ok(ConsensusValidationResult::new_with_errors(drive_query_fabrication_result.errors));
            }

            let drive_query = drive_query_fabrication_result.into_data()?;

            //todo: deal with cost of this operation
                let outcome = drive
                    .query_documents(
                        drive_query,
                        None,
                        Some(tx),
                    )?
                    .documents
                    .into_iter(),


    }).collect();

    for result in results.into_iter() {
        let result = result?;
        let documents_from_fetch: Vec<Document> = result
            .into_iter()
            .map(|d| d.try_into().map_err(Into::<ProtocolError>::into))
            .try_collect()?;
        documents.extend(documents_from_fetch)
    }

    Ok(documents)
}