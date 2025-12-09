use crate::consensus::basic::state_transition::{
    OutputBelowMinimumError, TransitionNoOutputsError, TransitionOverMaxOutputsError,
};
use crate::consensus::basic::BasicError;
use crate::state_transition::identity_credit_transfer_to_addresses_transition::v0::IdentityCreditTransferToAddressesTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for IdentityCreditTransferToAddressesTransitionV0 {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // Validate at least one recipient address
        if self.recipient_addresses.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::TransitionNoOutputsError(TransitionNoOutputsError::new()).into(),
            );
        }

        // Validate maximum recipient addresses
        if self.recipient_addresses.len()
            > platform_version.dpp.state_transitions.max_address_outputs as usize
        {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::TransitionOverMaxOutputsError(TransitionOverMaxOutputsError::new(
                    self.recipient_addresses.len().min(u16::MAX as usize) as u16,
                    platform_version.dpp.state_transitions.max_address_outputs,
                ))
                .into(),
            );
        }

        let min_output_amount = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_output_amount;

        // Validate each recipient address amount is at least min_output_amount
        for amount in self.recipient_addresses.values() {
            if *amount < min_output_amount {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::OutputBelowMinimumError(OutputBelowMinimumError::new(
                        *amount,
                        min_output_amount,
                    ))
                    .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }
}
