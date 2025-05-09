use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::InvalidDocumentTransitionIdError;
use dpp::consensus::signature::{InvalidSignaturePublicKeySecurityLevelError, SignatureError};
use dpp::dashcore::Network;
use dpp::document::Document;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::PartialIdentity;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::{StateTransitionIdentitySigned, StateTransitionLike};
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::validation::ConsensusValidationResult;

use dpp::version::PlatformVersion;

use drive::state_transition_action::batch::BatchTransitionAction;
use crate::execution::validation::state_transition::state_transitions::batch::action_validation::document::document_replace_transition_action::DocumentReplaceTransitionActionValidation;
use crate::execution::validation::state_transition::state_transitions::batch::action_validation::document::document_delete_transition_action::DocumentDeleteTransitionActionValidation;
use crate::execution::validation::state_transition::state_transitions::batch::action_validation::document::document_create_transition_action::DocumentCreateTransitionActionValidation;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::DocumentPurchaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::DocumentReplaceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::DocumentTransferTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_update_price_transition_action::DocumentUpdatePriceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use crate::error::execution::ExecutionError;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};
use crate::execution::validation::state_transition::batch::action_validation::document::document_purchase_transition_action::DocumentPurchaseTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_transfer_transition_action::DocumentTransferTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_update_price_transition_action::DocumentUpdatePriceTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::TokenBaseTransitionActionValidation;

pub(in crate::execution::validation::state_transition::state_transitions::batch) trait DocumentsBatchStateTransitionStructureValidationV0
{
    fn validate_advanced_structure_from_state_v0(
        &self,
        block_info: &BlockInfo,
        network: Network,
        action: &BatchTransitionAction,
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DocumentsBatchStateTransitionStructureValidationV0 for BatchTransition {
    fn validate_advanced_structure_from_state_v0(
        &self,
        block_info: &BlockInfo,
        network: Network,
        action: &BatchTransitionAction,
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let security_levels = action.combined_security_level_requirement()?;

        let signing_key = identity.loaded_public_keys.get(&self.signature_public_key_id()).ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution("the key must exist for advanced structure validation as we already fetched it during signature validation")))?;

        if !security_levels.contains(&signing_key.security_level()) {
            // We only need to bump the first identity data contract nonce as that will make a replay
            // attack not possible

            let first_transition = self.first_transition().ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution("There must be at least one state transition as this is already verified in basic validation")))?;

            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_batched_transition_ref(
                    first_transition,
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
        for transition in self.transitions_iter() {
            if let BatchedTransitionRef::Document(DocumentTransition::Create(create_transition)) =
                transition
            {
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
                            create_transition.base(),
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
                BatchedTransitionAction::DocumentAction(document_action) => match document_action {
                    DocumentTransitionAction::CreateAction(create_action) => {
                        let result = create_action.validate_structure(
                            identity.id,
                            block_info,
                            network,
                            platform_version,
                        )?;
                        if !result.is_valid() {
                            let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(document_action.base(), self.owner_id(), self.user_fee_increase()),
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
                                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(replace_action.base(), self.owner_id(), self.user_fee_increase()),
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
                                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(delete_action.base(), self.owner_id(), self.user_fee_increase()),
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
                                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(transfer_action.base(), self.owner_id(), self.user_fee_increase()),
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
                                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(update_price_action.base(), self.owner_id(), self.user_fee_increase()),
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
                                    BumpIdentityDataContractNonceAction::from_borrowed_document_base_transition_action(purchase_action.base(), self.owner_id(), self.user_fee_increase()),
                                );

                            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                                bump_action,
                                result.errors,
                            ));
                        }
                    }
                },
                BatchedTransitionAction::TokenAction(token_transition_action) => {
                    // token actions only need to do advanced structure validation on the base action
                    let result = token_transition_action
                        .base()
                        .validate_structure(platform_version)?;
                    if !result.is_valid() {
                        let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                            BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition_action(token_transition_action.base(), self.owner_id(), self.user_fee_increase()),
                        );

                        return Ok(ConsensusValidationResult::new_with_data_and_errors(
                            bump_action,
                            result.errors,
                        ));
                    }
                }
                BatchedTransitionAction::BumpIdentityDataContractNonce(_) => {
                    return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "we should not have a bump identity contract nonce at this stage",
                    )));
                }
            }
        }
        Ok(ConsensusValidationResult::new())
    }
}
