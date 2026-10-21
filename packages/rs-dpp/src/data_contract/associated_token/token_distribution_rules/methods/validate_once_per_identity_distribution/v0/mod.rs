use crate::consensus::basic::token::InvalidTokenOncePerIdentityDistributionAmountError;
use crate::consensus::basic::UnsupportedFeatureError;
use crate::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
use crate::data_contract::associated_token::token_once_per_identity_distribution::accessors::v0::TokenOncePerIdentityDistributionV0Methods;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl TokenDistributionRules {
    #[inline(always)]
    pub(super) fn validate_once_per_identity_distribution_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let TokenDistributionRules::V1(v1) = self else {
            return SimpleConsensusValidationResult::new();
        };

        // Version 0 stays the wire format of every token without the distribution, so rules
        // that do not use it have exactly one encoding, the one older clients decode.
        let Some(once_per_identity_distribution) = &v1.once_per_identity_distribution else {
            return SimpleConsensusValidationResult::new_with_error(
                UnsupportedFeatureError::new(
                    "version 1 token distribution rules without a once-per-identity distribution"
                        .to_string(),
                    platform_version.protocol_version,
                )
                .into(),
            );
        };

        // A claim mints the amount into a signed balance, so it is capped like the base
        // supply; a zero amount would make every claim a paid no-op.
        let amount = once_per_identity_distribution.amount();
        if amount == 0 || amount > i64::MAX as u64 {
            return SimpleConsensusValidationResult::new_with_error(
                InvalidTokenOncePerIdentityDistributionAmountError::new(amount, i64::MAX as u64)
                    .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}
