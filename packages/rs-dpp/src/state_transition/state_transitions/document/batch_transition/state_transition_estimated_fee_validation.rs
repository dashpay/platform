use crate::consensus::basic::overflow_error::OverflowError;
use crate::consensus::basic::BasicError;
use crate::consensus::state::identity::IdentityInsufficientBalanceError;
use crate::consensus::ConsensusError;
use crate::fee::Credits;
use crate::shielded::compute_shielded_verification_fee;
use crate::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use crate::state_transition::batch_transition::document_base_transition::v1::v1_methods::DocumentBaseTransitionV1Methods;
use crate::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use crate::state_transition::batch_transition::BatchTransition;
use crate::state_transition::StateTransitionHasUserFeeIncrease;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionIdentityEstimatedFeeValidation,
    StateTransitionOwned,
};
use crate::tokens::gas_fees_paid_by::GasFeesPaidBy;
use crate::tokens::token_payment_info::v0::v0_accessors::TokenPaymentInfoAccessorsV0;
use crate::tokens::token_payment_info::v1::v1_accessors::TokenPaymentInfoAccessorsV1;
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
        let base_fees = self.calculate_min_required_fee(platform_version)?;
        self.validate_estimated_principal_and_fees(identity_known_balance, base_fees)
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
        self.validate_estimated_principal_and_fees(identity_known_balance, 0)
    }

    /// The principal a sponsored batch's signer funds, plus the compute fee of the Orchard
    /// bundles it carries.
    ///
    /// A sponsor pays the gas of a batch that executes, but a sub-transition its state
    /// validation replaces with a nonce bump takes the sponsor off the whole batch, and the
    /// signer pays for the work that ran on it. So the signer has to hold what the verification
    /// costs even while a sponsor is expected to pay it: whichever way the batch ends, somebody
    /// can be charged for the proofs already verified. A batch carrying no bundle adds nothing
    /// and is asked for its principal alone, as before.
    pub fn validate_estimated_principal_with_shielded_compute(
        &self,
        identity_known_balance: Credits,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let compute_fee = self.shielded_bundle_compute_fee(platform_version)?;
        // A batch carrying a bundle is held to the floor an unsponsored one meets, because the
        // sponsor is not who pays when it fails: a sub-transition replaced by a nonce bump takes
        // the sponsor off the batch, and the signer then owes the verification that ran together
        // with the signature, the contract reads, the pool reads and the bump itself. Asking for
        // the whole floor inherits that floor's own guarantee — an unsponsored batch meeting it
        // can be charged — instead of guessing at a bound for the failed event. A batch carrying
        // no bundle has no such work to pay for and keeps the exemption: its signer funds the
        // principal and fee validation judges the gas against whoever ends up paying.
        let base_fees = if compute_fee == 0 {
            0
        } else {
            self.calculate_min_required_fee(platform_version)?
                .saturating_add(compute_fee)
        };
        self.validate_estimated_principal_and_fees(
            identity_known_balance,
            self.fees_raised_by_the_chosen_increase(base_fees),
        )
    }

    /// The shielded compute fee the batch's Orchard bundles will be charged: one bundle
    /// verification plus the per-action work, summed over the sub-transitions that carry a
    /// bundle, each priced exactly as the transformer that builds it prices it.
    ///
    /// A bundle is carried either by a token transition against a shielded pool or by a
    /// document whose token cost is paid out of the payment token's pool.
    pub fn shielded_bundle_compute_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        let mut total: Credits = 0;
        for transition in self.transitions_iter() {
            let actions = match transition {
                BatchedTransitionRef::Token(token_transition) => {
                    token_transition.shielded_pool_actions()
                }
                BatchedTransitionRef::Document(document_transition) => document_transition
                    .base()
                    .token_payment_info_ref()
                    .as_ref()
                    .and_then(|info| info.shielded_payment())
                    .map(|payment| payment.actions.as_slice()),
            };
            let Some(actions) = actions else {
                continue;
            };
            total = total.saturating_add(compute_shielded_verification_fee(
                actions.len(),
                platform_version,
            )?);
        }
        Ok(total)
    }

    /// The minimum balance pre-check of a batch, reserving the shielded compute fee of the
    /// bundles it carries on top of the flat per-sub-transition minimum.
    ///
    /// The flat minimum is orders of magnitude below what one bundle costs, and reserving only
    /// that leaves the Halo 2 verification unfunded. A claim into a pool makes the gap
    /// exploitable: its claimable amount is only known against state, so its proof is skipped
    /// in check tx, and a signer holding only the flat minimum passes the mempool without any
    /// verification running. Every validator then runs it inside block validation, where the
    /// concurrency permit that guards admission does not apply, and the batch is refused
    /// unpaid — nothing charged, the identity contract nonce untouched — so the same bytes
    /// replay. Reserving the fee here refuses the batch before the proof work is done, and
    /// refuses nothing that could have been executed: every bundle-carrying sub-transition is
    /// charged this same fee as its action is built, before anything about it can fail, so a
    /// signer who cannot cover it is refused by fee validation in any case.
    ///
    /// That last claim rests on a number, not on the structure. The floor asks for
    /// `flat_minimum + compute_fee` where fee validation asks for `metered_fee + compute_fee`,
    /// so the floor refuses somebody fee validation would not exactly when
    /// `metered_fee < flat_minimum`. A sub-transition carrying a bundle writes notes and
    /// nullifiers, whose metered storage alone prices it two orders of magnitude above the
    /// flat minimum, so no such batch exists. Anything that lets a bundle ride on a
    /// sub-transition cheaper than the flat minimum reopens that band; the same reasoning is
    /// recorded on `SystemLimits::max_transitions_in_documents_batch`, which is where the
    /// shape of a batch is decided.
    ///
    /// The caller decides whether to ask for it. A batch that asks the contract owner to pay
    /// its gas is not asked, because the compute fee is gas.
    pub fn validate_estimated_fee_with_shielded_compute(
        &self,
        identity_known_balance: Credits,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let base_fees = self
            .calculate_min_required_fee(platform_version)?
            .saturating_add(self.shielded_bundle_compute_fee(platform_version)?);
        self.validate_estimated_principal_and_fees(
            identity_known_balance,
            self.fees_raised_by_the_chosen_increase(base_fees),
        )
    }

    /// `base_fees` raised by the percentage the signer asked for.
    ///
    /// Settlement raises the processing fee that way and then judges the balance against the
    /// raised figure, so a floor that ignored the increase would admit a signer settlement
    /// cannot charge. Raised exactly as `FeeResult::apply_user_fee_increase` raises it, and only
    /// over the fee: a document purchase and a contested collateral are amounts a signer owes,
    /// not fees, and settlement does not raise them either.
    ///
    /// Only the generation the unreleased protocol version 14 selects asks for this. The
    /// generation every released version selects reaches
    /// `validate_estimated_principal_and_fees` directly and keeps the threshold and the
    /// `required_balance` it has always reported.
    fn fees_raised_by_the_chosen_increase(&self, base_fees: Credits) -> Credits {
        base_fees.saturating_add(
            ((base_fees as u128)
                .saturating_mul(self.user_fee_increase() as u128)
                .saturating_div(100))
            .min(Credits::MAX as u128) as Credits,
        )
    }

    /// The balance has to cover the principal of the batch (document purchases and the
    /// collateral of contested creates) plus `base_fees`.
    fn validate_estimated_principal_and_fees(
        &self,
        identity_known_balance: Credits,
        base_fees: Credits,
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
