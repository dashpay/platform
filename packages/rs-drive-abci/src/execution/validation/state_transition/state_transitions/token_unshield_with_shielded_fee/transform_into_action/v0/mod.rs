use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::token_pool_paid_common::{
    resolve_pooled_token, validate_credit_pool_fee_spend, validate_token_pool_holds,
    validate_token_pool_spend,
};
use dpp::consensus::state::token::TokenTransferRecipientIdentityNotExistError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::token_unshield_with_shielded_fee_transition::TokenUnshieldWithShieldedFeeTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::shielded::token_unshield_with_shielded_fee::TokenUnshieldWithShieldedFeeTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::token_unshield_with_shielded_fee) trait TokenUnshieldWithShieldedFeeStateTransitionTransformIntoActionValidationV0
{
    fn transform_into_action_v0(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl TokenUnshieldWithShieldedFeeStateTransitionTransformIntoActionValidationV0
    for TokenUnshieldWithShieldedFeeTransition
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
        let TokenUnshieldWithShieldedFeeTransition::V0(v0) = self;
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
        // The recipient must exist. A pooled token can never freeze an account, so no frozen
        // check is read.
        if drive
            .fetch_identity_balance(v0.recipient_id.to_buffer(), transaction, platform_version)?
            .is_none()
        {
            return Ok(ConsensusValidationResult::new_with_error(
                TokenTransferRecipientIdentityNotExistError::new(v0.recipient_id).into(),
            ));
        }
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
        if let Some(rejection) =
            validate_token_pool_holds(drive, v0.token_id, v0.amount, transaction, platform_version)?
        {
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
        let result = TokenUnshieldWithShieldedFeeTransitionAction::try_from_transition(
            self,
            fee_amount,
            current_credit_pool_balance,
        );
        Ok(result.map(|action| action.into()))
    }
}
