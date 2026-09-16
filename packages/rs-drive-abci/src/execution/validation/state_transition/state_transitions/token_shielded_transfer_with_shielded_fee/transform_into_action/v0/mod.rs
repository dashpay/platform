use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::token_pool_paid_common::{
    resolve_pooled_token, validate_credit_pool_fee_spend, validate_token_pool_spend,
};
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::token_shielded_transfer_with_shielded_fee_transition::TokenShieldedTransferWithShieldedFeeTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::token_shielded_transfer_with_shielded_fee::TokenShieldedTransferWithShieldedFeeTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::token_shielded_transfer_with_shielded_fee) trait TokenShieldedTransferWithShieldedFeeStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl TokenShieldedTransferWithShieldedFeeStateTransitionTransformIntoActionValidationV0
    for TokenShieldedTransferWithShieldedFeeTransition
{
    /// Both Orchard proofs and the fee floor were checked by the processor. Here the token
    /// must own a pool and not be paused, the token bundle must spend recorded and unspent
    /// notes of that pool, the fee bundle must spend recorded and unspent notes of the credit
    /// pool which must hold what leaves it, and the transition's own rules apply.
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let TokenShieldedTransferWithShieldedFeeTransition::V0(v0) = self;
        let (_contract, _configuration) = match resolve_pooled_token(
            drive,
            v0.data_contract_id,
            v0.token_contract_position,
            v0.token_id,
            transaction,
            platform_version,
        )? {
            Ok(resolved) => resolved,
            Err(rejection) => return Ok(rejection),
        };
        let token_nullifiers: Vec<[u8; 32]> =
            v0.token_actions.iter().map(|a| a.nullifier).collect();
        if let Some(rejection) = validate_token_pool_spend(
            drive,
            v0.token_id,
            &v0.token_anchor,
            &token_nullifiers,
            transaction,
            platform_version,
        )? {
            return Ok(rejection);
        }
        let fee_amount = v0.credit_amount;
        let fee_nullifiers: Vec<[u8; 32]> = v0.fee_actions.iter().map(|a| a.nullifier).collect();
        let current_credit_pool_balance = match validate_credit_pool_fee_spend(
            drive,
            &v0.fee_anchor,
            &fee_nullifiers,
            fee_amount,
            transaction,
            platform_version,
        )? {
            Ok(balance) => balance,
            Err(rejection) => return Ok(rejection),
        };
        let result = TokenShieldedTransferWithShieldedFeeTransitionAction::try_from_transition(
            self,
            fee_amount,
            current_credit_pool_balance,
        );
        Ok(result.map(|action| action.into()))
    }
}
