use dpp::consensus::basic::BasicError;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use dpp::consensus::basic::document::DataContractNotPresentError;

use crate::error::Error;
use crate::execution::validation::state_transition::common::validate_protocol_version::v0::validate_protocol_version_v0;
use crate::execution::validation::state_transition::common::validate_schema::v0::validate_schema_v0;
use crate::execution::validation::state_transition::documents_batch::validate_document_transitions_basic;
use dpp::document::document_transition::{DocumentTransitionExt, DocumentTransitionObjectLike};
use dpp::document::validation::basic::validate_documents_batch_transition_basic::DOCUMENTS_BATCH_TRANSITIONS_SCHEMA_VALIDATOR;
use dpp::document::DocumentsBatchTransition;
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::prelude::DocumentTransition;
use dpp::state_transition::documents_batch_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::validation::{SimpleConsensusValidationResult, ValidationResult};
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

pub(crate) trait StateTransitionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl StateTransitionStructureValidationV0 for DocumentsBatchTransition {
    fn validate_structure_v0(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema_v0(&DOCUMENTS_BATCH_TRANSITIONS_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_protocol_version_v0(self.protocol_version);
        if !result.is_valid() {
            return Ok(result);
        }

        let mut document_transitions_by_contracts: BTreeMap<Identifier, Vec<&DocumentTransition>> =
            BTreeMap::new();

        self.get_transitions()
            .iter()
            .for_each(|document_transition| {
                let contract_identifier = document_transition.get_data_contract_id();

                match document_transitions_by_contracts.entry(*contract_identifier) {
                    Entry::Vacant(vacant) => {
                        vacant.insert(vec![document_transition]);
                    }
                    Entry::Occupied(mut identifiers) => {
                        identifiers.get_mut().push(document_transition);
                    }
                };
            });

        let mut result = ValidationResult::default();

        for (data_contract_id, transitions) in document_transitions_by_contracts {
            // We will be adding to block cache, contracts that are pulled
            // This block cache only gets merged to the main cache if the block is finalized
            let Some(contract_fetch_info) =
                drive
                    .get_contract_with_fetch_info_and_fee(
                        data_contract_id.0.0,
                        None,
                        true,
                        tx,
                        platform_version,
                    )?
                    .1
                else {
                    result.add_error(BasicError::DataContractNotPresentError(DataContractNotPresentError::new(
                        data_contract_id.0.0.into()
                    )));
                    return Ok(result);
                };

            let existing_data_contract = &contract_fetch_info.contract;

            let transitions_as_objects: Vec<Value> = transitions
                .into_iter()
                .map(|t| t.to_object().unwrap())
                .collect();
            let validation_result = validate_document_transitions_basic(
                existing_data_contract,
                self.owner_id,
                transitions_as_objects
                    .iter()
                    .map(|t| t.to_btree_ref_string_map().unwrap()),
            )?;
            result.merge(validation_result);
        }

        Ok(result)
    }
}
