use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use dpp::consensus::signature::IdentityNotFoundError;
use dpp::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::consensus::state::state_error::StateError;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;

use drive::grovedb::TransactionArg;
use drive::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::identity_credit_withdrawal) trait IdentityCreditWithdrawalStateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreditWithdrawalStateTransitionStateValidationV0
    for IdentityCreditWithdrawalTransition
{
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
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

        let Some(revision) = platform.drive.fetch_identity_revision(
            self.identity_id().to_buffer(),
            true,
            tx,
            platform_version,
        )?
        else {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityNotFoundError::new(self.identity_id()).into(),
            ));
        };

        // Check revision
        if revision + 1 != self.revision() {
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidIdentityRevisionError(InvalidIdentityRevisionError::new(
                    self.identity_id(),
                    revision,
                ))
                .into(),
            ));
        }

        self.transform_into_action_v0(platform)
    }

    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let last_block_time =
            platform
                .state
                .last_committed_block_time_ms()
                .ok_or(Error::Execution(ExecutionError::StateNotInitialized(
                    "expected a last platform block during identity update validation",
                )))?;

        Ok(ConsensusValidationResult::new_with_data(
            IdentityCreditWithdrawalTransitionAction::from_identity_credit_withdrawal(
                self,
                last_block_time,
            )
            .into(),
        ))
    }
}
