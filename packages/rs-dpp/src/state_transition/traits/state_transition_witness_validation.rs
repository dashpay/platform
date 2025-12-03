use crate::address_funds::AddressWitnessVerificationOperations;
use crate::consensus::signature::InvalidStateTransitionSignatureError;
use crate::serialization::Signable;
use crate::state_transition::StateTransitionWitnessSigned;
use crate::validation::SimpleConsensusValidationResult;

/// Result of witness validation, containing both the validation result and the operations performed.
pub struct WitnessValidationResult {
    /// The consensus validation result (empty on success, contains errors on failure)
    pub validation_result: SimpleConsensusValidationResult,
    /// The operations performed during validation (for fee calculation)
    pub operations: AddressWitnessVerificationOperations,
}

impl WitnessValidationResult {
    /// Create a new result with the given validation result and operations
    pub fn new(
        validation_result: SimpleConsensusValidationResult,
        operations: AddressWitnessVerificationOperations,
    ) -> Self {
        Self {
            validation_result,
            operations,
        }
    }

    /// Create a new error result with no operations tracked
    pub fn new_with_error(error: crate::consensus::ConsensusError) -> Self {
        Self {
            validation_result: SimpleConsensusValidationResult::new_with_error(error),
            operations: AddressWitnessVerificationOperations::new(),
        }
    }
}

/// Trait for validating input witnesses against signable bytes.
///
/// This trait is implemented by state transitions that have inputs and input_witnesses,
/// where each input address must have a corresponding valid witness (signature).
pub trait StateTransitionWitnessValidation: StateTransitionWitnessSigned + Signable {
    /// Validates that all input witnesses are valid for the given signable bytes.
    ///
    /// This method verifies that:
    /// 1. The number of witnesses matches the number of inputs
    /// 2. Each witness correctly signs for its corresponding input address
    ///
    /// # Arguments
    /// * `signable_bytes` - The bytes that were signed (typically from `state_transition.signable_bytes()`)
    ///
    /// # Returns
    /// * `WitnessValidationResult` - Contains validation result and operations performed
    fn validate_witnesses(&self, signable_bytes: &[u8]) -> WitnessValidationResult {
        let mut total_operations = AddressWitnessVerificationOperations::new();

        // Validate each witness against its corresponding input address
        for (i, (address, witness)) in self
            .inputs()
            .keys()
            .zip(self.witnesses().iter())
            .enumerate()
        {
            match address.verify_bytes_against_witness(witness, signable_bytes) {
                Ok(operations) => {
                    total_operations.combine(&operations);
                }
                Err(e) => {
                    return WitnessValidationResult::new_with_error(
                        InvalidStateTransitionSignatureError::new(format!(
                            "Witness {} verification failed: {}",
                            i, e
                        ))
                        .into(),
                    );
                }
            }
        }

        WitnessValidationResult::new(SimpleConsensusValidationResult::new(), total_operations)
    }
}
