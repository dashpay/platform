use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use crate::error::Error;
use crate::platform::PlatformStateRef;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::basic::BasicError;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::DriveContractExt;
use dpp::document::Document;
use dpp::get_from_transition;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DocumentTransition;
use dpp::validation::ConsensusValidationResult;
use drive::contract::Contract;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::query::{DriveQuery, InternalClauses, WhereClause, WhereOperator};

pub(super) fn fetch_documents_for_transitions(
    platform: &PlatformStateRef,
    document_transitions: &[&DocumentTransition],
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<Vec<Document>>, Error> {
    let mut transitions_by_contracts_and_types: BTreeMap<
        (&Identifier, &String),
        Vec<&DocumentTransition>,
    > = BTreeMap::new();

    for document_transition in document_transitions {
        let document_type = &document_transition.base().document_type_name;
        let data_contract_id = &document_transition.base().data_contract_id;

        match transitions_by_contracts_and_types.entry((data_contract_id, document_type)) {
            Entry::Vacant(v) => {
                v.insert(vec![document_transition]);
            }
            Entry::Occupied(mut o) => o.get_mut().push(document_transition),
        }
    }

    let validation_results_of_documents = transitions_by_contracts_and_types
        .into_iter()
        .map(|((contract_id, document_type_name), transitions)| {
            fetch_documents_for_transitions_knowing_contract_id_and_document_type_name(
                platform,
                contract_id,
                document_type_name,
                transitions.as_slice(),
                transaction,
            )
        })
        .collect::<Result<Vec<ConsensusValidationResult<Vec<Document>>>, Error>>()?;

    let validation_result = ConsensusValidationResult::flatten(validation_results_of_documents);

    Ok(validation_result)
}

pub(super) fn fetch_documents_for_transitions_knowing_contract_id_and_document_type_name(
    platform: &PlatformStateRef,
    contract_id: &Identifier,
    document_type_name: &str,
    transitions: &[&DocumentTransition],
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<Vec<Document>>, Error> {
    let drive = platform.drive;
    //todo: deal with fee result
    //we only want to add to the cache if we are validating in a transaction
    let add_to_cache_if_pulled = transaction.is_some();
    let (_, contract_fetch_info) = drive.get_contract_with_fetch_info(
        contract_id.to_buffer(),
        Some(&platform.state.epoch()),
        add_to_cache_if_pulled,
        transaction,
    )?;

    let Some(contract_fetch_info) = contract_fetch_info else {
        return Ok(ConsensusValidationResult::new_with_error(BasicError::DataContractNotPresent { data_contract_id: *contract_id}.into()));
    };

    let contract_fetch_info = contract_fetch_info;

    let Some(document_type) = contract_fetch_info.contract.optional_document_type_for_name(document_type_name) else {
        return Ok(ConsensusValidationResult::new_with_error(BasicError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(document_type_name.to_string(), *contract_id)).into()));
    };
    fetch_documents_for_transitions_knowing_contract_and_document_type(
        drive,
        &contract_fetch_info.contract,
        document_type,
        transitions,
        transaction,
    )
}

pub(super) fn fetch_documents_for_transitions_knowing_contract_and_document_type(
    drive: &Drive,
    contract: &Contract,
    document_type: &DocumentType,
    transitions: &[&DocumentTransition],
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<Vec<Document>>, Error> {
    let ids: Vec<Value> = transitions
        .iter()
        .map(|dt| Value::Identifier(get_from_transition!(dt, id).to_buffer()))
        .collect();

    let drive_query = DriveQuery {
        contract,
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
        limit: transitions.len() as u16,
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time: None,
    };

    //todo: deal with cost of this operation
    let documents = drive
        .query_documents(drive_query, None, false, transaction)?
        .documents;

    Ok(ConsensusValidationResult::new_with_data(documents))
}
