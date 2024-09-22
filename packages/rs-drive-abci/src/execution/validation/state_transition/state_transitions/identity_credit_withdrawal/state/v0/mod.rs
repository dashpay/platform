use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;

use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use dpp::consensus::signature::IdentityNotFoundError;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;

use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::identity_credit_withdrawal) trait IdentityCreditWithdrawalStateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreditWithdrawalStateTransitionStateValidationV0
    for IdentityCreditWithdrawalTransition
{
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let maybe_existing_identity_balance = platform.drive.fetch_identity_balance(
            self.identity_id().to_buffer(),
            tx,
            platform_version,
        )?;

        let Some(existing_identity_balance) = maybe_existing_identity_balance else {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityNotFoundError::new(self.identity_id()).into(),
            ));
        };

        if existing_identity_balance < self.amount() {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(
                    self.identity_id(),
                    existing_identity_balance,
                    self.amount(),
                )
                .into(),
            ));
        }

        self.transform_into_action_v0(platform, block_info, tx, platform_version)
    }

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        Ok(
            IdentityCreditWithdrawalTransitionAction::try_from_identity_credit_withdrawal(
                &platform.drive,
                tx,
                self,
                block_info,
                platform_version,
            )
            .map(|consensus_validation_result| {
                consensus_validation_result.map(|withdrawal| withdrawal.into())
            })?,
        )
    }
}
