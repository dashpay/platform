use dpp::block::block_info::BlockInfo;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
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
    /// What an offer needs that reads nothing beyond the contract the action carries: a
    /// tradeable token. Its value was worked out, and refused when too large, by the
    /// transformer, which holds it in escrow.
    fn validate_structure_v0(
        &self,
        _block_info: &BlockInfo,
        _platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        validate_token_tradeable(self.base())
    }
}
