use crate::consensus::basic::overflow_error::OverflowError;
use crate::consensus::basic::BasicError;
use crate::consensus::state::identity::IdentityInsufficientBalanceError;
use crate::consensus::ConsensusError;
use crate::fee::Credits;
use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use crate::state_transition::batch_transition::document_base_transition::v1::v1_methods::DocumentBaseTransitionV1Methods;
use crate::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use crate::state_transition::batch_transition::BatchTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionIdentityEstimatedFeeValidation,
    StateTransitionOwned,
};
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use crate::tokens::token_payment_info::v0::v0_accessors::TokenPaymentInfoAccessorsV0;
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
            // Other errors shouldn't happen
            Err(e) => return Err(e),
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
                // Other errors shouldn't happen
                Err(e) => return Err(e),
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

impl BatchTransition {
    /// Whether every transition of the batch asks the contract owner to pay its gas: each one is
    /// a document transition whose token payment info requests `ContractOwner` or
    /// `PreferContractOwner`. Only the data contracts can say whether that request is honoured,
    /// so this is what the minimum balance pre-check can know before they are loaded.
    pub fn requests_gas_sponsorship(&self) -> bool {
        let mut any = false;
        for transition in self.transitions_iter() {
            let BatchedTransitionRef::Document(document_transition) = transition else {
                return false;
            };
            let requested = document_transition
                .base()
                .token_payment_info_ref()
                .as_ref()
                .map(|info| info.gas_fees_paid_by())
                .unwrap_or_default();
            if requested == GasFeesPaidBy::DocumentOwner {
                return false;
            }
            any = true;
        }
        any
    }

    /// The minimum balance pre-check of a batch that asks the contract owner to pay its gas:
    /// the document owner still funds the principal (document purchases and the collateral of
    /// contested creates) from their own balance, but not the per-transition fee minimum, which
    /// fee validation then judges against whoever ends up paying.
    pub fn validate_estimated_principal(
        &self,
        identity_known_balance: Credits,
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
            Err(e) => return Err(e),
        };

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
                Err(e) => return Err(e),
            };

        let needed_balance = purchases_amount.saturating_add(conflicting_indices_collateral_amount);

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
