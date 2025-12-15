use crate::address_funds::{AddressFundsFeeStrategyStep, PlatformAddress};
use crate::fee::Credits;
use crate::prelude::AddressNonce;
use crate::state_transition::identity_create_from_addresses_transition::accessors::IdentityCreateFromAddressesTransitionAccessorsV0;
use crate::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use crate::state_transition::{
    StateTransitionAddressEstimatedFeeValidation, StateTransitionAddressesFeeStrategy,
    StateTransitionEstimatedFeeValidation, StateTransitionWitnessSigned,
};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl StateTransitionEstimatedFeeValidation for IdentityCreateFromAddressesTransition {
    fn calculate_estimated_fee(&self, platform_version: &PlatformVersion) -> Credits {
        let min_fees = &platform_version.fee_version.state_transition_min_fees;
        let input_count = self.inputs().len();
        let output_count = if self.output().is_some() { 1 } else { 0 };
        let keys_in_creation = self.public_keys().len();
        min_fees
            .identity_create_from_addresses_base_cost
            .saturating_add(
                min_fees
                    .address_funds_transfer_input_cost
                    .saturating_mul(input_count as u64),
            )
            .saturating_add(
                min_fees
                    .address_funds_transfer_output_cost
                    .saturating_mul(output_count),
            )
            .saturating_add(
                min_fees
                    .identity_key_in_creation_cost
                    .saturating_mul(keys_in_creation as u64),
            )
    }
}

impl StateTransitionAddressEstimatedFeeValidation for IdentityCreateFromAddressesTransition {
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
