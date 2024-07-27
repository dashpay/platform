use crate::error::Error;
use dpp::consensus::basic::overflow_error::OverflowError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::consensus::ConsensusError;
use dpp::identity::PartialIdentity;
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::ProtocolError;

use dpp::validation::SimpleConsensusValidationResult;

use crate::error::execution::ExecutionError;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions) trait DocumentsBatchTransitionBalanceValidationV0
{
    fn validate_advanced_minimum_balance_pre_check_v0(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentsBatchTransitionBalanceValidationV0 for DocumentsBatchTransition {
    fn validate_advanced_minimum_balance_pre_check_v0(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let balance =
            identity
                .balance
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "expected to have a balance on identity for documents batch transition",
                )))?;

        let purchases_amount = match self.all_purchases_amount() {
            Ok(purchase_amount) => purchase_amount.unwrap_or_default(),
            Err(ProtocolError::Overflow(e)) => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::OverflowError(OverflowError::new(
                        e.to_owned(),
                    ))),
                ))
            }
            Err(e) => return Err(e.into()),
        };

        // If we added documents that had a conflicting index we need to put up a collateral that voters can draw on

        let conflicting_indices_collateral_amount =
            match self.all_conflicting_index_collateral_voting_funds() {
                Ok(conflicting_indices_collateral_amount) => {
                    conflicting_indices_collateral_amount.unwrap_or_default()
                }
                Err(ProtocolError::Overflow(e)) => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(BasicError::OverflowError(OverflowError::new(
                            e.to_owned(),
                        ))),
                    ))
                }
                Err(e) => return Err(e.into()),
            };

        let base_fees = match platform_version.fee_version.state_transition_min_fees.document_batch_sub_transition.checked_mul(self.transitions().len() as u64) {
            None => return Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::BasicError(BasicError::OverflowError(OverflowError::new("overflow when multiplying base fee and amount of sub transitions in documents batch transition".to_string()))))),
            Some(base_fees) => base_fees
        };

        // This is just the needed balance to pass this validation step, most likely the actual fees are smaller
        let needed_balance = match purchases_amount
            .checked_add(conflicting_indices_collateral_amount).and_then(|added| added.checked_add(base_fees)) {
            None => return Ok(SimpleConsensusValidationResult::new_with_error(ConsensusError::BasicError(BasicError::OverflowError(OverflowError::new("overflow when adding all purchases amount with conflicting_indices_collateral_amounts and base fees in documents batch transition".to_string()))))),
            Some(needed_balance) => needed_balance
        };

        if balance < needed_balance {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(identity.id, balance, needed_balance).into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
