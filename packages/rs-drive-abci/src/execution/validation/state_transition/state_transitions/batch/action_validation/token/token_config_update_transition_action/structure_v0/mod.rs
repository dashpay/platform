use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::{
    TokenConfigUpdateTransitionAction, TokenConfigUpdateTransitionActionAccessorsV0,
};

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};

pub(in crate::execution::validation::state_transition::state_transitions::batch) trait TokenConfigUpdateTransitionActionStructureValidationV0
{
    fn validate_structure_v0(
        &self,
        execution_context: &mut StateTransitionExecutionContext,
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
    ///
    /// Then, for an update that is performed, the contract it stores held to the compiled size
    /// cap (`DataContractCompiledMaxSizeExceededError`, 10282), as contract create and update
    /// are: the update re-stores the whole contract compiled, a contract stored before
    /// protocol version 15 for the first time. The compile is charged to `execution_context`.
    fn validate_structure_v0(
        &self,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        if let TokenConfigurationChangeItem::MarketplaceTradeMode(trade_mode) =
            self.update_token_configuration_item()
        {
            let result = self.base().token_configuration()?.validate_trade_mode(
                trade_mode,
                self.token_position(),
                platform_version,
            );
            if !result.is_valid() {
                return Ok(result);
            }
        }
        if !self.base().perform_action() {
            return Ok(SimpleConsensusValidationResult::new());
        }
        let mut validation_operations = vec![];
        let result = self
            .updated_contract()?
            .validate_compiled_size(&mut validation_operations, platform_version)?;
        execution_context.add_dpp_operations(validation_operations);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v1::DataContractV1Setters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::identifier::Identifier;
    use drive::drive::contract::DataContractFetchInfo;
    use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
        TokenBaseTransitionAction, TokenBaseTransitionActionV0,
    };
    use drive::state_transition_action::batch::batched_transition::token_transition::token_config_update_transition_action::TokenConfigUpdateTransitionActionV0;
    use platform_version::DefaultForPlatformVersion;
    use std::sync::Arc;

    /// A max supply update of a token added to the DPNS contract, performed or not.
    fn max_supply_update(perform_action: bool) -> TokenConfigUpdateTransitionAction {
        let mut fetch_info = DataContractFetchInfo::dpns_contract_fixture(
            PlatformVersion::latest().protocol_version,
        );
        fetch_info.contract.add_token(
            0,
            TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
        );
        TokenConfigUpdateTransitionAction::V0(TokenConfigUpdateTransitionActionV0 {
            base: TokenBaseTransitionAction::V0(TokenBaseTransitionActionV0 {
                token_id: Identifier::from([1; 32]),
                identity_contract_nonce: 1,
                token_contract_position: 0,
                data_contract: Arc::new(fetch_info),
                store_in_group: None,
                perform_action,
            }),
            update_token_configuration_item: TokenConfigurationChangeItem::MaxSupply(Some(1_000)),
            public_note: None,
        })
    }

    fn execution_context(platform_version: &PlatformVersion) -> StateTransitionExecutionContext {
        StateTransitionExecutionContext::default_for_platform_version(platform_version)
            .expect("expected an execution context")
    }

    /// The latest protocol version with a compiled size cap every contract is over.
    fn latest_with_a_tiny_compiled_size_cap() -> PlatformVersion {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version.system_limits.max_compiled_contract_size = Some(100);
        platform_version
    }

    #[test]
    fn should_admit_a_performed_update_whose_contract_compiles_under_the_cap() {
        let result = max_supply_update(true)
            .validate_structure_v0(
                &mut execution_context(PlatformVersion::latest()),
                PlatformVersion::latest(),
            )
            .expect("expected the check to run");
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn should_refuse_a_performed_update_whose_contract_compiles_over_the_cap() {
        let result = max_supply_update(true)
            .validate_structure_v0(
                &mut execution_context(PlatformVersion::latest()),
                &latest_with_a_tiny_compiled_size_cap(),
            )
            .expect("expected the check to run");
        assert!(matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractCompiledMaxSizeExceededError(error)
            )] if error.max_compiled_size() == 100
        ));
    }

    #[test]
    fn should_not_hold_an_update_that_is_not_performed_to_the_cap() {
        let result = max_supply_update(false)
            .validate_structure_v0(
                &mut execution_context(PlatformVersion::latest()),
                &latest_with_a_tiny_compiled_size_cap(),
            )
            .expect("expected the check to run");
        assert!(result.is_valid(), "{:?}", result.errors);
    }
}
