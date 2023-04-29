use dpp::identity::PartialIdentity;
use dpp::{identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition, state_transition::StateTransitionAction, validation::{ConsensusValidationResult, SimpleConsensusValidationResult}};
use dpp::consensus::basic::identity::{InvalidIdentityCreditWithdrawalTransitionCoreFeeError, InvalidIdentityCreditWithdrawalTransitionOutputScriptError, NotImplementedIdentityCreditWithdrawalTransitionPoolingError};
use dpp::consensus::signature::IdentityNotFoundError;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use dpp::consensus::state::state_error::StateError;
use dpp::identity::state_transition::identity_credit_withdrawal_transition::{IdentityCreditWithdrawalTransitionAction, Pooling};
use dpp::identity::state_transition::identity_credit_withdrawal_transition::validation::basic::validate_identity_credit_withdrawal_transition_basic::IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_SCHEMA_VALIDATOR;
use dpp::util::is_fibonacci_number::is_fibonacci_number;
use drive::grovedb::TransactionArg;
use drive::drive::Drive;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::common::validate_schema;
use crate::validation::state_transition::key_validation::validate_state_transition_identity_signature;

use super::StateTransitionValidation;

impl StateTransitionValidation for IdentityCreditWithdrawalTransition {
    fn validate_structure(
        &self,
        _drive: &Drive,
        _tx: TransactionArg,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = validate_schema(
            &IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_SCHEMA_VALIDATOR,
            self,
        );
        if !result.is_valid() {
            return Ok(result);
        }

        //todo: version validation
        //Ok(validate_protocol_version(self.protocol_version))

        // currently we do not support pooling, so we must validate that pooling is `Never`

        if self.pooling != Pooling::Never {
            result.add_error(
                NotImplementedIdentityCreditWithdrawalTransitionPoolingError::new(
                    self.pooling as u8,
                ),
            );

            return Ok(result);
        }

        // validate core_fee is in fibonacci sequence

        if !is_fibonacci_number(self.core_fee_per_byte) {
            result.add_error(InvalidIdentityCreditWithdrawalTransitionCoreFeeError::new(
                self.core_fee_per_byte,
            ));

            return Ok(result);
        }

        // validate output_script types
        if !self.output_script.is_p2pkh() && !self.output_script.is_p2sh() {
            result.add_error(
                InvalidIdentityCreditWithdrawalTransitionOutputScriptError::new(
                    self.output_script.clone(),
                ),
            );
        }

        Ok(result)
    }

    fn validate_identity_and_signatures(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
    ) -> Result<ConsensusValidationResult<Option<PartialIdentity>>, Error> {
        Ok(
            validate_state_transition_identity_signature(drive, self, false, transaction)?
                .map(Some),
        )
    }

    fn validate_state<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let maybe_existing_identity_balance = platform
            .drive
            .fetch_identity_balance(self.identity_id.to_buffer(), tx)?;

        let Some(existing_identity_balance) = maybe_existing_identity_balance else {
            return Ok(ConsensusValidationResult::new_with_error(IdentityNotFoundError::new(self.identity_id).into()));
        };

        if existing_identity_balance < self.amount {
            return Ok(ConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(self.identity_id, existing_identity_balance)
                    .into(),
            ));
        }

        let Some(revision) = platform.drive.fetch_identity_revision(self.identity_id.to_buffer(), true, tx)? else {
            return Ok(ConsensusValidationResult::new_with_error(IdentityNotFoundError::new(self.identity_id).into()));
        };

        // Check revision
        if revision + 1 != self.revision {
            return Ok(ConsensusValidationResult::new_with_error(
                StateError::InvalidIdentityRevisionError(InvalidIdentityRevisionError::new(
                    self.identity_id,
                    revision,
                ))
                .into(),
            ));
        }

        let last_block_time = platform.state.last_block_time_ms().ok_or(Error::Execution(
            ExecutionError::StateNotInitialized(
                "expected a last platform block during identity update validation",
            ),
        ))?;

        Ok(ConsensusValidationResult::new_with_data(
            IdentityCreditWithdrawalTransitionAction::from_identity_credit_withdrawal(
                self,
                last_block_time,
            )
            .into(),
        ))
    }
}
