use crate::address_funds::AddressFundsFeeStrategyStep;
use crate::consensus::basic::overflow_error::OverflowError;
use crate::consensus::basic::state_transition::{
    FeeStrategyDuplicateError, FeeStrategyEmptyError, FeeStrategyIndexOutOfBoundsError,
    FeeStrategyTooManyStepsError, InputBelowMinimumError, InputWitnessCountMismatchError,
    InputsNotLessThanOutputsError, OutputAddressAlsoInputError, OutputBelowMinimumError,
    TransitionNoInputsError, TransitionOverMaxInputsError,
};
use crate::consensus::basic::BasicError;
use crate::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use crate::consensus::state::state_error::StateError;
use crate::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;
use std::collections::HashSet;

impl IdentityCreateFromAddressesTransitionV0 {
    /// Narrow basic-structure check: validates all structural properties of
    /// the transition except for the `input_witnesses` count, and **does NOT**
    /// validate the public-key structure of `public_keys`.
    ///
    /// This is the same surface the server's basic-structure pipeline
    /// exercises. Public-key structure validation is intentionally skipped
    /// here so that submitting an invalid set of public keys reaches
    /// drive-abci's `advanced_structure_v0`, where it attaches a
    /// `BumpAddressInputNoncesAction` (penalty + processing fee) instead of
    /// failing for free at basic-structure. See the NOTE at the bottom of this
    /// function for the full rationale.
    ///
    /// SDK constructors that want to give callers pre-broadcast feedback on
    /// public-key structure problems must call
    /// `IdentityPublicKeyInCreation::validate_identity_public_keys_structure`
    /// directly *in addition to* this method.
    pub fn validate_structure_without_input_witnesses(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // Validate at least one input
        if self.inputs.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::TransitionNoInputsError(TransitionNoInputsError::new()).into(),
            );
        }

        // Validate at least one public key
        if self.public_keys.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::MissingMasterPublicKeyError(
                    crate::consensus::basic::identity::MissingMasterPublicKeyError::new(),
                )
                .into(),
            );
        }

        // Validate maximum inputs
        if self.inputs.len() > platform_version.dpp.state_transitions.max_address_inputs as usize {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::TransitionOverMaxInputsError(TransitionOverMaxInputsError::new(
                    self.inputs.len().min(u16::MAX as usize) as u16,
                    platform_version.dpp.state_transitions.max_address_inputs,
                ))
                .into(),
            );
        }

        // Validate maximum public keys
        if self.public_keys.len()
            > platform_version
                .dpp
                .state_transitions
                .identities
                .max_public_keys_in_creation as usize
        {
            return SimpleConsensusValidationResult::new_with_error(
                StateError::MaxIdentityPublicKeyLimitReachedError(
                    MaxIdentityPublicKeyLimitReachedError::new(
                        platform_version
                            .dpp
                            .state_transitions
                            .identities
                            .max_public_keys_in_creation as usize,
                    ),
                )
                .into(),
            );
        }

        // Validate output address is not also an input address
        if let Some((output_address, _)) = &self.output {
            if self.inputs.contains_key(output_address) {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::OutputAddressAlsoInputError(OutputAddressAlsoInputError::new())
                        .into(),
                );
            }
        }

        // Validate fee strategy is not empty
        if self.fee_strategy.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::FeeStrategyEmptyError(FeeStrategyEmptyError::new()).into(),
            );
        }

        // Validate fee strategy has at most max_address_fee_strategies steps
        let max_fee_strategies = platform_version
            .dpp
            .state_transitions
            .max_address_fee_strategies as usize;
        if self.fee_strategy.len() > max_fee_strategies {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::FeeStrategyTooManyStepsError(FeeStrategyTooManyStepsError::new(
                    self.fee_strategy.len().min(u8::MAX as usize) as u8,
                    max_fee_strategies.min(u8::MAX as usize) as u8,
                ))
                .into(),
            );
        }

        // Validate fee strategy has no duplicates
        let mut seen = HashSet::with_capacity(self.fee_strategy.len());
        for step in &self.fee_strategy {
            if !seen.insert(step) {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::FeeStrategyDuplicateError(FeeStrategyDuplicateError::new()).into(),
                );
            }
        }

        // Calculate number of outputs (0 or 1 for optional output)
        let output_count = if self.output.is_some() { 1 } else { 0 };

        // Validate fee strategy indices are within bounds
        for step in &self.fee_strategy {
            match step {
                AddressFundsFeeStrategyStep::DeductFromInput(index) => {
                    if *index as usize >= self.inputs.len() {
                        return SimpleConsensusValidationResult::new_with_error(
                            BasicError::FeeStrategyIndexOutOfBoundsError(
                                FeeStrategyIndexOutOfBoundsError::new(
                                    "DeductFromInput",
                                    *index,
                                    self.inputs.len().min(u16::MAX as usize) as u16,
                                ),
                            )
                            .into(),
                        );
                    }
                }
                AddressFundsFeeStrategyStep::ReduceOutput(index) => {
                    if *index as usize >= output_count {
                        return SimpleConsensusValidationResult::new_with_error(
                            BasicError::FeeStrategyIndexOutOfBoundsError(
                                FeeStrategyIndexOutOfBoundsError::new(
                                    "ReduceOutput",
                                    *index,
                                    output_count as u16,
                                ),
                            )
                            .into(),
                        );
                    }
                }
            }
        }

        let min_input_amount = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_input_amount;
        let min_output_amount = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_output_amount;
        let min_identity_funding_amount = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_identity_funding_amount;

        // Validate each input is at least min_input_amount
        for (_nonce, amount) in self.inputs.values() {
            if *amount < min_input_amount {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::InputBelowMinimumError(InputBelowMinimumError::new(
                        *amount,
                        min_input_amount,
                    ))
                    .into(),
                );
            }
        }

        // Validate output is at least min_output_amount (if present)
        if let Some((_, amount)) = &self.output {
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

        // Validate inputs >= outputs + min_identity_funding_amount
        let input_sum = self
            .inputs
            .values()
            .try_fold(0u64, |acc, (_, amount)| acc.checked_add(*amount));
        let input_sum = match input_sum {
            Some(sum) => sum,
            None => {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::OverflowError(OverflowError::new("Input sum overflow".to_string()))
                        .into(),
                );
            }
        };

        let output_sum: u64 = self.output.as_ref().map(|(_, amount)| *amount).unwrap_or(0);

        // Check for overflow when adding output_sum + min_identity_funding_amount
        let required_input = match output_sum.checked_add(min_identity_funding_amount) {
            Some(sum) => sum,
            None => {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::OverflowError(OverflowError::new(
                        "Required input calculation overflow".to_string(),
                    ))
                    .into(),
                );
            }
        };

        if input_sum < required_input {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InputsNotLessThanOutputsError(InputsNotLessThanOutputsError::new(
                    input_sum,
                    output_sum,
                    min_identity_funding_amount,
                ))
                .into(),
            );
        }

        // NOTE: Public-key structure validation (counts, duplicates,
        // purpose/security level constraints) is intentionally NOT performed
        // here. On the server, that validation lives in drive-abci's
        // `advanced_structure_v0`, where invalid public keys attach a
        // `BumpAddressInputNoncesAction` with a penalty + processing fee.
        // Running it here on the basic-structure / trait surface would create
        // a free failure mode, because basic-structure failures return only
        // errors with no penalty action. Client-side constructors invoke
        // `IdentityPublicKeyInCreation::validate_identity_public_keys_structure`
        // directly for pre-broadcast feedback.
        SimpleConsensusValidationResult::new()
    }

    /// Validates that the number of `input_witnesses` matches the number of
    /// inputs. Intended to be invoked after signing, once witnesses have been
    /// produced by the signer.
    pub fn validate_input_witnesses_count(&self) -> SimpleConsensusValidationResult {
        if self.inputs.len() != self.input_witnesses.len() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InputWitnessCountMismatchError(InputWitnessCountMismatchError::new(
                    self.inputs.len().min(u16::MAX as usize) as u16,
                    self.input_witnesses.len().min(u16::MAX as usize) as u16,
                ))
                .into(),
            );
        }
        SimpleConsensusValidationResult::new()
    }
}

impl StateTransitionStructureValidation for IdentityCreateFromAddressesTransitionV0 {
    /// Narrow basic-structure validation that mirrors what the server runs at
    /// the basic-structure stage. As documented on
    /// [`IdentityCreateFromAddressesTransitionV0::validate_structure_without_input_witnesses`],
    /// this **does NOT** validate `public_keys` structure: that check is run
    /// later in drive-abci's `advanced_structure_v0` so it can attach a
    /// `BumpAddressInputNoncesAction` penalty. SDK constructors are expected
    /// to call public-key validation separately for pre-broadcast UX.
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let result = self.validate_structure_without_input_witnesses(platform_version);
        if !result.is_valid() {
            return result;
        }
        self.validate_input_witnesses_count()
    }
}
