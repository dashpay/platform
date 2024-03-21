use crate::consensus::basic::document::{
    DocumentTransitionsAreAbsentError, DuplicateDocumentTransitionsWithIdsError,
    IdentityContractNonceOutOfBoundsError, MaxDocumentsTransitionsExceededError,
};
use crate::consensus::basic::BasicError;

use crate::identity::identity_nonce::MISSING_IDENTITY_REVISIONS_FILTER;
use crate::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use crate::state_transition::documents_batch_transition::validation::find_duplicates_by_id::find_duplicates_by_id;
use crate::state_transition::documents_batch_transition::DocumentsBatchTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

const MAX_TRANSITIONS_IN_BATCH: usize = 1;

impl DocumentsBatchTransition {
    #[inline(always)]
    pub(super) fn validate_base_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if self.transitions().is_empty() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DocumentTransitionsAreAbsentError::new().into(),
            ));
        }

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

        let mut result = SimpleConsensusValidationResult::default();

        for transitions in document_transitions_by_contracts.values() {
            for transition in transitions {
                // We need to make sure that the identity contract nonce is within the allowed bounds
                // This means that it is stored on 40 bits
                if transition.identity_contract_nonce() & MISSING_IDENTITY_REVISIONS_FILTER > 0 {
                    result.add_error(BasicError::IdentityContractNonceOutOfBoundsError(
                        IdentityContractNonceOutOfBoundsError::new(
                            transition.identity_contract_nonce(),
                        ),
                    ));
                }
            }

            // Make sure we don't have duplicate transitions
            let duplicate_transitions = find_duplicates_by_id(transitions, platform_version)?;

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
        }

        Ok(result)
    }
}
