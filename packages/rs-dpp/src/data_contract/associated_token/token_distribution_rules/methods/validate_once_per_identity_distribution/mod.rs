use crate::consensus::basic::UnsupportedFeatureError;
use crate::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

impl TokenDistributionRules {
    /// Validates the once-per-identity distribution of the rules and the wire format that
    /// carries it.
    ///
    /// Version 1 distribution rules joined the wire at protocol version 14. Older software can
    /// not decode them at all, so while an earlier protocol version is active (`None` in the
    /// version table) they are rejected as unsupported here, keeping new software in agreement
    /// with old software. From protocol version 14 on, version 1 rules must carry a
    /// once-per-identity distribution (version 0 stays the wire format of every token without
    /// one) and its amount must be between 1 and `i64::MAX`.
    pub fn validate_once_per_identity_distribution(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .validation
            .data_contract
            .validate_once_per_identity_distribution
        {
            None => Ok(match self {
                TokenDistributionRules::V0(_) => SimpleConsensusValidationResult::new(),
                TokenDistributionRules::V1(_) => SimpleConsensusValidationResult::new_with_error(
                    UnsupportedFeatureError::new(
                        "version 1 token distribution rules".to_string(),
                        platform_version.protocol_version,
                    )
                    .into(),
                ),
            }),
            Some(0) => Ok(self.validate_once_per_identity_distribution_v0(platform_version)),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_once_per_identity_distribution".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use crate::data_contract::associated_token::token_distribution_rules::accessors::v1::TokenDistributionRulesV1Setters;
    use crate::data_contract::associated_token::token_distribution_rules::v1::TokenDistributionRulesV1;
    use crate::data_contract::associated_token::token_once_per_identity_distribution::v0::TokenOncePerIdentityDistributionV0;
    use crate::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;

    fn v0_rules() -> TokenDistributionRules {
        TokenConfigurationV0::default_most_restrictive().distribution_rules
    }

    fn v1_rules(amount: u64) -> TokenDistributionRules {
        let mut rules = v0_rules();
        rules.set_once_per_identity_distribution(Some(TokenOncePerIdentityDistribution::V0(
            TokenOncePerIdentityDistributionV0 { amount },
        )));
        rules
    }

    fn first_basic_error(result: SimpleConsensusValidationResult) -> BasicError {
        match result.errors.into_iter().next() {
            Some(ConsensusError::BasicError(error)) => error,
            other => panic!("expected a basic error, got {other:?}"),
        }
    }

    #[test]
    fn should_reject_version_1_rules_before_protocol_version_14() {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        assert!(v0_rules()
            .validate_once_per_identity_distribution(platform_version)
            .expect("expected validation to run")
            .is_valid());
        let result = v1_rules(100)
            .validate_once_per_identity_distribution(platform_version)
            .expect("expected validation to run");
        assert!(matches!(
            first_basic_error(result),
            BasicError::UnsupportedFeatureError(_)
        ));
    }

    #[test]
    fn should_accept_version_0_rules_and_valid_version_1_rules() {
        let platform_version = PlatformVersion::latest();
        for rules in [v0_rules(), v1_rules(1), v1_rules(i64::MAX as u64)] {
            assert!(rules
                .validate_once_per_identity_distribution(platform_version)
                .expect("expected validation to run")
                .is_valid());
        }
    }

    #[test]
    fn should_reject_version_1_rules_without_a_distribution() {
        let platform_version = PlatformVersion::latest();
        let TokenDistributionRules::V0(v0) = v0_rules() else {
            panic!("expected version 0 distribution rules");
        };
        let rules = TokenDistributionRules::V1(TokenDistributionRulesV1::from(v0));
        let result = rules
            .validate_once_per_identity_distribution(platform_version)
            .expect("expected validation to run");
        assert!(matches!(
            first_basic_error(result),
            BasicError::UnsupportedFeatureError(_)
        ));
    }

    #[test]
    fn should_reject_an_amount_outside_of_1_to_i64_max() {
        let platform_version = PlatformVersion::latest();
        for amount in [0, i64::MAX as u64 + 1] {
            let result = v1_rules(amount)
                .validate_once_per_identity_distribution(platform_version)
                .expect("expected validation to run");
            assert!(matches!(
                first_basic_error(result),
                BasicError::InvalidTokenOncePerIdentityDistributionAmountError(_)
            ));
        }
    }
}
