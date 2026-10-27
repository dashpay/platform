use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::token::TokenNotTradeableError;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Getters;
use dpp::state_transition::batch_transition::batched_transition::token_order_terms::validate_token_order_terms_for_token;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_place_ask_transition_action::{TokenPlaceAskTransitionAction, TokenPlaceAskTransitionActionAccessorsV0};

use crate::error::Error;
use crate::execution::validation::state_transition::batch::action_validation::token::token_order_common::validate_token_tradeable;

pub(in crate::execution::validation::state_transition::state_transitions::batch) trait TokenPlaceAskTransitionActionStructureValidationV0
{
    fn validate_structure_v0(
        &self,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl TokenPlaceAskTransitionActionStructureValidationV0 for TokenPlaceAskTransitionAction {
    /// What a listing needs that reads nothing beyond the contract the action carries: a
    /// tradeable token, an amount of at least the token's minimum fill, and a worth of at most
    /// `i64::MAX` credits at the token's decimals (`validate_token_order_terms_for_token`), both
    /// as the token has them now.
    fn validate_structure_v0(
        &self,
        _block_info: &BlockInfo,
        _platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = validate_token_tradeable(self.base())?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let token_configuration = self.base().token_configuration()?;
        let Some(min_fill) = token_configuration.min_fill() else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                TokenNotTradeableError::new(self.base().token_id()).into(),
            ));
        };
        Ok(validate_token_order_terms_for_token(
            self.amount(),
            self.price(),
            min_fill,
            token_configuration.conventions().decimals(),
        ))
    }
}
