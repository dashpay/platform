use dpp::consensus::basic::BasicError;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use dpp::consensus::basic::document::{
    DataContractNotPresentError, MaxDocumentsTransitionsExceededError,
};
use dpp::consensus::ConsensusError;

use crate::error::Error;
use dpp::identifier::Identifier;

use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;

use dpp::validation::{SimpleConsensusValidationResult, ValidationResult};
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;

const MAX_TRANSITIONS_IN_BATCH: usize = 1000;

pub(in crate::execution::validation::state_transition::state_transitions::documents_batch) trait DocumentsBatchStateTransitionStructureValidationV0
{
    fn validate_structure_v0(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentsBatchStateTransitionStructureValidationV0 for DocumentsBatchTransition {
    fn validate_structure_v0(
        &self,
        drive: &Drive,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = SimpleConsensusValidationResult::default();

        if self.transitions().len() > MAX_TRANSITIONS_IN_BATCH {
            result.add_error::<ConsensusError>(
                MaxDocumentsTransitionsExceededError::new(MAX_TRANSITIONS_IN_BATCH as u32).into(),
            )
        }

        if !result.is_valid() {
            return Ok(result);
        }

        let mut document_transitions_by_contracts: BTreeMap<Identifier, Vec<&DocumentTransition>> =
            BTreeMap::new();

        self.transitions().iter().for_each(|document_transition| {
            let contract_identifier = document_transition.data_contract_id();

            match document_transitions_by_contracts.entry(contract_identifier) {
                Entry::Vacant(vacant) => {
                    vacant.insert(vec![document_transition]);
                }
                Entry::Occupied(mut identifiers) => {
                    identifiers.get_mut().push(document_transition);
                }
            };
        });

        let mut result = ValidationResult::default();

        for (data_contract_id, _transitions) in document_transitions_by_contracts {
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
                        data_contract_id
                    )));
                    return Ok(result);
                };

            let _existing_data_contract = &contract_fetch_info.contract;

            //todo document structure validation
            // let transitions_as_objects: Vec<Value> = transitions
            //     .into_iter()
            //     .map(|t| t.to_object().unwrap())
            //     .collect();
            // let validation_result = validate_document_transitions_basic(
            //     existing_data_contract,
            //     self.owner_id(),
            //     transitions_as_objects
            //         .iter()
            //         .map(|t| t.to_btree_ref_string_map().unwrap()),
            // )?;
            // result.merge(validation_result);
        }

        Ok(result)
    }
}
