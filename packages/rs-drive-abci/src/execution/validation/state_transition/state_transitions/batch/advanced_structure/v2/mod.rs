use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::identity::PartialIdentity;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::{StateTransitionHasUserFeeIncrease, StateTransitionOwned};
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::batch::BatchTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_config_update_transition_action::TokenConfigUpdateTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_fill_ask_transition_action::TokenFillAskTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_fill_bid_transition_action::TokenFillBidTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_place_bid_transition_action::TokenPlaceBidTransitionActionValidation;
use crate::execution::validation::state_transition::batch::action_validation::token::token_place_ask_transition_action::TokenPlaceAskTransitionActionValidation;
use crate::execution::validation::state_transition::state_transitions::batch::advanced_structure::v1::DocumentsBatchStateTransitionStructureValidationV1;

pub(in crate::execution::validation::state_transition::state_transitions::batch) trait DocumentsBatchStateTransitionStructureValidationV2
{
    fn validate_advanced_structure_from_state_v2(
        &self,
        block_info: &BlockInfo,
        network: Network,
        action: &BatchTransitionAction,
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DocumentsBatchStateTransitionStructureValidationV2 for BatchTransition {
    /// v1, then the structure checks of token actions that protocol version 15 introduces. They
    /// read only the contract each action carries, so they run here, where check_tx runs them
    /// too, rather than in the state validation check_tx skips:
    ///
    /// - a `TokenConfigUpdate` that makes a token tradeable needs a transferable token
    ///   (`NonTransferableTokenTradeableError`, 10281);
    /// - a `TokenPlaceAsk` needs a tradeable token and the token's decimals, a `TokenPlaceBid`
    ///   a tradeable token, and a `TokenFillAsk` or a `TokenFillBid` a token that is still
    ///   tradeable.
    ///
    /// A failing action bumps the signer's nonce, as every advanced structure failure does.
    fn validate_advanced_structure_from_state_v2(
        &self,
        block_info: &BlockInfo,
        network: Network,
        action: &BatchTransitionAction,
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let result = self.validate_advanced_structure_from_state_v1(
            block_info,
            network,
            action,
            identity,
            execution_context,
            platform_version,
        )?;
        if !result.is_valid() {
            return Ok(result);
        }

        for transition in action.transitions() {
            let BatchedTransitionAction::TokenAction(token_transition_action) = transition else {
                continue;
            };
            let structure_result = match token_transition_action {
                TokenTransitionAction::ConfigUpdateAction(config_update_action) => {
                    config_update_action.validate_structure(platform_version)?
                }
                TokenTransitionAction::PlaceAskAction(place_ask_action) => {
                    place_ask_action.validate_structure(block_info, platform_version)?
                }
                TokenTransitionAction::FillAskAction(fill_ask_action) => {
                    fill_ask_action.validate_structure(platform_version)?
                }
                TokenTransitionAction::PlaceBidAction(place_bid_action) => {
                    place_bid_action.validate_structure(block_info, platform_version)?
                }
                TokenTransitionAction::FillBidAction(fill_bid_action) => {
                    fill_bid_action.validate_structure(platform_version)?
                }
                _ => continue,
            };
            if !structure_result.is_valid() {
                let bump_action = StateTransitionAction::BumpIdentityDataContractNonceAction(
                    BumpIdentityDataContractNonceAction::from_borrowed_token_base_transition_action(
                        token_transition_action.base(),
                        self.owner_id(),
                        self.user_fee_increase(),
                    ),
                );
                return Ok(ConsensusValidationResult::new_with_data_and_errors(
                    bump_action,
                    structure_result.errors,
                ));
            }
        }

        Ok(result)
    }
}
