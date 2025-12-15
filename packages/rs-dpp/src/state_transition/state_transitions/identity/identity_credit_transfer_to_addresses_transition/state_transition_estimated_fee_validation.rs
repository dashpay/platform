use crate::consensus::state::identity::IdentityInsufficientBalanceError;
use crate::fee::Credits;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;
use crate::state_transition::{
    StateTransitionEstimatedFeeValidation, StateTransitionIdentityEstimatedFeeValidation,
};
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionEstimatedFeeValidation for IdentityCreditTransferToAddressesTransition {
    fn calculate_estimated_fee(&self, platform_version: &PlatformVersion) -> Credits {
        let min_fees = &platform_version.fee_version.state_transition_min_fees;
        let output_count = self.recipient_addresses().len() as u64;
        min_fees.credit_transfer_to_addresses.saturating_add(
            min_fees
                .address_funds_transfer_output_cost
                .saturating_mul(output_count),
        )
    }
}

impl StateTransitionIdentityEstimatedFeeValidation for IdentityCreditTransferToAddressesTransition {
    fn validate_estimated_fee(
        &self,
        identity_known_balance: Credits,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let amount = self
            .recipient_addresses()
            .values()
            .fold(0u64, |acc, &val| acc.saturating_add(val));

        let required_fee = self.calculate_estimated_fee(platform_version);
        let outbound_amount = amount.saturating_add(required_fee);

        if identity_known_balance < outbound_amount {
            return SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(
                    self.identity_id(),
                    identity_known_balance,
                    outbound_amount,
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}
