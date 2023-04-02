use std::collections::{hash_map::Entry, HashMap};

use dpp::{
    consensus::basic::BasicError,
    document::{
        document_transition::{DocumentTransitionExt, DocumentTransitionObjectLike},
        validation::basic::validate_documents_batch_transition_basic::{
            validate_document_transitions, DOCUMENTS_BATCH_TRANSITIONS_SCHEMA,
        },
        DocumentsBatchTransition,
    },
    platform_value::{Identifier, Value},
    prelude::DocumentTransition,
    state_transition::StateTransitionAction,
    validation::{ConsensusValidationResult, SimpleConsensusValidationResult, ValidationResult},
};
use drive::{drive::Drive, grovedb::Transaction};

use crate::error::Error;

use super::{
    common::{validate_protocol_version, validate_schema},
    StateTransitionValidation,
};

impl StateTransitionValidation for DocumentsBatchTransition {
    fn validate_type(
        &self,
        drive: &Drive,
        tx: &Transaction,
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
            let Some(contract_fetch_info) =
                drive
                .get_contract_with_fetch_info(data_contract_id.0.0, None, Some(tx))?
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
            let validation_result = validate_document_transitions(
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
        tx: &Transaction,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        todo!()
    }
}
