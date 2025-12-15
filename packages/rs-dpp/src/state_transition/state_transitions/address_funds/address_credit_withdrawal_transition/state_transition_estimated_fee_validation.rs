use crate::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::AddressNonce;
use crate::state_transition::address_credit_withdrawal_transition::accessors::AddressCreditWithdrawalTransitionAccessorsV0;
use crate::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use crate::state_transition::{
    StateTransitionAddressEstimatedFeeValidation, StateTransitionAddressesFeeStrategy,
    StateTransitionEstimatedFeeValidation, StateTransitionWitnessSigned,
};
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl StateTransitionEstimatedFeeValidation for AddressCreditWithdrawalTransition {
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        let min_fees = &platform_version.fee_version.state_transition_min_fees;
        let input_count = self.inputs().len();
        let output_count = if self.output().is_some() { 1 } else { 0 };
        Ok(min_fees
            .address_credit_withdrawal
            .saturating_add(
                min_fees
                    .address_funds_transfer_input_cost
                    .saturating_mul(input_count as u64),
            )
            .saturating_add(
                min_fees
                    .address_funds_transfer_output_cost
                    .saturating_mul(output_count),
            ))
    }
}

impl StateTransitionAddressEstimatedFeeValidation for AddressCreditWithdrawalTransition {
    fn calculate_amount_available(
        &self,
        remaining_balances: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> Credits {
        let mut amount = 0u64;
        for step in self.fee_strategy() {
            match step {
                AddressFundsFeeStrategyStep::DeductFromInput(index) => {
                    if let Some((_, (_, credits))) = remaining_balances.iter().nth(*index as usize)
                    {
                        amount = amount.saturating_add(*credits);
                    }
                }
                AddressFundsFeeStrategyStep::ReduceOutput(_index) => {
                    if let Some((_, credits)) = self.output() {
                        amount = amount.saturating_add(*credits);
                    }
                }
            }
        }
        amount
    }
}
