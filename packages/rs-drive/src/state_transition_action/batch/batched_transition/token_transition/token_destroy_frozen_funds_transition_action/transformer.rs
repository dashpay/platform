use dpp::platform_value::Identifier;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use std::sync::Arc;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_destroy_frozen_funds_transition_action::{TokenDestroyFrozenFundsTransitionActionV0, TokenDestroyFrozenFundsTransitionAction};
use dpp::state_transition::batch_transition::token_destroy_frozen_funds_transition::TokenDestroyFrozenFundsTransition;
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::BatchedTransitionAction;

/// Implement methods to transform a `TokenDestroyFrozenFundsTransition` into a `TokenDestroyFrozenFundsTransitionAction`.
impl TokenDestroyFrozenFundsTransitionAction {
    /// Transform a `TokenDestroyFrozenFundsTransition` into a `TokenDestroyFrozenFundsTransitionAction` using the provided data contract lookup.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance used for accessing the system.
    /// * `owner_id` - The identifier of the owner initiating the destroy_frozen_funds transition.
    /// * `transaction` - The transaction argument used for state changes.
    /// * `value` - A `TokenDestroyFrozenFundsTransition` instance.
    /// * `approximate_without_state_for_costs` - A flag indicating whether to approximate state costs without full state.
    /// * `drive_operations` - A mutable reference to the vector of low-level operations that need to be performed.
    /// * `get_data_contract` - A closure that fetches the `DataContractFetchInfo` given a contract ID.
    /// * `platform_version` - The platform version for the context in which the transition is being executed.
    ///
    /// # Returns
    ///
    /// * `Result<(ConsensusValidationResult<BatchedTransitionAction>, FeeResult), Error>` - A `TokenDestroyFrozenFundsTransitionAction` if successful, otherwise `ProtocolError`.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_token_destroy_frozen_funds_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenDestroyFrozenFundsTransition,
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
            TokenDestroyFrozenFundsTransition::V0(v0) => {
                TokenDestroyFrozenFundsTransitionActionV0::try_from_token_destroy_frozen_funds_transition_with_contract_lookup(
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

    /// Transform a borrowed `TokenDestroyFrozenFundsTransition` into a `TokenDestroyFrozenFundsTransitionAction` using the provided data contract lookup.
    ///
    /// # Arguments
    ///
    /// * `drive` - A reference to the `Drive` instance used for accessing the system.
    /// * `owner_id` - The identifier of the owner initiating the destroy_frozen_funds transition.
    /// * `transaction` - The transaction argument used for state changes.
    /// * `value` - A reference to a `TokenDestroyFrozenFundsTransition`.
    /// * `approximate_without_state_for_costs` - A flag indicating whether to approximate state costs without full state.
    /// * `drive_operations` - A mutable reference to the vector of low-level operations that need to be performed.
    /// * `get_data_contract` - A closure that fetches the `DataContractFetchInfo` given a contract ID.
    /// * `platform_version` - The platform version for the context in which the transition is being executed.
    ///
    /// # Returns
    ///
    #[allow(clippy::too_many_arguments)]
    /// * `Result<(ConsensusValidationResult<BatchedTransitionAction>, FeeResult), Error>` - A `TokenDestroyFrozenFundsTransitionAction` if successful, otherwise `ProtocolError`.
    pub fn try_from_borrowed_token_destroy_frozen_funds_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenDestroyFrozenFundsTransition,
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
            TokenDestroyFrozenFundsTransition::V0(v0) => {
                TokenDestroyFrozenFundsTransitionActionV0::try_from_borrowed_token_destroy_frozen_funds_transition_with_contract_lookup(
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
