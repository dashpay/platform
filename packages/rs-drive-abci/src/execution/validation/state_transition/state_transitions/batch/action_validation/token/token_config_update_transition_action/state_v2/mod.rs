use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::prelude::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::{
    TokenConfigUpdateTransitionAction, TokenConfigUpdateTransitionActionAccessorsV0,
};

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_config_update_transition_action::state_v1::TokenConfigUpdateTransitionActionStateValidationV1;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait TokenConfigUpdateTransitionActionStateValidationV2 {
    fn validate_state_v2(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl TokenConfigUpdateTransitionActionStateValidationV2 for TokenConfigUpdateTransitionAction {
    /// v1, then the marketplace rules of the configuration the update proposes (protocol version
    /// 15): a `MarketplaceTradeMode` item can make a token tradeable only if the token is
    /// transferable (`NonTransferableTokenTradeableError`, 10281), the rule contract create and
    /// update apply to the configuration they carry. `transferable` is fixed at creation, so the
    /// trade mode is the only half of the rule an update can change.
    fn validate_state_v2(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.validate_state_v1(
            platform,
            owner_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let contract = &self.data_contract_fetch_info_ref().contract;
        let mut proposed_token_configuration = contract
            .expected_token_configuration(self.token_position())?
            .clone();
        proposed_token_configuration
            .apply_token_configuration_item(self.update_token_configuration_item().clone());

        Ok(proposed_token_configuration
            .validate_marketplace_rules(self.token_position(), platform_version))
    }
}
