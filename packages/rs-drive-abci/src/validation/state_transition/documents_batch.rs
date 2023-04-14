use std::collections::{btree_map::Entry, BTreeMap};

use dpp::document::document_transition::{DocumentTransitionExt, DocumentTransitionObjectLike};
use dpp::document::validation::basic::validate_documents_batch_transition_basic::DOCUMENTS_BATCH_TRANSITIONS_SCHEMA_VALIDATOR;
use dpp::identity::PartialIdentity;
use dpp::{
    consensus::basic::BasicError,
    document::{
        validation::basic::validate_documents_batch_transition_basic::validate_document_transitions as validate_document_transitions_basic,
        DocumentsBatchTransition,
    },
    platform_value::{Identifier, Value},
    prelude::DocumentTransition,
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext, StateTransitionAction,
    },
    validation::{ConsensusValidationResult, SimpleConsensusValidationResult, ValidationResult},
};

use drive::drive::Drive;
use drive::grovedb::TransactionArg;

use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::document_state_validation::validate_documents_batch_transition_state::validate_document_batch_transition_state;
use crate::validation::state_transition::key_validation::validate_state_transition_identity_signature;

use super::{
    common::{validate_protocol_version, validate_schema},
    StateTransitionValidation,
};

impl StateTransitionValidation for DocumentsBatchTransition {
    fn validate_structure(
        &self,
        drive: &Drive,
        tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(&DOCUMENTS_BATCH_TRANSITIONS_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_protocol_version(self.protocol_version);
        if !result.is_valid() {
            return Ok(result);
        }

        let mut document_transitions_by_contracts: BTreeMap<Identifier, Vec<&DocumentTransition>> =
            BTreeMap::new();

        self.get_transitions()
            .iter()
            .for_each(|document_transition| {
                let contract_identifier = document_transition.get_data_contract_id();

                match document_transitions_by_contracts.entry(contract_identifier.clone()) {
                    Entry::Vacant(vacant) => {
                        vacant.insert(vec![&document_transition]);
                    }
                    Entry::Occupied(mut identifiers) => {
                        identifiers.get_mut().push(&document_transition);
                    }
                };
            });

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
                self.owner_id,
                transitions_as_objects
                    .iter()
                    .map(|t| t.to_btree_ref_string_map().unwrap()),
            )?;
            result.merge(validation_result);
        }

        Ok(result)
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        Ok(
            validate_state_transition_identity_signature(drive, self, false, transaction)?
                .map(|partial_identity| Some(partial_identity)),
        )
    }

    fn validate_state<'a, C: CoreRPCLike>(
        &self,
        platform: &'a PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let validation_result = validate_document_batch_transition_state(
            &platform.into(),
            self,
            tx,
            &StateTransitionExecutionContext::default(),
        )?;
        Ok(validation_result.map(Into::into))
    }
}
