use dpp::platform_value::Identifier;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use std::sync::Arc;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_burn_transition_action::{
    TokenBurnTransitionAction, TokenBurnTransitionActionV0,
};
use dpp::state_transition::batch_transition::token_burn_transition::TokenBurnTransition;
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::BatchedTransitionAction;

/// Implement methods to transform a `TokenBurnTransition` into a `TokenBurnTransitionAction`.
impl TokenBurnTransitionAction {
    /// Transform a `TokenBurnTransition` into a `TokenBurnTransitionAction` using the provided data contract lookup.
    ///
    /// This method processes a `TokenBurnTransition` and converts it into a `TokenBurnTransitionAction` while
    /// looking up necessary data contracts and calculating transaction fees.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance used for accessing the system.
    /// * `owner_id` - The identifier of the owner initiating the burn transition.
    /// * `value` - The `TokenBurnTransition` instance to be converted into a `TokenBurnTransitionAction`.
    /// * `approximate_without_state_for_costs` - A flag indicating whether to approximate state costs without full state information.
    /// * `transaction` - The transaction argument used for state changes.
    /// * `block_info` - Block information needed to process the transition.
    /// * `get_data_contract` - A closure that retrieves the `DataContractFetchInfo` for a given contract ID.
    /// * `platform_version` - The platform version in use for the context of the transition.
    ///
    /// # Returns
    ///
    /// * `Result<(TokenBurnTransitionAction, FeeResult), Error>` - A result containing the `TokenBurnTransitionAction` and associated fees if successful, otherwise an error.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_token_burn_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenBurnTransition,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        match value {
            TokenBurnTransition::V0(v0) => {
                TokenBurnTransitionActionV0::try_from_token_burn_transition_with_contract_lookup(
                    drive,
                    owner_id,
                    v0,
                    approximate_without_state_for_costs,
                    transaction,
                    block_info,
                    user_fee_increase,
                    get_data_contract,
                    platform_version,
                )
            }
        }
    }

    /// Transform a borrowed `TokenBurnTransition` into a `TokenBurnTransitionAction` using the provided data contract lookup.
    ///
    /// This method processes a borrowed reference to a `TokenBurnTransition` and converts it into a `TokenBurnTransitionAction`
    /// while looking up necessary data contracts and calculating transaction fees.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance used for accessing the system.
    /// * `owner_id` - The identifier of the owner initiating the burn transition.
    /// * `value` - A reference to the `TokenBurnTransition` to be converted into a `TokenBurnTransitionAction`.
    /// * `approximate_without_state_for_costs` - A flag indicating whether to approximate state costs without full state information.
    /// * `transaction` - The transaction argument used for state changes.
    /// * `block_info` - Block information needed to process the transition.
    /// * `get_data_contract` - A closure that retrieves the `DataContractFetchInfo` for a given contract ID.
    /// * `platform_version` - The platform version in use for the context of the transition.
    ///
    /// # Returns
    ///
    #[allow(clippy::too_many_arguments)]
    /// * `Result<(TokenBurnTransitionAction, FeeResult), Error>` - A result containing the `TokenBurnTransitionAction` and associated fees if successful, otherwise an error.
    pub fn try_from_borrowed_token_burn_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenBurnTransition,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        user_fee_increase: UserFeeIncrease,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<
        (
            ConsensusValidationResult<BatchedTransitionAction>,
            FeeResult,
        ),
        Error,
    > {
        match value {
            TokenBurnTransition::V0(v0) => {
                TokenBurnTransitionActionV0::try_from_borrowed_token_burn_transition_with_contract_lookup(
                    drive,
                    owner_id,
                    v0,
                    approximate_without_state_for_costs,
                    transaction,
                    block_info,
                    user_fee_increase,
                    get_data_contract,
                    platform_version,
                )
            }
        }
    }
}
