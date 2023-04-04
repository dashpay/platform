use std::collections::{hash_map::Entry, HashMap};

use dpp::{
    consensus::{basic::BasicError, ConsensusError},
    data_contract::{errors::DataContractNotPresentError, DataContract, DriveContractExt},
    document::{
        document_transition::{
            DocumentCreateTransitionAction, DocumentDeleteTransitionAction,
            DocumentReplaceTransitionAction, DocumentTransitionAction, DocumentTransitionExt,
            DocumentTransitionObjectLike,
        },
        validation::{
            basic::validate_documents_batch_transition_basic::{
                validate_document_transitions as validate_document_transitions_basic,
                DOCUMENTS_BATCH_TRANSITIONS_SCHEMA,
            },
            state::validate_documents_batch_transition_state::{
                check_created_inside_time_window, check_if_document_can_be_found,
                check_if_document_is_already_present, check_if_timestamps_are_equal,
                check_ownership, check_revision, check_updated_inside_time_window,
            },
        },
        Document, DocumentsBatchTransition,
    },
    get_from_transition,
    platform_value::{platform_value, string_encoding::Encoding, Identifier, Value},
    prelude::DocumentTransition,
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext, StateTransitionAction,
    },
    validation::{ConsensusValidationResult, SimpleConsensusValidationResult, ValidationResult},
    ProtocolError,
};
use drive::{
    grovedb::Transaction,
    query::{DriveQuery, InternalClauses, WhereClause, WhereOperator},
};

use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::error::Error;

use super::{
    common::{validate_protocol_version, validate_schema},
    StateTransitionValidation,
};

impl StateTransitionValidation for DocumentsBatchTransition {
    fn validate_type(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(DOCUMENTS_BATCH_TRANSITIONS_SCHEMA.clone(), self);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_protocol_version(self.protocol_version);
        if !result.is_valid() {
            return Ok(result);
        }

        let mut document_transitions_by_contracts: HashMap<Identifier, Vec<&DocumentTransition>> =
            HashMap::new();

        for document_transition in self.get_transitions() {
            let contract_identifier = document_transition.get_data_contract_id();

            match document_transitions_by_contracts.entry(contract_identifier.clone()) {
                Entry::Vacant(vacant) => {
                    vacant.insert(vec![&document_transition]);
                }
                Entry::Occupied(mut identifiers) => {
                    identifiers.get_mut().push(&document_transition);
                }
            };
        }

        let mut result = ValidationResult::default();

        for (data_contract_id, transitions) in document_transitions_by_contracts {
            // We will be adding to block cache, contracts that are pulled
            // This block cache only gets merged to the main cache if the block is finalized
            let Some(contract_fetch_info) =
                drive
                .get_contract_with_fetch_info(data_contract_id.0.0, None, true, tx)?
                .1
            else {
                result.add_error(BasicError::DataContractNotPresent {
                    data_contract_id: data_contract_id.0.0.into()
                });
                return Ok(result);
            };

            let existing_data_contract = &contract_fetch_info.contract;

            let transitions_as_objects: Vec<Value> = transitions
                .into_iter()
                .map(|t| t.to_object().unwrap())
                .collect();
            let validation_result = validate_document_transitions_basic(
                &existing_data_contract,
                existing_data_contract.owner_id,
                transitions_as_objects
                    .iter()
                    .map(|t| t.to_btree_ref_string_map().unwrap()),
            )?;
            result.merge(validation_result);
        }

        Ok(result)
    }

    fn validate_signature(&self, drive: &Drive) -> Result<SimpleConsensusValidationResult, Error> {
        todo!()
    }

    fn validate_key_signature(&self) -> Result<SimpleConsensusValidationResult, Error> {
        todo!()
    }

    fn validate_state(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        todo!()
    }
}

// fn validate_document_transitions(
//     drive: &Drive,
//     tx: &Transaction,
//     data_contract_id: &Identifier,
//     owner_id: Identifier,
//     document_transitions: &[&DocumentTransition],
//     execution_context: &StateTransitionExecutionContext,
// ) -> Result<ConsensusValidationResult<Vec<DocumentTransitionAction>>, Error> {
//     let mut result = ConsensusValidationResult::<Vec<DocumentTransitionAction>>::default();

//     let tmp_execution_context = StateTransitionExecutionContext::default();

//     // Data Contract must exist
//     let Some(contract_fetch_info) =
//             drive
//             .get_contract_with_fetch_info(data_contract_id.0 .0, None, false, Some(tx))?
//             .1
//     else {
//         return Err(ProtocolError::DataContractNotPresentError(
//             DataContractNotPresentError::new(data_contract_id.clone())).into())
//     };

//     let data_contract = &contract_fetch_info.contract;

//     execution_context.add_operations(tmp_execution_context.get_operations());

//     let fetched_documents = fetch_documents(
//         drive,
//         tx,
//         document_transitions,
//         execution_context,
//         data_contract,
//     )?;

//     // Calculate time window for timestamp
//     let last_header_time_millis =  state_repository.fetch_latest_platform_block_time().await?;

//     let document_transition_actions = if !execution_context.is_dry_run() {
//         let document_transition_actions = document_transitions
//             .iter()
//             .filter_map(|transition| {
//                 let validation_result = validate_transition(
//                     transition,
//                     &fetched_documents,
//                     last_header_time_millis,
//                     &owner_id,
//                 );
//                 match validation_result {
//                     Ok(validation_result) => {
//                         if validation_result.has_data() && validation_result.is_valid() {
//                             Some(validation_result.into_data())
//                         } else {
//                             result.add_errors(validation_result.errors);
//                             None
//                         }
//                     }
//                     Err(protocol_error) => Some(Err(protocol_error)),
//                 }
//             })
//             .collect::<Result<Vec<DocumentTransitionAction>, ProtocolError>>()?;
//         if !result.is_valid() {
//             return Ok(result);
//         }
//         document_transition_actions
//     } else {
//         vec![]
//     };

//     let validation_uniqueness_by_indices_result = validate_documents_uniqueness_by_indices(
//         state_repository,
//         &owner_id,
//         document_transitions
//             .iter()
//             .filter(|d| d.as_transition_delete().is_none())
//             .cloned(),
//         &data_contract,
//         execution_context,
//     )
//     .await?;

//     if !validation_uniqueness_by_indices_result.is_valid() {
//         result.add_errors(validation_uniqueness_by_indices_result.errors)
//     }

//     let data_trigger_execution_context = DataTriggerExecutionContext {
//         state_repository: state_repository.to_owned(),
//         owner_id: &owner_id,
//         data_contract: &data_contract,
//         state_transition_execution_context: execution_context,
//     };
//     let data_trigger_execution_results =
//         execute_data_triggers(document_transitions, &data_trigger_execution_context).await?;

//     for execution_result in data_trigger_execution_results.into_iter() {
//         if !execution_result.is_ok() {
//             result.add_errors(
//                 execution_result
//                     .errors
//                     .into_iter()
//                     .map(ConsensusError::from)
//                     .collect(),
//             )
//         }
//     }

//     if !result.is_valid() {
//         Ok(result)
//     } else {
//         Ok(document_transition_actions.into())
//     }
// }

fn validate_transition(
    transition: &DocumentTransition,
    fetched_documents: &[Document],
    last_header_block_time_millis: u64,
    owner_id: &Identifier,
) -> Result<ConsensusValidationResult<DocumentTransitionAction>, ProtocolError> {
    match transition {
        DocumentTransition::Create(document_create_transition) => {
            let mut result = ConsensusValidationResult::<DocumentTransitionAction>::new_with_data(
                DocumentTransitionAction::CreateAction(DocumentCreateTransitionAction::default()),
            );
            let validation_result = check_if_timestamps_are_equal(transition);
            result.merge(validation_result);

            let validation_result =
                check_created_inside_time_window(transition, last_header_block_time_millis);
            result.merge(validation_result);

            let validation_result =
                check_updated_inside_time_window(transition, last_header_block_time_millis);
            result.merge(validation_result);

            let validation_result =
                check_if_document_is_already_present(transition, fetched_documents);
            result.merge(validation_result);

            if result.is_valid() {
                Ok(
                    DocumentTransitionAction::CreateAction(document_create_transition.into())
                        .into(),
                )
            } else {
                Ok(result)
            }
        }
        DocumentTransition::Replace(document_replace_transition) => {
            let mut result = ConsensusValidationResult::<DocumentTransitionAction>::new_with_data(
                DocumentTransitionAction::ReplaceAction(DocumentReplaceTransitionAction::default()),
            );
            let validation_result =
                check_updated_inside_time_window(transition, last_header_block_time_millis);
            result.merge(validation_result);

            let validation_result = check_revision(transition, fetched_documents);
            result.merge(validation_result);

            let validation_result = check_if_document_can_be_found(transition, fetched_documents);
            let original_document = if validation_result.has_data() {
                validation_result.into_data()?
            } else {
                result.add_errors(validation_result.errors);
                return Ok(result);
            };

            let validation_result = check_ownership(transition, original_document, owner_id);
            result.merge(validation_result);

            if result.is_valid() {
                Ok(DocumentTransitionAction::ReplaceAction(
                    DocumentReplaceTransitionAction::from_document_replace_transition(
                        document_replace_transition,
                        original_document.created_at,
                    ),
                )
                .into())
            } else {
                Ok(result)
            }
        }
        DocumentTransition::Delete(document_delete_transition) => {
            let mut result = ConsensusValidationResult::<DocumentTransitionAction>::new_with_data(
                DocumentTransitionAction::DeleteAction(DocumentDeleteTransitionAction::default()),
            );
            let validation_result = check_if_document_can_be_found(transition, fetched_documents);
            let original_document = if validation_result.has_data() {
                validation_result.into_data()?
            } else {
                result.add_errors(validation_result.errors);
                return Ok(result);
            };

            let validation_result = check_ownership(transition, original_document, owner_id);
            if !validation_result.is_valid() {
                result.add_errors(validation_result.errors);
            }

            if result.is_valid() {
                Ok(
                    DocumentTransitionAction::DeleteAction(document_delete_transition.into())
                        .into(),
                )
            } else {
                Ok(result)
            }
        }
    }
}

fn fetch_documents<'a>(
    drive: &'a Drive,
    tx: &Transaction<'a>,
    document_transitions: &[&DocumentTransition],
    execution_context: &StateTransitionExecutionContext,
    data_contract: &DataContract,
) -> Result<Vec<Document>, Error> {
    let mut transitions_by_contracts_and_types: HashMap<String, Vec<&DocumentTransition>> =
        HashMap::new();

    for document_transition in document_transitions {
        let document_type = get_from_transition!(document_transition, document_type_name);
        let data_contract_id = get_from_transition!(document_transition, data_contract_id);
        let unique_key = format!("{}{}", data_contract_id, document_type);

        match transitions_by_contracts_and_types.entry(unique_key) {
            Entry::Vacant(v) => {
                v.insert(vec![document_transition]);
            }
            Entry::Occupied(mut o) => o.get_mut().push(document_transition),
        }
    }

    let mut documents = vec![];
    for (_, dts) in transitions_by_contracts_and_types {
        let ids: Vec<Value> = dts
            .iter()
            .map(|dt| Value::Text(get_from_transition!(dt, id).to_string(Encoding::Base58)))
            .collect();

        documents.extend(
            drive
                .query_documents(
                    DriveQuery {
                        contract: data_contract,
                        document_type: data_contract.document_type_for_name(
                            get_from_transition!(dts[0], document_type_name),
                        )?,
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
                    },
                    None,
                    Some(tx),
                )?
                .documents
                .into_iter(),
        );
    }

    Ok(documents)
}
