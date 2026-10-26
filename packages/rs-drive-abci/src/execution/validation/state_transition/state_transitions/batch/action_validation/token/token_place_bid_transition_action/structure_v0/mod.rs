use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::token::TokenOrderDecimalsMismatchError;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Getters;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_place_bid_transition_action::{TokenPlaceBidTransitionAction, TokenPlaceBidTransitionActionAccessorsV0};

use crate::error::Error;
use crate::execution::validation::state_transition::batch::action_validation::token::token_order_common::validate_token_tradeable;

pub(in crate::execution::validation::state_transition::state_transitions::batch) trait TokenPlaceBidTransitionActionStructureValidationV0
{
    fn validate_structure_v0(
        &self,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl TokenPlaceBidTransitionActionStructureValidationV0 for TokenPlaceBidTransitionAction {
    /// What an offer needs that reads nothing beyond the contract the action carries, as a
    /// listing does: a tradeable token, and the decimals it was signed at being the token's (its
    /// price is per whole token, so a change of the token's decimals between signing and
    /// execution would reprice it). Its escrow was worked out at the signed decimals by the
    /// transformer, and its value refused when too large by the transition's structure.
    fn validate_structure_v0(
        &self,
        _block_info: &BlockInfo,
        _platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = validate_token_tradeable(self.base())?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let token_decimals = self.base().token_configuration()?.conventions().decimals();
        let decimals = self.decimals();
        if decimals != token_decimals {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                TokenOrderDecimalsMismatchError::new(
                    self.base().token_id(),
                    decimals,
                    token_decimals,
                )
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
