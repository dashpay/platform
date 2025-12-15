use crate::consensus::basic::overflow_error::OverflowError;
use crate::consensus::basic::BasicError;
use crate::consensus::state::identity::IdentityInsufficientBalanceError;
use crate::consensus::ConsensusError;
use crate::fee::Credits;
use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use crate::state_transition::batch_transition::BatchTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionIdentityEstimatedFeeValidation,
    StateTransitionOwned,
};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for BatchTransition {
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        Ok(platform_version
            .fee_version
            .state_transition_min_fees
            .document_batch_sub_transition
            .saturating_mul(self.transitions_len() as u64))
    }
}

impl StateTransitionIdentityEstimatedFeeValidation for BatchTransition {
    fn validate_estimated_fee(
        &self,
        identity_known_balance: Credits,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let purchases_amount = match self.all_document_purchases_amount() {
            Ok(purchase_amount) => purchase_amount.unwrap_or_default(),
            Err(ProtocolError::Overflow(e)) => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::OverflowError(OverflowError::new(
                        e.to_owned(),
                    ))),
                ))
            }
            // Other errors shouldn't happen, but if they do, treat as zero
            Err(_) => 0,
        };

        // If we added documents that had a conflicting index we need to put up a collateral that voters can draw on
        let conflicting_indices_collateral_amount =
            match self.all_conflicting_index_collateral_voting_funds() {
                Ok(collateral_amount) => collateral_amount.unwrap_or_default(),
                Err(ProtocolError::Overflow(e)) => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(BasicError::OverflowError(OverflowError::new(
                            e.to_owned(),
                        ))),
                    ))
                }
                // Other errors shouldn't happen, but if they do, treat as zero
                Err(_) => 0,
            };

        let base_fees = self.calculate_min_required_fee(platform_version)?;

        // This is just the needed balance to pass this validation step, most likely the actual fees are smaller
        let needed_balance = purchases_amount
            .saturating_add(conflicting_indices_collateral_amount)
            .saturating_add(base_fees);

        if identity_known_balance < needed_balance {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(
                    self.owner_id(),
                    identity_known_balance,
                    needed_balance,
                )
                .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
