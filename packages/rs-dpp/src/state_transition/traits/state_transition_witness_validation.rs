use crate::consensus::signature::InvalidStateTransitionSignatureError;
use crate::serialization::Signable;
use crate::state_transition::StateTransitionWitnessSigned;
use crate::validation::SimpleConsensusValidationResult;

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
    /// * `SimpleConsensusValidationResult` - Empty result on success, or errors describing failures
    fn validate_witnesses(&self, signable_bytes: &[u8]) -> SimpleConsensusValidationResult {
        // Validate each witness against its corresponding input address
        for (i, (address, witness)) in self
            .inputs()
            .keys()
            .zip(self.witnesses().iter())
            .enumerate()
        {
            if let Err(e) = address.verify_bytes_against_witness(witness, signable_bytes) {
                return SimpleConsensusValidationResult::new_with_error(
                    InvalidStateTransitionSignatureError::new(format!(
                        "Witness {} verification failed: {}",
                        i, e
                    ))
                    .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }
}
