use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_shield_transition_action::{TokenShieldTransitionAction, TokenShieldTransitionActionV0};
use crate::state_transition_action::batch::BatchedTransitionAction;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::Identifier;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::state_transition::batch_transition::token_shield_transition::TokenShieldTransition;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::sync::Arc;

impl TokenShieldTransitionAction {
    /// Converts a borrowed `TokenShieldTransition` into a `TokenShieldTransitionAction`, resolving the token's contract
    /// and metering the lookup. A failed base resolution becomes a paid nonce bump.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_shield_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenShieldTransition,
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
            TokenShieldTransition::V0(v0) => {
                TokenShieldTransitionActionV0::try_from_borrowed_token_shield_transition_with_contract_lookup(
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
