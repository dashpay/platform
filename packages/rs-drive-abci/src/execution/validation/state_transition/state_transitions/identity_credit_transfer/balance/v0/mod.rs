use crate::error::Error;
use dpp::balances::credits::MIN_LEFTOVER_CREDITS_BEFORE_PROCESSING;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::signature::IdentityNotFoundError;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::identity::PartialIdentity;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;

use dpp::validation::SimpleConsensusValidationResult;

use crate::platform_types::platform::PlatformStateRef;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

pub(in crate::execution::validation::state_transition::state_transitions) trait IdentityCreditTransferTransitionBalanceValidationV0
{
    fn validate_balance_v0(
        &self,
        identity: Option<&mut PartialIdentity>,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditTransferTransitionBalanceValidationV0 for IdentityCreditTransferTransition {
    fn validate_balance_v0(
        &self,
        identity: Option<&mut PartialIdentity>,
        platform: &PlatformStateRef,
        _unused_block_info: &BlockInfo,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let balance = if let Some(identity) = identity {
            let balance = if let Some(balance) = identity.balance {
                balance
            } else {
                let maybe_existing_identity_balance = platform.drive.fetch_identity_balance(
                    self.identity_id().to_buffer(),
                    tx,
                    platform_version,
                )?;

                let Some(existing_identity_balance) = maybe_existing_identity_balance else {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        IdentityNotFoundError::new(self.identity_id()).into(),
                    ));
                };

                identity.balance = Some(existing_identity_balance);

                existing_identity_balance
            };
            balance
        } else {
            let maybe_existing_identity_balance = platform.drive.fetch_identity_balance(
                self.identity_id().to_buffer(),
                tx,
                platform_version,
            )?;

            let Some(existing_identity_balance) = maybe_existing_identity_balance else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    IdentityNotFoundError::new(self.identity_id()).into(),
                ));
            };

            existing_identity_balance
        };

        if balance < self.amount() + MIN_LEFTOVER_CREDITS_BEFORE_PROCESSING {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(self.identity_id(), balance, self.amount())
                    .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
