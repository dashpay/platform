use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::batched_transition::token_transition::token_mint_to_pool_transition_action::{TokenMintToPoolTransitionAction, TokenMintToPoolTransitionActionV0};
use crate::state_transition_action::batch::BatchedTransitionAction;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::Identifier;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use dpp::state_transition::batch_transition::token_mint_to_pool_transition::TokenMintToPoolTransition;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::sync::Arc;

impl TokenMintToPoolTransitionAction {
    /// Resolves the transition against state (contract lookup, base checks and, for a claim,
    /// the amount) into an action, or into a nonce bump carrying the consensus errors.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_mint_to_pool_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenMintToPoolTransition,
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
            TokenMintToPoolTransition::V0(v0) => {
                TokenMintToPoolTransitionActionV0::try_from_borrowed_token_mint_to_pool_transition_with_contract_lookup(
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
