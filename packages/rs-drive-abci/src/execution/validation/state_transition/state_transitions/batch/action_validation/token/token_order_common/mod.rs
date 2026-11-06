//! Stateful checks shared by the token marketplace validators: the token must be tradeable and
//! an identity whose tokens a listing moves must not be frozen. Every read is metered into the
//! execution context, so the batch owner pays for it.

use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::state::token::{IdentityTokenAccountFrozenError, TokenNotTradeableError};
use dpp::consensus::ConsensusError;
use dpp::prelude::Identifier;
use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::query::TransactionArg;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
    TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
};

/// The token's configuration must allow trading (`marketplaceRules.tradeMode` `Tradeable`). A
/// token whose trade mode changes back to `NotTradeable` keeps its open orders, which can still
/// be cancelled but no longer filled.
pub(crate) fn validate_token_tradeable(
    base: &TokenBaseTransitionAction,
) -> Result<SimpleConsensusValidationResult, Error> {
    if !base.token_configuration()?.is_tradeable() {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            TokenNotTradeableError::new(base.token_id()).into(),
        ));
    }
    Ok(SimpleConsensusValidationResult::new())
}

/// The token account of `identity_id` must not be frozen.
#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_token_account_not_frozen(
    platform: &PlatformStateRef,
    token_id: Identifier,
    identity_id: Identifier,
    action: &str,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let (info, fee_result) = platform.drive.fetch_identity_token_info_with_costs(
        token_id.to_buffer(),
        identity_id.to_buffer(),
        block_info,
        true,
        transaction,
        platform_version,
    )?;

    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee_result));

    if info.is_some_and(|info| info.frozen()) {
        return Ok(SimpleConsensusValidationResult::new_with_error(
            ConsensusError::StateError(StateError::IdentityTokenAccountFrozenError(
                IdentityTokenAccountFrozenError::new(token_id, identity_id, action.to_string()),
            )),
        ));
    }
    Ok(SimpleConsensusValidationResult::new())
}
