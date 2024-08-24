use crate::error::Error;
use dpp::consensus::basic::document::InvalidDocumentTransitionIdError;
use dpp::consensus::signature::{InvalidSignaturePublicKeySecurityLevelError, SignatureError};
use dpp::document::Document;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::PartialIdentity;
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};

use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::{StateTransitionIdentitySigned, StateTransitionLike};

use dpp::validation::ConsensusValidationResult;

use dpp::version::PlatformVersion;

use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use drive::state_transition_action::document::documents_batch::DocumentsBatchTransitionAction;
use crate::execution::validation::state_transition::state_transitions::documents_batch::action_validation::document_replace_transition_action::DocumentReplaceTransitionActionValidation;
use crate::execution::validation::state_transition::state_transitions::documents_batch::action_validation::document_delete_transition_action::DocumentDeleteTransitionActionValidation;
use crate::execution::validation::state_transition::state_transitions::documents_batch::action_validation::document_create_transition_action::DocumentCreateTransitionActionValidation;
use dpp::state_transition::documents_batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use drive::state_transition_action::StateTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use crate::error::execution::ExecutionError;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::documents_batch::action_validation::document_purchase_transition_action::DocumentPurchaseTransitionActionValidation;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_transfer_transition_action::DocumentTransferTransitionActionValidation;
use crate::execution::validation::state_transition::documents_batch::action_validation::document_update_price_transition_action::DocumentUpdatePriceTransitionActionValidation;

pub(in crate::execution::validation::state_transition::state_transitions::documents_batch) trait DocumentsBatchStateTransitionStructureValidationV0
{
    fn validate_advanced_structure_from_state_v0(
        &self,
        action: &DocumentsBatchTransitionAction,
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DocumentsBatchStateTransitionStructureValidationV0 for DocumentsBatchTransition {
    fn validate_advanced_structure_from_state_v0(
        &self,
        action: &DocumentsBatchTransitionAction,
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let security_levels = action.contract_based_security_level_requirement()?;

        let signing_key = identity.loaded_public_keys.get(&self.signature_public_key_id()).ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution("the key must exist for advanced structure validation as we already fetched it during signature validation")))?;

        if !security_levels.contains(&signing_key.security_level()) {
            // We only need to bump the first identity data contract nonce as that will make a replay
            // attack not possible

            let first_transition = self.transitions().first().ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution("There must be at least one state transition as this is already verified in basic validation")))?;

            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition(
                    first_transition.base(),
                    self.owner_id(),
                    self.user_fee_increase(),
                ),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![SignatureError::InvalidSignaturePublicKeySecurityLevelError(
                    InvalidSignaturePublicKeySecurityLevelError::new(
                        signing_key.security_level(),
                        security_levels,
                    ),
                )
                .into()],
            ));
        }

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

                // This hash will take 2 blocks (128 bytes)
                execution_context.add_operation(ValidationOperation::DoubleSha256(2));

                let id = create_transition.base().id();
                if generated_document_id != id {
                    let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                        BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition(
                            transition.base(),
                            self.owner_id(),
                            self.user_fee_increase(),
                        ),
                    );

                    return Ok(ConsensusValidationResult::new_with_data_and_errors(
                        bump_action,
                        vec![
                            InvalidDocumentTransitionIdError::new(generated_document_id, id).into(),
                        ],
                    ));
                }
            }
        }

        // Next we need to validate the structure of all actions (this means with the data contract)
        for transition in action.transitions() {
            match transition {
                DocumentTransitionAction::CreateAction(create_action) => {
                    let result = create_action.validate_structure(identity.id, platform_version)?;
                    if !result.is_valid() {
                        let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                            BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(transition.base().expect("there is always a base for the create action"), self.owner_id(), self.user_fee_increase()),
                        );

                        return Ok(ConsensusValidationResult::new_with_data_and_errors(
                            bump_action,
                            result.errors,
                        ));
                    }
                }
                DocumentTransitionAction::ReplaceAction(replace_action) => {
                    let result = replace_action.validate_structure(platform_version)?;
                    if !result.is_valid() {
                        let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                            BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(transition.base().expect("there is always a base for the replace action"), self.owner_id(), self.user_fee_increase()),
                        );

                        return Ok(ConsensusValidationResult::new_with_data_and_errors(
                            bump_action,
                            result.errors,
                        ));
                    }
                }
                DocumentTransitionAction::DeleteAction(delete_action) => {
                    let result = delete_action.validate_structure(platform_version)?;
                    if !result.is_valid() {
                        let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                            BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(transition.base().expect("there is always a base for the delete action"), self.owner_id(), self.user_fee_increase()),
                        );

                        return Ok(ConsensusValidationResult::new_with_data_and_errors(
                            bump_action,
                            result.errors,
                        ));
                    }
                }
                DocumentTransitionAction::TransferAction(transfer_action) => {
                    let result = transfer_action.validate_structure(platform_version)?;
                    if !result.is_valid() {
                        let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                            BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(transition.base().expect("there is always a base for the transfer action"), self.owner_id(), self.user_fee_increase()),
                        );

                        return Ok(ConsensusValidationResult::new_with_data_and_errors(
                            bump_action,
                            result.errors,
                        ));
                    }
                }
                DocumentTransitionAction::UpdatePriceAction(update_price_action) => {
                    let result = update_price_action.validate_structure(platform_version)?;
                    if !result.is_valid() {
                        let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                            BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(transition.base().expect("there is always a base for the update price action"), self.owner_id(), self.user_fee_increase()),
                        );

                        return Ok(ConsensusValidationResult::new_with_data_and_errors(
                            bump_action,
                            result.errors,
                        ));
                    }
                }
                DocumentTransitionAction::PurchaseAction(purchase_action) => {
                    let result = purchase_action.validate_structure(platform_version)?;
                    if !result.is_valid() {
                        let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                            BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(transition.base().expect("there is always a base for the purchase action"), self.owner_id(), self.user_fee_increase()),
                        );

                        return Ok(ConsensusValidationResult::new_with_data_and_errors(
                            bump_action,
                            result.errors,
                        ));
                    }
                }
                DocumentTransitionAction::BumpIdentityDataContractNonce(_) => {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "we should not have a bump identity contract nonce at this stage",
                    )));
                }
            }
        }
        Ok(ConsensusValidationResult::new())
    }
}
