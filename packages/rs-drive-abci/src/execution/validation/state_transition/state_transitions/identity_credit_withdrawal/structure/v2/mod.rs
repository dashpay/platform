use dpp::consensus::basic::identity::InvalidIdentityCreditWithdrawalTransitionAmountError;
use dpp::consensus::ConsensusError;

use super::v1::IdentityCreditWithdrawalStateTransitionStructureValidationV1;
use crate::error::Error;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use dpp::withdrawal::{min_withdrawal_amount_with_core_fee, validate_core_fee_per_byte_cap};

pub(in crate::execution::validation::state_transition::state_transitions::identity_credit_withdrawal) trait IdentityCreditWithdrawalStateTransitionStructureValidationV2 {
    fn validate_basic_structure_v2(&self, platform_version: &PlatformVersion) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditWithdrawalStateTransitionStructureValidationV2
    for IdentityCreditWithdrawalTransition
{
    /// The v1 rules plus the protocol version 14 cap on `core_fee_per_byte` and a minimum that
    /// leaves `min_withdrawal_amount` above the Core fee, since from v14 that fee is carved out
    /// of the withdrawn amount instead of being drawn from the Core credit pool on top of it.
    fn validate_basic_structure_v2(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let result = self.validate_basic_structure_v1(platform_version)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_core_fee_per_byte_cap(self.core_fee_per_byte(), platform_version);
        if !result.is_valid() {
            return Ok(result);
        }

        let minimum_amount =
            min_withdrawal_amount_with_core_fee(self.core_fee_per_byte(), platform_version);
        if self.amount() < minimum_amount {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::from(InvalidIdentityCreditWithdrawalTransitionAmountError::new(
                    self.amount(),
                    minimum_amount,
                    platform_version.system_limits.max_withdrawal_amount,
                )),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
    use assert_matches::assert_matches;
    use dpp::consensus::basic::BasicError;
    use dpp::dashcore::Network;
    use dpp::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
    use dpp::withdrawal::{core_fee_in_credits, Pooling};

    fn transition(amount: u64, core_fee_per_byte: u32) -> IdentityCreditWithdrawalTransition {
        IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
            identity_id: Default::default(),
            amount,
            core_fee_per_byte,
            pooling: Pooling::Never,
            output_script: None,
            nonce: 0,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }

    #[test]
    fn should_enforce_core_fee_limit_and_output_floor() {
        let platform_version = PlatformVersion::latest();

        assert_matches!(
            transition(3_000_000, 10_946)
                .validate_basic_structure_v2(platform_version)
                .expect("validation")
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
            )]
        );
        assert_matches!(
            transition(1_286_349_999, 6_765)
                .validate_basic_structure_v2(platform_version)
                .expect("validation")
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidIdentityCreditWithdrawalTransitionAmountError(_)
            )]
        );
        assert!(transition(1_286_350_000, 6_765)
            .validate_basic_structure_v2(platform_version)
            .expect("validation")
            .is_valid());
    }

    #[test]
    fn should_report_an_out_of_range_amount_once() {
        assert_matches!(
            transition(1, 1)
                .validate_basic_structure_v2(PlatformVersion::latest())
                .expect("validation")
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidIdentityCreditWithdrawalTransitionAmountError(_)
            )]
        );
    }

    #[test]
    fn protocol_version_14_adds_the_core_fee_to_the_minimum_through_the_dispatcher() {
        let version_13 = PlatformVersion::get(13).expect("protocol version 13");
        let version_14 = PlatformVersion::get(14).expect("protocol version 14");
        let at_the_old_minimum = transition(version_13.system_limits.min_withdrawal_amount, 1);

        assert!(at_the_old_minimum
            .validate_basic_structure(Network::Testnet, version_13)
            .expect("validation")
            .is_valid());
        assert_matches!(
            at_the_old_minimum
                .validate_basic_structure(Network::Testnet, version_14)
                .expect("validation")
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidIdentityCreditWithdrawalTransitionAmountError(_)
            )]
        );

        let above_the_fee = transition(
            version_14.system_limits.min_withdrawal_amount
                + core_fee_in_credits(1).expect("core fee"),
            1,
        );
        assert!(above_the_fee
            .validate_basic_structure(Network::Testnet, version_14)
            .expect("validation")
            .is_valid());
    }
}
