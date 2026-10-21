use crate::consensus::basic::state_transition::WithdrawalBelowMinAmountError;
use crate::consensus::basic::BasicError;
use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
use crate::validation::SimpleConsensusValidationResult;
use crate::withdrawal::{min_withdrawal_amount_with_core_fee, validate_core_fee_per_byte_cap};
use platform_version::version::PlatformVersion;

impl AddressCreditWithdrawalTransitionV0 {
    /// Stateless rules from protocol version 14: the v0 rules plus a cap on `core_fee_per_byte`
    /// and a minimum that leaves `min_withdrawal_amount` above the Core fee. From v14 the Core
    /// fee is carved out of the withdrawn amount instead of being drawn from the Core credit pool
    /// on top of it, so the amount has to cover both.
    pub(super) fn validate_structure_v1(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let result = self.validate_structure_v0(platform_version);
        if !result.is_valid() {
            return result;
        }

        let result = validate_core_fee_per_byte_cap(self.core_fee_per_byte, platform_version);
        if !result.is_valid() {
            return result;
        }

        // v0 rejected an input sum that overflows and an output that is not below the inputs,
        // so the withdrawn amount is exact here. Saturating keeps the rejecting direction should
        // that ever change.
        let input_sum = self
            .inputs
            .values()
            .fold(0u64, |acc, (_, amount)| acc.saturating_add(*amount));
        let output_amount = self.output.as_ref().map_or(0, |(_, amount)| *amount);
        let withdrawal_amount = input_sum.saturating_sub(output_amount);
        let min_withdrawal_amount =
            min_withdrawal_amount_with_core_fee(self.core_fee_per_byte, platform_version);
        if withdrawal_amount < min_withdrawal_amount {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::WithdrawalBelowMinAmountError(WithdrawalBelowMinAmountError::new(
                    withdrawal_amount,
                    min_withdrawal_amount,
                    platform_version.system_limits.max_withdrawal_amount,
                ))
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
    use crate::consensus::ConsensusError;
    use crate::identity::core_script::CoreScript;
    use crate::state_transition::StateTransitionStructureValidation;
    use crate::withdrawal::Pooling;
    use assert_matches::assert_matches;
    use std::collections::BTreeMap;

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
    fn should_enforce_core_fee_limit_and_output_floor() {
        let platform_version = PlatformVersion::latest();

        assert_matches!(
            transition(3_000_000, 10_946)
                .validate_structure(platform_version)
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
            )]
        );
        assert_matches!(
            transition(1_286_349_999, 6_765)
                .validate_structure(platform_version)
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::WithdrawalBelowMinAmountError(_)
            )]
        );
        assert!(transition(1_286_350_000, 6_765)
            .validate_structure(platform_version)
            .is_valid());
    }

    #[test]
    fn should_keep_the_v0_rules() {
        let platform_version = PlatformVersion::latest();
        let mut no_witness = transition(2_000_000, 1);
        no_witness.input_witnesses.clear();

        assert_matches!(
            no_witness
                .validate_structure(platform_version)
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::InputWitnessCountMismatchError(_)
            )]
        );
    }
}
