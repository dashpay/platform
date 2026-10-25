use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::{
    TokenConfigUpdateTransitionAction, TokenConfigUpdateTransitionActionAccessorsV0,
};

use crate::error::Error;

pub(in crate::execution::validation::state_transition::state_transitions::batch) trait TokenConfigUpdateTransitionActionStructureValidationV0
{
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl TokenConfigUpdateTransitionActionStructureValidationV0 for TokenConfigUpdateTransitionAction {
    /// The marketplace rules of the trade mode the update proposes (protocol version 15): a
    /// `MarketplaceTradeMode` item can make a token tradeable only if the token is transferable
    /// (`NonTransferableTokenTradeableError`, 10281), the rule contract create and update apply to
    /// the configuration they carry. `transferable` is fixed at creation and no other item changes
    /// the trade mode, so the proposed mode is checked against the configuration the action
    /// carries, without building the proposed configuration and without a read.
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let TokenConfigurationChangeItem::MarketplaceTradeMode(trade_mode) =
            self.update_token_configuration_item()
        else {
            return Ok(SimpleConsensusValidationResult::new());
        };
        Ok(self.base().token_configuration()?.validate_trade_mode(
            trade_mode,
            self.token_position(),
            platform_version,
        ))
    }
}
