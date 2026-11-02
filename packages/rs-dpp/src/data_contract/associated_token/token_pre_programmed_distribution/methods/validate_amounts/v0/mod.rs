use crate::balances::credits::TokenAmount;
use crate::consensus::basic::data_contract::PreProgrammedDistributionAmountOverLimitError;
use crate::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use crate::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use crate::data_contract::TokenContractPosition;
use crate::validation::SimpleConsensusValidationResult;

impl TokenPreProgrammedDistribution {
    #[inline(always)]
    pub(super) fn validate_amounts_v0(
        &self,
        token_position: TokenContractPosition,
    ) -> SimpleConsensusValidationResult {
        for (timestamp, release) in self.distributions() {
            // A single amount over the limit puts the total over it too, so the total is the
            // only thing to check. A total that does not even fit a `u64` is over the limit.
            let total_fits = release
                .values()
                .try_fold(0 as TokenAmount, |total, amount| total.checked_add(*amount))
                .is_some_and(|total| total <= i64::MAX as TokenAmount);

            if !total_fits {
                return SimpleConsensusValidationResult::new_with_error(
                    PreProgrammedDistributionAmountOverLimitError::new(token_position, *timestamp)
                        .into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::codes::ErrorWithCode;
    use crate::consensus::ConsensusError;
    use crate::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use crate::prelude::TimestampMillis;
    use assert_matches::assert_matches;
    use platform_value::Identifier;
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    const MAX_AMOUNT: TokenAmount = i64::MAX as TokenAmount;

    fn distribution(
        releases: impl IntoIterator<Item = (TimestampMillis, Vec<TokenAmount>)>,
    ) -> TokenPreProgrammedDistribution {
        let distributions = releases
            .into_iter()
            .map(|(timestamp, amounts)| {
                let release = amounts
                    .into_iter()
                    .enumerate()
                    .map(|(recipient, amount)| {
                        (Identifier::from([recipient as u8 + 1; 32]), amount)
                    })
                    .collect::<BTreeMap<_, _>>();
                (timestamp, release)
            })
            .collect();
        TokenPreProgrammedDistribution::V0(TokenPreProgrammedDistributionV0 { distributions })
    }

    #[test]
    fn should_accept_releases_whose_totals_fit() {
        let platform_version = PlatformVersion::latest();

        let result = distribution([
            (100, vec![MAX_AMOUNT]),
            (200, vec![MAX_AMOUNT - 1, 1]),
            (300, vec![]),
        ])
        .validate_amounts(0, platform_version)
        .expect("expected to validate");

        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
    }

    #[test]
    fn should_reject_an_amount_over_the_limit() {
        let platform_version = PlatformVersion::latest();

        let result = distribution([(100, vec![5]), (200, vec![MAX_AMOUNT + 1])])
            .validate_amounts(3, platform_version)
            .expect("expected to validate");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::PreProgrammedDistributionAmountOverLimitError(e)
            )] if e.token_position() == 3 && e.timestamp() == 200
        );
        assert_eq!(result.errors[0].code(), 10277);
    }

    #[test]
    fn should_reject_a_release_whose_amounts_total_over_the_limit() {
        let platform_version = PlatformVersion::latest();

        let result = distribution([(100, vec![MAX_AMOUNT, 1])])
            .validate_amounts(0, platform_version)
            .expect("expected to validate");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::PreProgrammedDistributionAmountOverLimitError(e)
            )] if e.token_position() == 0 && e.timestamp() == 100
        );
    }

    #[test]
    fn should_reject_a_release_whose_total_does_not_fit_a_u64() {
        let platform_version = PlatformVersion::latest();

        let result = distribution([(100, vec![u64::MAX, u64::MAX])])
            .validate_amounts(0, platform_version)
            .expect("expected to validate");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::PreProgrammedDistributionAmountOverLimitError(_)
            )]
        );
    }

    #[test]
    fn should_judge_every_release_on_its_own_total() {
        let platform_version = PlatformVersion::latest();

        // Releases are separate sum trees, so two full releases do not add up.
        let result = distribution([(100, vec![MAX_AMOUNT]), (200, vec![MAX_AMOUNT])])
            .validate_amounts(0, platform_version)
            .expect("expected to validate");

        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
    }
}
