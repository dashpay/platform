use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::token::TokenOrderExpiredError;
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
    /// What an offer needs that reads nothing beyond the contract the action carries and the
    /// block: a tradeable token and, when it expires, an expiry after the block. Its value was
    /// worked out, and refused when too large, by the transformer, which holds it in escrow.
    fn validate_structure_v0(
        &self,
        block_info: &BlockInfo,
        _platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = validate_token_tradeable(self.base())?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        if let Some(expires_at) = self.expires_at() {
            if expires_at <= block_info.time_ms {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    TokenOrderExpiredError::new(self.order_id(), expires_at, block_info.time_ms)
                        .into(),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
