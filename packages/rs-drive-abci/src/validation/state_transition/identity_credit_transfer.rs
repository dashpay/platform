use dpp::identity::state_transition::identity_credit_transfer_transition::{
    IdentityCreditTransferTransition, IdentityCreditTransferTransitionAction,
};

use dpp::consensus::signature::{IdentityNotFoundError, SignatureError};
use dpp::identity::PartialIdentity;
use dpp::{
    state_transition::StateTransitionAction,
    validation::{ConsensusValidationResult, SimpleConsensusValidationResult},
};
use dpp::identity::state_transition::identity_credit_transfer_transition::validation::basic::identity_credit_transfer_basic::IDENTITY_CREDIT_TRANSFER_TRANSITION_SCHEMA_VALIDATOR;
use drive::grovedb::TransactionArg;

use super::StateTransitionValidation;
use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::common::{validate_protocol_version, validate_schema};
use crate::validation::state_transition::ValidationDataShareContext;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;

impl StateTransitionValidation for IdentityCreditTransferTransition {
    fn validate_structure<C: CoreRPCLike>(
        &self,
        _platform: &PlatformRef<C>,
        _context: &mut ValidationDataShareContext,
        _tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = validate_schema(&IDENTITY_CREDIT_TRANSFER_TRANSITION_SCHEMA_VALIDATOR, self);
        if !result.is_valid() {
            return Ok(result);
        }

        Ok(validate_protocol_version(self.protocol_version))
    }

    fn validate_identity_and_signatures<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _context: &mut ValidationDataShareContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        let mut validation_result = ConsensusValidationResult::<Option<PartialIdentity>>::default();

        let maybe_partial_identity = platform
            .drive
            .fetch_identity_with_balance(self.identity_id.to_buffer(), tx)?;

        let partial_identity = match maybe_partial_identity {
            None => {
                validation_result.add_error(SignatureError::IdentityNotFoundError(
                    IdentityNotFoundError::new(self.identity_id),
                ));
                return Ok(validation_result);
            }
            Some(partial_identity) => partial_identity,
        };

        validation_result.set_data(Some(partial_identity));
        Ok(validation_result)
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _context: &mut ValidationDataShareContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let maybe_existing_identity_balance = platform
            .drive
            .fetch_identity_balance(self.identity_id.to_buffer(), tx)?;

        let Some(existing_identity_balance) = maybe_existing_identity_balance else {
            return Ok(ConsensusValidationResult::new_with_error(IdentityNotFoundError::new(self.identity_id)));
        };

        if existing_identity_balance < self.amount {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(self.identity_id, existing_identity_balance),
            ));
        }

        let maybe_existing_recipient = platform
            .drive
            .fetch_identity_balance(self.recipient_id.to_buffer(), tx)?;

        if maybe_existing_recipient.is_none() {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityNotFoundError::new(self.recipient_id),
            ));
        }

        self.transform_into_action(platform, _context, tx)
    }

    fn transform_into_action<C: CoreRPCLike>(
        &self,
        _platform: &PlatformRef<C>,
        _context: &ValidationDataShareContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        Ok(ConsensusValidationResult::new_with_data(
            IdentityCreditTransferTransitionAction::from(self).into(),
        ))
    }
}
