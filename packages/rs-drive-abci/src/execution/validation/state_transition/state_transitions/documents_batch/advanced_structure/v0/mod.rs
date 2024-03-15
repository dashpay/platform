use crate::error::Error;
use dpp::consensus::basic::document::InvalidDocumentTransitionIdError;
use dpp::document::Document;
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransition;

use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::StateTransitionLike;

use dpp::validation::SimpleConsensusValidationResult;

use dpp::version::PlatformVersion;

use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use drive::state_transition_action::document::documents_batch::DocumentsBatchTransitionAction;
use crate::execution::validation::state_transition::state_transitions::documents_batch::action_validation::document_replace_transition_action::DocumentReplaceTransitionActionValidation;
use crate::execution::validation::state_transition::state_transitions::documents_batch::action_validation::document_delete_transition_action::DocumentDeleteTransitionActionValidation;
use crate::execution::validation::state_transition::state_transitions::documents_batch::action_validation::document_create_transition_action::DocumentCreateTransitionActionValidation;
use dpp::state_transition::documents_batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use crate::error::execution::ExecutionError;

pub(in crate::execution::validation::state_transition::state_transitions::documents_batch) trait DocumentsBatchStateTransitionStructureValidationV0
{
    fn validate_advanced_structure_from_state_v0(
        &self,
        action: &DocumentsBatchTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentsBatchStateTransitionStructureValidationV0 for DocumentsBatchTransition {
    fn validate_advanced_structure_from_state_v0(
        &self,
        action: &DocumentsBatchTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // We should validate that all newly created documents have valid ids
        for transition in self.transitions() {
            if let DocumentTransition::Create(create_transition) = transition {
                // Validate the ID
                let generated_document_id = Document::generate_document_id_v0(
                    create_transition.base().data_contract_id_ref(),
                    &self.owner_id(),
                    create_transition.base().document_type_name(),
                    &create_transition.entropy(),
                );

                let id = create_transition.base().id();
                if generated_document_id != id {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidDocumentTransitionIdError::new(generated_document_id, id).into(),
                    ));
                }
            }
        }

        // Next we need to validate the structure of all actions (this means with the data contract)
        for transition in action.transitions() {
            match transition {
                DocumentTransitionAction::CreateAction(create_action) => {
                    let result = create_action.validate_structure(platform_version)?;
                    if !result.is_valid() {
                        return Ok(result);
                    }
                }
                DocumentTransitionAction::ReplaceAction(replace_action) => {
                    let result = replace_action.validate_structure(platform_version)?;
                    if !result.is_valid() {
                        return Ok(result);
                    }
                }
                DocumentTransitionAction::DeleteAction(delete_action) => {
                    let result = delete_action.validate_structure(platform_version)?;
                    if !result.is_valid() {
                        return Ok(result);
                    }
                }
                DocumentTransitionAction::BumpIdentityDataContractNonce(_) => {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "we should not have a bump identity contract nonce at this stage",
                    )));
                }
            }
        }
        Ok(SimpleConsensusValidationResult::new())
    }
}
