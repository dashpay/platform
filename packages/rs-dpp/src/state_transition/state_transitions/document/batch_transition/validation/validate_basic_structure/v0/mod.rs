use crate::consensus::basic::document::{
    DocumentTransitionsAreAbsentError, DuplicateDocumentTransitionsWithIdsError,
    MaxDocumentsTransitionsExceededError, NonceOutOfBoundsError,
};
use crate::consensus::basic::BasicError;

use crate::identity::identity_nonce::MISSING_IDENTITY_REVISIONS_FILTER;
use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use crate::state_transition::batch_transition::validation::find_duplicates_by_id::find_duplicates_by_id;
use crate::state_transition::batch_transition::BatchTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use crate::consensus::basic::group::GroupActionNotAllowedOnTransitionError;
use crate::consensus::basic::token::{InvalidActionIdError, InvalidTokenIdError};
use crate::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use crate::state_transition::batch_transition::batched_transition::token_transition::{TokenTransition, TokenTransitionV0Methods};
use crate::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionTypeGetter;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_config_update_transition::validate_structure::TokenConfigUpdateTransitionStructureValidation;
use crate::state_transition::batch_transition::token_destroy_frozen_funds_transition::validate_structure::TokenDestroyFrozenFundsTransitionStructureValidation;
use crate::state_transition::batch_transition::token_emergency_action_transition::validate_structure::TokenEmergencyActionTransitionStructureValidation;
use crate::state_transition::batch_transition::token_freeze_transition::validate_structure::TokenFreezeTransitionStructureValidation;
use crate::state_transition::batch_transition::token_mint_transition::validate_structure::TokenMintTransitionStructureValidation;
use crate::state_transition::batch_transition::token_claim_transition::validate_structure::TokenClaimTransitionStructureValidation;
use crate::state_transition::batch_transition::token_direct_purchase_transition::validate_structure::TokenDirectPurchaseTransitionStructureValidation;
use crate::state_transition::batch_transition::token_set_price_for_direct_purchase_transition::validate_structure::TokenSetPriceForDirectPurchaseTransitionStructureValidation;
use crate::state_transition::batch_transition::token_transfer_transition::validate_structure::TokenTransferTransitionStructureValidation;
use crate::state_transition::batch_transition::token_unfreeze_transition::validate_structure::TokenUnfreezeTransitionStructureValidation;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::document_transition::{DocumentTransition, DocumentTransitionV0Methods};
use crate::state_transition::StateTransitionLike;
use crate::state_transition::state_transitions::document::batch_transition::batched_transition::token_burn_transition::validate_structure::TokenBurnTransitionStructureValidation;

impl BatchTransition {
    #[inline(always)]
    pub(super) fn validate_base_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if self.transitions_are_empty() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DocumentTransitionsAreAbsentError::new().into(),
            ));
        }

        let transitions_len = self.transitions_len();

        if transitions_len > u16::MAX as usize
            || transitions_len as u16
                > platform_version
                    .system_limits
                    .max_transitions_in_documents_batch
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                MaxDocumentsTransitionsExceededError::new(
                    platform_version
                        .system_limits
                        .max_transitions_in_documents_batch,
                )
                .into(),
            ));
        }

        // Group transitions by contract ID
        let mut document_transitions_by_contracts: BTreeMap<Identifier, Vec<&DocumentTransition>> =
            BTreeMap::new();

        // Group transitions by contract ID
        let mut token_transitions: Vec<&TokenTransition> = vec![];

        self.transitions_iter()
            .for_each(|batch_transition| match batch_transition {
                BatchedTransitionRef::Document(document_transition) => {
                    let contract_identifier = document_transition.data_contract_id();

                    match document_transitions_by_contracts.entry(contract_identifier) {
                        Entry::Vacant(vacant) => {
                            vacant.insert(vec![document_transition]);
                        }
                        Entry::Occupied(mut identifiers) => {
                            identifiers.get_mut().push(document_transition);
                        }
                    };
                }
                BatchedTransitionRef::Token(token_transition) => {
                    token_transitions.push(token_transition)
                }
            });

        let mut result = SimpleConsensusValidationResult::default();

        for transitions in document_transitions_by_contracts.values() {
            for transition in transitions {
                // We need to make sure that the identity contract nonce is within the allowed bounds
                // This means that it is stored on 40 bits
                if transition.identity_contract_nonce() & MISSING_IDENTITY_REVISIONS_FILTER > 0 {
                    result.add_error(BasicError::NonceOutOfBoundsError(
                        NonceOutOfBoundsError::new(transition.identity_contract_nonce()),
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

        for transition in token_transitions {
            // We need to make sure that the identity contract nonce is within the allowed bounds
            // This means that it is stored on 40 bits
            if transition.identity_contract_nonce() & MISSING_IDENTITY_REVISIONS_FILTER > 0 {
                result.add_error(BasicError::NonceOutOfBoundsError(
                    NonceOutOfBoundsError::new(transition.identity_contract_nonce()),
                ));
            }

            let transition_token_id = transition.base().token_id();
            let calculated_token_id = transition.base().calculate_token_id();

            // We need to verify that the token id is correct
            if transition_token_id != calculated_token_id {
                result.add_error(BasicError::InvalidTokenIdError(InvalidTokenIdError::new(
                    calculated_token_id,
                    transition_token_id,
                )));
            }

            let consensus_result = match transition {
                TokenTransition::Burn(burn_transition) => {
                    burn_transition.validate_structure(platform_version)?
                }
                TokenTransition::Mint(mint_transition) => {
                    mint_transition.validate_structure(platform_version)?
                }
                TokenTransition::Transfer(transfer_transition) => {
                    transfer_transition.validate_structure(self.owner_id(), platform_version)?
                }
                TokenTransition::Freeze(freeze_transition) => {
                    freeze_transition.validate_structure(platform_version)?
                }
                TokenTransition::Unfreeze(unfreeze_transition) => {
                    unfreeze_transition.validate_structure(platform_version)?
                }
                TokenTransition::DestroyFrozenFunds(destroy_frozen_funds_transition) => {
                    destroy_frozen_funds_transition.validate_structure(platform_version)?
                }
                TokenTransition::EmergencyAction(emergency_action_transition) => {
                    emergency_action_transition.validate_structure(platform_version)?
                }
                TokenTransition::ConfigUpdate(config_update_transition) => {
                    config_update_transition.validate_structure(platform_version)?
                }
                TokenTransition::Claim(release_transition) => {
                    release_transition.validate_structure(platform_version)?
                }
                TokenTransition::DirectPurchase(direct_purchase_transition) => {
                    direct_purchase_transition.validate_structure(platform_version)?
                }
                TokenTransition::SetPriceForDirectPurchase(
                    set_price_for_direct_purchase_transition,
                ) => {
                    set_price_for_direct_purchase_transition.validate_structure(platform_version)?
                }
            };

            if !consensus_result.is_valid() {
                return Ok(consensus_result);
            }

            // We need to verify that the action id given matches the expected action id
            // But only if we are the proposer
            if let Some(group_state_transition_info) = transition.base().using_group_info() {
                if group_state_transition_info.action_is_proposer {
                    if let Some(calculated_action_id) =
                        transition.calculate_action_id(self.owner_id())
                    {
                        if group_state_transition_info.action_id != calculated_action_id {
                            result.add_error(BasicError::InvalidActionIdError(
                                InvalidActionIdError::new(
                                    calculated_action_id,
                                    group_state_transition_info.action_id,
                                ),
                            ));
                        }
                    } else {
                        result.add_error(BasicError::GroupActionNotAllowedOnTransitionError(
                            GroupActionNotAllowedOnTransitionError::new(
                                transition.action_type().to_string(),
                            ),
                        ));
                    }
                } else if !transition.can_calculate_action_id() {
                    result.add_error(BasicError::GroupActionNotAllowedOnTransitionError(
                        GroupActionNotAllowedOnTransitionError::new(
                            transition.action_type().to_string(),
                        ),
                    ));
                }
            }
        }

        Ok(result)
    }
}
