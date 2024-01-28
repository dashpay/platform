use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use crate::error::Error;
use crate::platform_types::platform::PlatformStateRef;
use dpp::consensus::basic::document::{DataContractNotPresentError, InvalidDocumentTypeError};
use dpp::consensus::basic::BasicError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;

use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use dpp::document::Document;
use dpp::platform_value::{Identifier, Value};
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::query::{DriveQuery, InternalClauses, WhereClause, WhereOperator};

#[allow(dead_code)]
pub(crate) fn fetch_documents_for_transitions(
    platform: &PlatformStateRef,
    document_transitions: &[&DocumentTransition],
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Vec<Document>>, Error> {
    let mut transitions_by_contracts_and_types: BTreeMap<
        (&Identifier, &String),
        Vec<&DocumentTransition>,
    > = BTreeMap::new();

    for document_transition in document_transitions {
        let document_type = document_transition.base().document_type_name();
        let data_contract_id = document_transition.base().data_contract_id_ref();

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
                platform_version,
            )
        })
        .collect::<Result<Vec<ConsensusValidationResult<Vec<Document>>>, Error>>()?;

    let validation_result = ConsensusValidationResult::flatten(validation_results_of_documents);

    Ok(validation_result)
}

#[allow(dead_code)]
pub(crate) fn fetch_documents_for_transitions_knowing_contract_id_and_document_type_name(
    platform: &PlatformStateRef,
    contract_id: &Identifier,
    document_type_name: &str,
    transitions: &[&DocumentTransition],
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Vec<Document>>, Error> {
    let drive = platform.drive;
    //todo: deal with fee result
    //we only want to add to the cache if we are validating in a transaction
    let add_to_cache_if_pulled = transaction.is_some();
    let (_, contract_fetch_info) = drive.get_contract_with_fetch_info_and_fee(
        contract_id.to_buffer(),
        Some(&platform.state.last_committed_block_epoch()),
        add_to_cache_if_pulled,
        transaction,
        platform_version,
    )?;

    let Some(contract_fetch_info) = contract_fetch_info else {
        return Ok(ConsensusValidationResult::new_with_error(
            BasicError::DataContractNotPresentError(DataContractNotPresentError::new(*contract_id))
                .into(),
        ));
    };

    let Some(document_type) = contract_fetch_info
        .contract
        .document_type_optional_for_name(document_type_name)
    else {
        return Ok(ConsensusValidationResult::new_with_error(
            BasicError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(
                document_type_name.to_string(),
                *contract_id,
            ))
            .into(),
        ));
    };
    fetch_documents_for_transitions_knowing_contract_and_document_type(
        drive,
        &contract_fetch_info.contract,
        document_type,
        transitions,
        transaction,
        platform_version,
    )
}

pub(crate) fn fetch_documents_for_transitions_knowing_contract_and_document_type(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    transitions: &[&DocumentTransition],
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Vec<Document>>, Error> {
    if transitions.is_empty() {
        return Ok(ConsensusValidationResult::new_with_data(vec![]));
    }

    let ids: Vec<Value> = transitions
        .iter()
        .map(|dt| Value::Identifier(dt.get_id().to_buffer()))
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
        offset: None,
        limit: Some(transitions.len() as u16),
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
    };

    //todo: deal with cost of this operation
    let documents_outcome = drive.query_documents(
        drive_query,
        None,
        false,
        transaction,
        Some(platform_version.protocol_version),
    )?;

    Ok(ConsensusValidationResult::new_with_data(
        documents_outcome.documents_owned(),
    ))
}

pub(crate) fn fetch_document_with_id(
    drive: &Drive,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    id: Identifier,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Option<Document>, Error> {
    let drive_query = DriveQuery {
        contract,
        document_type,
        internal_clauses: InternalClauses {
            primary_key_in_clause: None,
            primary_key_equal_clause: Some(WhereClause {
                field: "$id".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(id.to_buffer()),
            }),
            in_clause: None,
            range_clause: None,
            equal_clauses: Default::default(),
        },
        offset: None,
        limit: Some(1),
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
    };

    //todo: deal with cost of this operation
    let documents_outcome = drive.query_documents(
        drive_query,
        None,
        false,
        transaction,
        Some(platform_version.protocol_version),
    )?;

    let mut documents = documents_outcome.documents_owned();

    if documents.is_empty() {
        Ok(None)
    } else {
        Ok(Some(documents.remove(0)))
    }
}
