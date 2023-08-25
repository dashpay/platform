use crate::consensus::basic::document::{
    DataContractNotPresentError, DuplicateDocumentTransitionsWithIdsError,
    MaxDocumentsTransitionsExceededError,
};
use crate::consensus::basic::BasicError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::DataContract;
use crate::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use crate::state_transition::documents_batch_transition::validation::find_duplicates_by_id::find_duplicates_by_id;
use crate::state_transition::documents_batch_transition::DocumentsBatchTransition;
use crate::state_transition::StateTransitionLike;
use crate::validation::{SimpleConsensusValidationResult, ValidationResult};
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

const MAX_TRANSITIONS_IN_BATCH: usize = 100;

impl DocumentsBatchTransition {
    pub(super) fn validate_v0<'d>(
        &self,
        get_data_contract: impl Fn(Identifier) -> Result<Option<&'d DataContract>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if self.transitions().len() > MAX_TRANSITIONS_IN_BATCH {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                MaxDocumentsTransitionsExceededError::new(MAX_TRANSITIONS_IN_BATCH as u32).into(),
            ));
        }

        // Group transitions by contract ID
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

        for (data_contract_id, transitions) in document_transitions_by_contracts {
            // Get data contracts by IDs

            // We will be adding to block cache, contracts that are pulled
            // This block cache only gets merged to the main cache if the block is finalized
            let Some(data_contract) =
                get_data_contract(data_contract_id)?
                else {
                    result.add_error(BasicError::DataContractNotPresentError(DataContractNotPresentError::new(
                        data_contract_id
                    )));
                    return Ok(result);
                };

            // Make sure we don't have duplicate transitions
            let duplicate_transitions = find_duplicates_by_id(&transitions, platform_version)?;

            if !duplicate_transitions.is_empty() {
                let references: Vec<(String, [u8; 32])> = duplicate_transitions
                    .into_iter()
                    .map(|transition| {
                        Ok((
                            transition.base().document_type_name().clone(),
                            transition.base().id().to_buffer(),
                        ))
                    })
                    .collect::<Result<Vec<(String, [u8; 32])>, anyhow::Error>>()?;

                result.add_error(BasicError::DuplicateDocumentTransitionsWithIdsError(
                    DuplicateDocumentTransitionsWithIdsError::new(references),
                ));
            }

            // Validate transitions data
            for transition in transitions {
                let result =
                    transition.validate(data_contract, self.owner_id(), platform_version)?;

                if !result.is_valid() {
                    return Ok(result);
                }
            }
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
