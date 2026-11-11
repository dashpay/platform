use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::batch::batched_transition::token_transition::token_fill_bid_transition_action::{TokenFillBidTransitionAction, TokenFillBidTransitionActionAccessorsV0};

use crate::error::Error;
use crate::execution::validation::state_transition::batch::action_validation::token::token_order_common::validate_token_tradeable;

pub(in crate::execution::validation::state_transition::state_transitions::batch) trait TokenFillBidTransitionActionStructureValidationV0
{
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl TokenFillBidTransitionActionStructureValidationV0 for TokenFillBidTransitionAction {
    /// A sale needs a token that is still tradeable: one whose trade mode changed back to
    /// `NotTradeable` keeps its open offers, which can be cancelled but no longer filled.
    fn validate_structure_v0(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        validate_token_tradeable(self.base())
    }
}
