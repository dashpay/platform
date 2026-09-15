use crate::consensus::basic::data_contract::TokenShieldedPoolIncompatibleRulesError;
use crate::consensus::basic::unsupported_version_error::UnsupportedVersionError;
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Getters;
use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::data_contract::associated_token::token_configuration::v1::TokenConfigurationV1;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::ChangeControlRules;
use crate::data_contract::TokenContractPosition;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::validation::SimpleConsensusValidationResult;
use bincode::{Decode, DecodeUntrusted, Encode};
use derive_more::From;
use platform_version::version::PlatformVersion;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt;

pub mod accessors;
mod methods;
pub mod v0;
pub mod v1;

#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(
    Serialize, Deserialize, Encode, Decode, Debug, Clone, PartialEq, Eq, From, DecodeUntrusted,
)]
#[serde(tag = "$formatVersion")]
pub enum TokenConfiguration {
    #[serde(rename = "0")]
    V0(TokenConfigurationV0),
    /// V0 plus the per-token shielded pool opt-in. Admitted from protocol version 14
    /// (`token_versions.token_configuration_format`).
    #[serde(rename = "1")]
    V1(TokenConfigurationV1),
}
impl TokenConfiguration {
    pub fn as_cow_v0(&self) -> Cow<'_, TokenConfigurationV0> {
        match self {
            TokenConfiguration::V0(v0) => Cow::Borrowed(v0),
            TokenConfiguration::V1(v1) => Cow::Borrowed(&v1.base),
        }
    }

    /// The `$formatVersion` this configuration is serialized with.
    pub fn format_version(&self) -> u16 {
        match self {
            TokenConfiguration::V0(_) => 0,
            TokenConfiguration::V1(_) => 1,
        }
    }

    /// Checks the configuration's format version against the bounds the platform version
    /// admits (`dpp.contract_versions.token_versions.token_configuration_format`).
    ///
    /// A format above the bound is a consensus error, not a decode failure: nodes running
    /// software that knows the newer variant must still refuse it until the protocol version
    /// that introduces it activates, so a mixed-version network agrees.
    pub fn validate_format_version(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let bounds = &platform_version
            .dpp
            .contract_versions
            .token_versions
            .token_configuration_format;
        let format_version = self.format_version();
        if format_version < bounds.min_version || format_version > bounds.max_version {
            SimpleConsensusValidationResult::new_with_error(
                UnsupportedVersionError::new(
                    format_version,
                    bounds.min_version,
                    bounds.max_version,
                )
                .into(),
            )
        } else {
            SimpleConsensusValidationResult::new()
        }
    }

    /// A shielded pool makes freezing and confiscation unenforceable: shielded notes belong to
    /// no identity account, so a holder who expects a freeze simply shields first. Rather than
    /// let an issuer advertise controls that only cover transparent balances, a token with
    /// `hasShieldedPool` must permanently disable `freezeRules`, `unfreezeRules` and
    /// `destroyFrozenFundsRules`: no one may take the action and no one may administer the
    /// rule, so no configuration update can ever switch them on. Checked on contract create
    /// and update; the flag itself is immutable.
    pub fn validate_shielded_pool_rules(
        &self,
        token_contract_position: TokenContractPosition,
    ) -> SimpleConsensusValidationResult {
        if !self.has_shielded_pool() {
            return SimpleConsensusValidationResult::new();
        }
        let rules: [(&ChangeControlRules, &str); 3] = [
            (self.freeze_rules(), "freezeRules"),
            (self.unfreeze_rules(), "unfreezeRules"),
            (self.destroy_frozen_funds_rules(), "destroyFrozenFundsRules"),
        ];
        for (rule, name) in rules {
            let disabled = *rule.authorized_to_make_change_action_takers()
                == AuthorizedActionTakers::NoOne
                && *rule.admin_action_takers() == AuthorizedActionTakers::NoOne;
            if !disabled {
                return SimpleConsensusValidationResult::new_with_error(
                    TokenShieldedPoolIncompatibleRulesError::new(
                        token_contract_position,
                        name.to_string(),
                    )
                    .into(),
                );
            }
        }
        SimpleConsensusValidationResult::new()
    }
}

impl fmt::Display for TokenConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenConfiguration::V0(v0) => write!(f, "{}", v0),
            TokenConfiguration::V1(v1) => write!(f, "{}", v1),
        }
    }
}

#[cfg(all(test, feature = "json-conversion"))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;

    #[test]
    fn token_configuration_large_supply_json_round_trip() {
        let mut config = TokenConfigurationV0::default_most_restrictive();
        config.base_supply = u64::MAX;
        let config = TokenConfiguration::V0(config);

        let json = config.to_json().expect("to_json should succeed");

        // u64::MAX > JS MAX_SAFE_INTEGER, so it should be serialized as a string
        assert!(
            json["baseSupply"].is_string(),
            "baseSupply should be a string for large values, got: {:?}",
            json["baseSupply"]
        );
        assert_eq!(json["baseSupply"].as_str().unwrap(), u64::MAX.to_string());

        let restored = TokenConfiguration::from_json(json).expect("from_json should succeed");
        assert_eq!(config, restored);
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;

    /// `default_most_restrictive` already populates ~25 inner fields with
    /// non-default values (decimals=8, base_supply=100_000, etc.) — exactly
    /// what we want for the round-trip structural check below.
    fn fixture() -> TokenConfiguration {
        TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive())
    }

    /// Tier 3: TokenConfiguration embeds ~25 fields, several of which are
    /// themselves versioned enums (TokenConfigurationConvention,
    /// ChangeControlRules x7, TokenKeepsHistoryRules, TokenDistributionRules,
    /// TokenMarketplaceRules). An inline wire-shape literal would be 200+
    /// lines and would re-test the nested types' own assertions. Instead we
    /// assert only the envelope (top-level keys + `$formatVersion`) and trust
    /// the nested types' tests for inner shape correctness.
    #[test]
    fn json_round_trip_with_envelope_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Envelope check: format version + top-level keys present.
        assert_eq!(
            json.get("$formatVersion").and_then(|v| v.as_str()),
            Some("0")
        );
        for key in [
            "conventions",
            "conventionsChangeRules",
            "baseSupply",
            "maxSupply",
            "keepsHistory",
            "startAsPaused",
            "allowTransferToFrozenBalance",
            "maxSupplyChangeRules",
            "distributionRules",
            "marketplaceRules",
            "manualMintingRules",
            "manualBurningRules",
            "freezeRules",
            "unfreezeRules",
            "destroyFrozenFundsRules",
            "emergencyActionRules",
            "mainControlGroup",
            "mainControlGroupCanBeModified",
            "description",
        ] {
            assert!(
                json.get(key).is_some(),
                "expected top-level key {:?} in JSON envelope",
                key
            );
        }
        let recovered = TokenConfiguration::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_envelope_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Same envelope-only check on the platform_value side.
        let map = value.as_map().expect("value is a Map");
        let has_key = |k: &str| {
            map.iter()
                .any(|(key, _)| matches!(key, platform_value::Value::Text(t) if t == k))
        };
        assert!(has_key("$formatVersion"));
        for key in [
            "conventions",
            "conventionsChangeRules",
            "baseSupply",
            "maxSupply",
            "keepsHistory",
            "startAsPaused",
            "allowTransferToFrozenBalance",
            "maxSupplyChangeRules",
            "distributionRules",
            "marketplaceRules",
            "manualMintingRules",
            "manualBurningRules",
            "freezeRules",
            "unfreezeRules",
            "destroyFrozenFundsRules",
            "emergencyActionRules",
            "mainControlGroup",
            "mainControlGroupCanBeModified",
            "description",
        ] {
            assert!(
                has_key(key),
                "expected top-level key {:?} in Value envelope",
                key
            );
        }
        let recovered = TokenConfiguration::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    mod shielded_pool_rules {
        use super::*;
        use crate::consensus::basic::BasicError;
        use crate::consensus::ConsensusError;
        use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
        use crate::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
        use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
        use crate::prelude::Identifier;

        fn rules(
            authorized: AuthorizedActionTakers,
            admin: AuthorizedActionTakers,
        ) -> ChangeControlRules {
            ChangeControlRules::V0(ChangeControlRulesV0 {
                authorized_to_make_change: authorized,
                admin_action_takers: admin,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            })
        }

        fn pooled_configuration() -> TokenConfiguration {
            let mut configuration =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            configuration.set_has_shielded_pool(true);
            configuration
        }

        fn rejected_rule(configuration: &TokenConfiguration) -> Option<String> {
            let result = configuration.validate_shielded_pool_rules(3);
            match result.errors.as_slice() {
                [] => None,
                [ConsensusError::BasicError(
                    BasicError::TokenShieldedPoolIncompatibleRulesError(error),
                )] => {
                    assert_eq!(error.token_contract_position(), 3);
                    Some(error.rule().to_string())
                }
                other => panic!("unexpected errors: {other:?}"),
            }
        }

        #[test]
        fn most_restrictive_defaults_are_compatible_with_a_pool() {
            assert_eq!(rejected_rule(&pooled_configuration()), None);
        }

        #[test]
        fn a_token_without_a_pool_may_keep_freeze_rules() {
            let mut configuration =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            configuration.set_freeze_rules(rules(
                AuthorizedActionTakers::ContractOwner,
                AuthorizedActionTakers::ContractOwner,
            ));
            assert_eq!(rejected_rule(&configuration), None);
        }

        #[test]
        fn a_pooled_token_rejects_an_authorized_freezer() {
            let mut configuration = pooled_configuration();
            configuration.set_freeze_rules(rules(
                AuthorizedActionTakers::ContractOwner,
                AuthorizedActionTakers::NoOne,
            ));
            assert_eq!(
                rejected_rule(&configuration),
                Some("freezeRules".to_string())
            );
        }

        #[test]
        fn a_pooled_token_rejects_an_admin_who_could_enable_unfreezing_later() {
            let mut configuration = pooled_configuration();
            configuration.set_unfreeze_rules(rules(
                AuthorizedActionTakers::NoOne,
                AuthorizedActionTakers::MainGroup,
            ));
            assert_eq!(
                rejected_rule(&configuration),
                Some("unfreezeRules".to_string())
            );
        }

        #[test]
        fn a_pooled_token_rejects_frozen_funds_destruction() {
            let mut configuration = pooled_configuration();
            configuration.set_destroy_frozen_funds_rules(rules(
                AuthorizedActionTakers::Identity(Identifier::from([1u8; 32])),
                AuthorizedActionTakers::NoOne,
            ));
            assert_eq!(
                rejected_rule(&configuration),
                Some("destroyFrozenFundsRules".to_string())
            );
        }
    }
}
