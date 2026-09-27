mod v0;
mod v1;

use crate::consensus::basic::unsupported_version_error::UnsupportedVersionError;
use crate::consensus::basic::BasicError;
use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for AddressCreditWithdrawalTransitionV0 {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match platform_version
            .dpp
            .state_transitions
            .address_funds
            .validate_credit_withdrawal_structure
        {
            0 => self.validate_structure_v0(platform_version),
            1 => self.validate_structure_v1(platform_version),
            version => SimpleConsensusValidationResult::new_with_error(
                BasicError::UnsupportedVersionError(UnsupportedVersionError::new(version, 0, 1))
                    .into(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
    use crate::consensus::ConsensusError;
    use crate::identity::core_script::CoreScript;
    use crate::withdrawal::{core_fee_in_credits, Pooling};
    use assert_matches::assert_matches;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    /// Helper: returns a valid credit withdrawal transition that passes all validations.
    /// Uses an input large enough to cover the minimum withdrawal amount.
    fn valid_withdrawal_transition() -> AddressCreditWithdrawalTransitionV0 {
        let mut inputs = BTreeMap::new();
        // Need at least the v12 min_withdrawal_amount (1_000_000) for a valid withdrawal;
        // use 2_000_000 to sit comfortably above the floor rather than on its boundary.
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (0, 2_000_000));

        AddressCreditWithdrawalTransitionV0 {
            inputs,
            output: None,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            core_fee_per_byte: 1, // 1 is a fibonacci number
            pooling: Pooling::Never,
            output_script: CoreScript::new_p2pkh([5u8; 20]),
            user_fee_increase: 0,
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            }],
        }
    }

    #[test]
    fn should_validate_a_valid_transition() {
        let platform_version = PlatformVersion::latest();
        let transition = valid_withdrawal_transition();
        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid result, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_return_invalid_if_no_inputs() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.inputs.clear();
        transition.input_witnesses.clear();

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::TransitionNoInputsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_too_many_inputs() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        let max = platform_version.dpp.state_transitions.max_address_inputs;
        transition.inputs.clear();
        transition.input_witnesses.clear();
        for i in 0..=(max as usize) {
            let mut hash = [0u8; 20];
            hash[0] = (i & 0xFF) as u8;
            hash[1] = ((i >> 8) & 0xFF) as u8;
            transition
                .inputs
                .insert(PlatformAddress::P2pkh(hash), (0, 1_000_000));
            transition.input_witnesses.push(AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            });
        }

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::TransitionOverMaxInputsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_witness_count_mismatch() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.input_witnesses.clear();

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InputWitnessCountMismatchError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_output_address_also_input() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        let shared_addr = PlatformAddress::P2pkh([1u8; 20]);
        transition.output = Some((shared_addr, 500_000));

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::OutputAddressAlsoInputError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_fee_strategy_empty() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.fee_strategy.clear();

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyEmptyError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_fee_strategy_too_many_steps() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        let max = platform_version
            .dpp
            .state_transitions
            .max_address_fee_strategies;
        transition.fee_strategy.clear();
        transition.inputs.clear();
        transition.input_witnesses.clear();
        for i in 0..=(max as usize) {
            let mut hash = [0u8; 20];
            hash[0] = (i & 0xFF) as u8;
            hash[1] = ((i >> 8) & 0xFF) as u8;
            transition
                .inputs
                .insert(PlatformAddress::P2pkh(hash), (0, 1_000_000));
            transition.input_witnesses.push(AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            });
            transition
                .fee_strategy
                .push(AddressFundsFeeStrategyStep::DeductFromInput(i as u16));
        }

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyTooManyStepsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_fee_strategy_has_duplicates() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.fee_strategy = vec![
            AddressFundsFeeStrategyStep::DeductFromInput(0),
            AddressFundsFeeStrategyStep::DeductFromInput(0),
        ];

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyDuplicateError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_deduct_from_input_index_out_of_bounds() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(99)];

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyIndexOutOfBoundsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_reduce_output_index_out_of_bounds_with_no_output() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.output = None;
        transition.fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyIndexOutOfBoundsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_input_below_minimum() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0, 1));

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InputBelowMinimumError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_output_below_minimum() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        // Need enough input to cover output + withdrawal amount
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0, 10_000_000));
        transition.output = Some((PlatformAddress::P2pkh([9u8; 20]), 1));

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::OutputBelowMinimumError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_pooling_not_never() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.pooling = Pooling::IfAvailable;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::NotImplementedCreditWithdrawalTransitionPoolingError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_core_fee_not_fibonacci() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.core_fee_per_byte = 4;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
            )]
        );
    }

    #[test]
    fn should_accept_valid_fibonacci_core_fees() {
        let platform_version = PlatformVersion::latest();
        let fibonacci_numbers = [1, 2, 3, 5, 8, 13, 21];
        for fee in fibonacci_numbers {
            let mut transition = valid_withdrawal_transition();
            // Large enough to leave the v14 floor above the Core fee at every rate in the sweep.
            transition
                .inputs
                .insert(PlatformAddress::P2pkh([1u8; 20]), (0, 10_000_000));
            transition.core_fee_per_byte = fee;
            let result = transition.validate_structure(platform_version);
            assert!(
                result.is_valid(),
                "Expected valid for fibonacci fee {}, got errors: {:?}",
                fee,
                result.errors
            );
        }
    }

    #[test]
    fn should_return_invalid_if_output_script_not_p2pkh_or_p2sh() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.output_script = CoreScript::from_bytes(vec![0x00, 0x14, 0x01, 0x02]);

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionOutputScriptError(_)
            )]
        );
    }

    #[test]
    fn should_accept_valid_p2sh_output_script() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.output_script = CoreScript::new_p2sh([7u8; 20]);

        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid with P2SH output script, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_return_invalid_result_if_input_sum_overflows() {
        let platform_version = PlatformVersion::latest();

        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (0, u64::MAX));
        inputs.insert(PlatformAddress::P2pkh([2u8; 20]), (0, u64::MAX));

        let transition = AddressCreditWithdrawalTransitionV0 {
            inputs,
            input_witnesses: vec![
                AddressWitness::P2pkh {
                    signature: vec![0u8; 65].into(),
                },
                AddressWitness::P2pkh {
                    signature: vec![0u8; 65].into(),
                },
            ],
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            core_fee_per_byte: 1,
            output_script: CoreScript::random_p2pkh(&mut rand::rngs::StdRng::seed_from_u64(1)),
            ..Default::default()
        };

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::OverflowError(_))]
        );
    }

    #[test]
    fn should_return_invalid_if_withdrawal_amount_below_minimum() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        // Set input below the v12 min_withdrawal_amount (1_000_000) so the resulting
        // withdrawal is rejected as too small.
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0, 100_000));

        let result = transition.validate_structure(platform_version);
        // 100_000 withdrawal < min_withdrawal_amount (1_000_000), should fail
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::WithdrawalBelowMinAmountError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_input_sum_not_greater_than_output() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        // Set input and output equal -- no withdrawal amount
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0, 1_000_000));
        transition.output = Some((PlatformAddress::P2pkh([9u8; 20]), 1_000_000));

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::WithdrawalBalanceMismatchError(_)
            )]
        );
    }

    #[test]
    fn should_validate_with_output_change() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        // Large enough input to cover both output change and min withdrawal
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0, 5_000_000));
        transition.output = Some((PlatformAddress::P2pkh([9u8; 20]), 500_000));
        transition.fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid with output change, got errors: {:?}",
            result.errors
        );
    }

    fn transition(amount: u64, core_fee_per_byte: u32) -> AddressCreditWithdrawalTransitionV0 {
        AddressCreditWithdrawalTransitionV0 {
            inputs: BTreeMap::from([(PlatformAddress::P2pkh([1; 20]), (0, amount))]),
            output: None,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            core_fee_per_byte,
            pooling: Pooling::Never,
            output_script: CoreScript::new_p2pkh([5; 20]),
            user_fee_increase: 0,
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: vec![0; 65].into(),
            }],
        }
    }

    #[test]
    fn protocol_version_14_adds_the_core_fee_to_the_minimum() {
        let version_13 = PlatformVersion::get(13).expect("protocol version 13");
        let version_14 = PlatformVersion::get(14).expect("protocol version 14");
        let at_the_old_minimum = transition(version_13.system_limits.min_withdrawal_amount, 1);

        assert!(at_the_old_minimum.validate_structure(version_13).is_valid());
        assert_matches!(
            at_the_old_minimum
                .validate_structure(version_14)
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::WithdrawalBelowMinAmountError(_)
            )]
        );

        let above_the_fee = transition(
            version_14.system_limits.min_withdrawal_amount
                + core_fee_in_credits(1).expect("core fee"),
            1,
        );
        assert!(above_the_fee.validate_structure(version_14).is_valid());
    }

    #[test]
    fn protocol_version_14_caps_the_core_fee_rate() {
        let version_13 = PlatformVersion::get(13).expect("protocol version 13");
        let version_14 = PlatformVersion::get(14).expect("protocol version 14");
        let over_the_cap = transition(3_000_000_000, 10_946);

        assert!(over_the_cap.validate_structure(version_13).is_valid());
        assert_matches!(
            over_the_cap
                .validate_structure(version_14)
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
            )]
        );
    }
}
