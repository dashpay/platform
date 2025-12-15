use crate::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::AddressNonce;
use crate::state_transition::address_funds_transfer_transition::accessors::AddressFundsTransferTransitionAccessorsV0;
use crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use crate::state_transition::{
    StateTransitionAddressEstimatedFeeValidation, StateTransitionAddressesFeeStrategy,
    StateTransitionEstimatedFeeValidation, StateTransitionWitnessSigned,
};
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl StateTransitionEstimatedFeeValidation for AddressFundsTransferTransition {
    fn calculate_min_required_fee(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        let min_fees = &platform_version.fee_version.state_transition_min_fees;
        let input_count = self.inputs().len();
        let output_count = self.outputs().len().max(1);
        Ok(min_fees
            .address_funds_transfer_input_cost
            .saturating_mul(input_count as u64)
            .saturating_add(
                min_fees
                    .address_funds_transfer_output_cost
                    .saturating_mul(output_count as u64),
            ))
    }
}

impl StateTransitionAddressEstimatedFeeValidation for AddressFundsTransferTransition {
    fn calculate_amount_available(
        &self,
        remaining_balances: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> Credits {
        let mut amount = 0u64;
        let outputs: Vec<Credits> = self.outputs().values().copied().collect();
        for step in self.fee_strategy() {
            match step {
                AddressFundsFeeStrategyStep::DeductFromInput(index) => {
                    if let Some((_, (_, credits))) = remaining_balances.iter().nth(*index as usize)
                    {
                        amount = amount.saturating_add(*credits);
                    }
                }
                AddressFundsFeeStrategyStep::ReduceOutput(index) => {
                    if let Some(credits) = outputs.get(*index as usize) {
                        amount = amount.saturating_add(*credits);
                    }
                }
            }
        }
        amount
    }
}
