use crate::error::Error;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use dpp::consensus::signature::IdentityNotFoundError;

use dpp::consensus::state::identity::IdentityInsufficientBalanceError;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::{
    IdentityCreditTransferTransition, IdentityCreditTransferTransitionAction,
};

use dpp::state_transition_action::StateTransitionAction;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

pub(crate) trait StateTransitionStateValidationV0 {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    fn transform_into_action_v0(
        &self,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionStateValidationV0 for IdentityCreditTransferTransition {
    fn validate_state_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let maybe_existing_identity_balance = platform.drive.fetch_identity_balance(
            self.identity_id().to_buffer(),
            tx,
            platform_version,
        )?;

        let Some(existing_identity_balance) = maybe_existing_identity_balance else {
            return Ok(ConsensusValidationResult::new_with_error(IdentityNotFoundError::new(self.identity_id()).into()));
        };

        if existing_identity_balance < self.amount() {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(
                    self.identity_id(),
                    existing_identity_balance,
                )
                .into(),
            ));
        }

        let maybe_existing_recipient = platform.drive.fetch_identity_balance(
            self.recipient_id().to_buffer(),
            tx,
            platform_version,
        )?;

        if maybe_existing_recipient.is_none() {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityNotFoundError::new(self.recipient_id()).into(),
            ));
        }

        self.transform_into_action_v0()
    }

    fn transform_into_action_v0(
        &self,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        Ok(ConsensusValidationResult::new_with_data(
            IdentityCreditTransferTransitionAction::from(self).into(),
        ))
    }
}
