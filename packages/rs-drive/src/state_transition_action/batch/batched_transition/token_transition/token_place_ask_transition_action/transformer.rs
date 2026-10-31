use dpp::platform_value::Identifier;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use std::sync::Arc;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::{ConsensusValidationResult, UserFeeIncrease};
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_place_ask_transition_action::{TokenPlaceAskTransitionActionV0, TokenPlaceAskTransitionAction};
use dpp::state_transition::batch_transition::token_place_ask_transition::TokenPlaceAskTransition;
use platform_version::version::PlatformVersion;
use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::batch::BatchedTransitionAction;

impl TokenPlaceAskTransitionAction {
    /// Transforms a borrowed `TokenPlaceAskTransition` into a `TokenPlaceAskTransitionAction`
    /// using the provided data contract lookup.
    #[allow(clippy::too_many_arguments)]
    pub fn try_from_borrowed_token_place_ask_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: &TokenPlaceAskTransition,
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
            TokenPlaceAskTransition::V0(v0) => {
                TokenPlaceAskTransitionActionV0::try_from_borrowed_token_place_ask_transition_with_contract_lookup(
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
