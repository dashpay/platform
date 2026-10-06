use crate::consensus::basic::data_contract::TokenShieldedPoolIncompatibleRulesError;
use crate::consensus::basic::unsupported_version_error::UnsupportedVersionError;
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Getters;
use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use crate::data_contract::associated_token::token_configuration::v1::TokenConfigurationV1;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::ChangeControlRules;
use crate::data_contract::errors::DataContractError;
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
use std::collections::BTreeMap;
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

    /// The pool's outgoing notes threshold may not exceed
    /// `max_token_pool_notes_for_outgoing`: a threshold the pool never reaches would refuse
    /// every outflow and strand every shielded balance, irreversibly on a readonly contract.
    /// Checked on contract create and update and on the configuration a `TokenConfigUpdate`
    /// proposes.
    pub fn validate_minimum_pool_notes_for_outgoing(
        &self,
        token_contract_position: TokenContractPosition,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        validate_minimum_pool_notes_for_outgoing_bound(
            self.minimum_pool_notes_for_outgoing(),
            token_contract_position,
            platform_version,
        )
    }
}

/// The bound on a token shielded pool's outgoing notes threshold: at most
/// `max_token_pool_notes_for_outgoing`, refused as `KeyWrongBounds`. Shared by the token
/// configuration validation and the `TokenConfigUpdate` that changes the threshold.
pub fn validate_minimum_pool_notes_for_outgoing_bound(
    minimum_pool_notes: u64,
    token_contract_position: TokenContractPosition,
    platform_version: &PlatformVersion,
) -> SimpleConsensusValidationResult {
    let max_minimum_pool_notes = platform_version
        .system_limits
        .max_token_pool_notes_for_outgoing;
    if minimum_pool_notes > max_minimum_pool_notes {
        return SimpleConsensusValidationResult::new_with_error(
            DataContractError::KeyWrongBounds(format!(
                "token at position {token_contract_position}: minimumPoolNotesForOutgoing \
                 {minimum_pool_notes} is above the maximum {max_minimum_pool_notes}"
            ))
            .into(),
        );
    }
    SimpleConsensusValidationResult::new()
}

/// Validates every token configuration of a contract for `platform_version`: the format
/// version must be admitted, a pooled token's rules must be compatible with a pool and its
/// outgoing notes threshold within bounds. Returns the first error. Shared by the contract
/// create and update basic structure generations and by the pre-activation gate, so the three
/// cannot drift.
pub fn validate_token_configurations(
    tokens: &BTreeMap<TokenContractPosition, TokenConfiguration>,
    platform_version: &PlatformVersion,
) -> SimpleConsensusValidationResult {
    for (position, configuration) in tokens {
        let result = configuration.validate_format_version(platform_version);
        if !result.is_valid() {
            return result;
        }
        let result = configuration.validate_shielded_pool_rules(*position);
        if !result.is_valid() {
            return result;
        }
        let result =
            configuration.validate_minimum_pool_notes_for_outgoing(*position, platform_version);
        if !result.is_valid() {
            return result;
        }
    }
    SimpleConsensusValidationResult::new()
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

#[cfg(test)]
mod minimum_pool_notes_tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::codes::ErrorWithCode;
    use crate::consensus::ConsensusError;
    use crate::data_contract::associated_token::token_configuration::accessors::v0::{
        TokenConfigurationV0Getters, TokenConfigurationV0Setters,
    };
    use crate::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
    use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
    use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use crate::group::action_taker::{ActionGoal, ActionTaker};
    use crate::prelude::Identifier;

    fn pooled() -> TokenConfiguration {
        let mut configuration =
            TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
        configuration.set_has_shielded_pool(true);
        configuration
    }

    fn pooled_with_threshold(minimum_pool_notes: u64) -> TokenConfiguration {
        let mut configuration = pooled();
        let TokenConfiguration::V1(v1) = &mut configuration else {
            panic!("a pooled configuration is V1");
        };
        v1.minimum_pool_notes_for_outgoing = Some(minimum_pool_notes);
        configuration
    }

    fn rules(authorized: AuthorizedActionTakers) -> ChangeControlRules {
        ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: authorized,
            admin_action_takers: authorized,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        })
    }

    #[test]
    fn should_read_a_configuration_without_a_threshold_as_zero() {
        assert_eq!(
            TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive())
                .minimum_pool_notes_for_outgoing(),
            0
        );
        let configuration = pooled();
        let TokenConfiguration::V1(v1) = &configuration else {
            panic!("a pooled configuration is V1");
        };
        assert_eq!(v1.minimum_pool_notes_for_outgoing, None);
        assert_eq!(configuration.minimum_pool_notes_for_outgoing(), 0);
        // Nobody may change a threshold the issuer did not open to change.
        assert_eq!(
            configuration.authorized_action_takers_for_configuration_item(
                &TokenConfigurationChangeItem::MinimumPoolNotesForOutgoing(1)
            ),
            AuthorizedActionTakers::NoOne
        );
    }

    #[test]
    fn should_bound_the_threshold_by_the_system_limit_with_error_10241() {
        let platform_version = PlatformVersion::latest();
        let max = platform_version
            .system_limits
            .max_token_pool_notes_for_outgoing;
        assert_eq!(max, 250);

        let at_the_limit = BTreeMap::from([(0, pooled_with_threshold(max))]);
        let result = validate_token_configurations(&at_the_limit, platform_version);
        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);

        let over_the_limit = BTreeMap::from([(3, pooled_with_threshold(max + 1))]);
        let result = validate_token_configurations(&over_the_limit, platform_version);
        assert_matches::assert_matches!(
            result.errors.as_slice(),
            [error @ ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::KeyWrongBounds(message)
            ))] if error.code() == 10241 && message.contains("position 3")
        );
    }

    #[test]
    fn should_govern_the_threshold_by_its_own_rules_on_a_pooled_token_only() {
        let owner = Identifier::from([1; 32]);
        let other = Identifier::from([2; 32]);
        let groups = BTreeMap::new();
        let change = TokenConfigurationChangeItem::MinimumPoolNotesForOutgoing(12);
        let can_apply = |configuration: &TokenConfiguration, action_taker: Identifier| {
            configuration.can_apply_token_configuration_item(
                &change,
                &owner,
                None,
                &groups,
                &ActionTaker::SingleIdentity(action_taker),
                ActionGoal::ActionCompletion,
            )
        };

        let mut governed = pooled();
        let TokenConfiguration::V1(v1) = &mut governed else {
            panic!("a pooled configuration is V1");
        };
        v1.minimum_pool_notes_for_outgoing_change_rules =
            rules(AuthorizedActionTakers::ContractOwner);
        assert!(can_apply(&governed, owner));
        assert!(!can_apply(&governed, other));
        assert_eq!(
            governed.controlling_action_takers_for_configuration_item(&change),
            AuthorizedActionTakers::ContractOwner
        );
        governed.apply_token_configuration_item(change.clone());
        assert_eq!(governed.minimum_pool_notes_for_outgoing(), 12);

        // A token without a pool has no threshold, whoever asks and whatever its other rules.
        let mut unpooled = TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
        unpooled.set_max_supply_change_rules(rules(AuthorizedActionTakers::ContractOwner));
        assert!(!can_apply(&unpooled, owner));
        let before = unpooled.clone();
        unpooled.apply_token_configuration_item(change);
        assert_eq!(unpooled, before);
    }

    /// The control item moves who may set the threshold and the admin item moves who
    /// administers that, each under the threshold's own admin rule and neither touching the
    /// other or any rule of the nested V0 configuration.
    #[test]
    fn should_route_the_threshold_control_and_admin_items_to_their_own_rules() {
        let owner = Identifier::from([1; 32]);
        let heir = Identifier::from([3; 32]);
        let groups = BTreeMap::new();
        let can_apply = |configuration: &TokenConfiguration,
                         change: &TokenConfigurationChangeItem,
                         action_taker: Identifier| {
            configuration.can_apply_token_configuration_item(
                change,
                &owner,
                None,
                &groups,
                &ActionTaker::SingleIdentity(action_taker),
                ActionGoal::ActionCompletion,
            )
        };
        let threshold_rules = |configuration: &TokenConfiguration| {
            let TokenConfiguration::V1(v1) = configuration else {
                panic!("a pooled configuration is V1");
            };
            v1.minimum_pool_notes_for_outgoing_change_rules.clone()
        };

        let mut configuration = pooled();
        let TokenConfiguration::V1(v1) = &mut configuration else {
            panic!("a pooled configuration is V1");
        };
        v1.minimum_pool_notes_for_outgoing_change_rules =
            ChangeControlRules::V0(ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                admin_action_takers: AuthorizedActionTakers::ContractOwner,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: true,
            });
        let max_supply_rules_before = configuration.max_supply_change_rules().clone();
        let set_threshold = TokenConfigurationChangeItem::MinimumPoolNotesForOutgoing(8);
        let control = TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingControlGroup(
            AuthorizedActionTakers::Identity(heir),
        );
        let admin = TokenConfigurationChangeItem::MinimumPoolNotesForOutgoingAdminGroup(
            AuthorizedActionTakers::Identity(heir),
        );

        for item in [&control, &admin] {
            assert_eq!(
                configuration.authorized_action_takers_for_configuration_item(item),
                AuthorizedActionTakers::ContractOwner
            );
            assert!(can_apply(&configuration, item, owner));
            assert!(!can_apply(&configuration, item, heir));
        }
        assert!(!can_apply(&configuration, &set_threshold, owner));

        configuration.apply_token_configuration_item(control);
        let rules = threshold_rules(&configuration);
        assert_eq!(
            *rules.authorized_to_make_change_action_takers(),
            AuthorizedActionTakers::Identity(heir)
        );
        assert_eq!(
            *rules.admin_action_takers(),
            AuthorizedActionTakers::ContractOwner
        );
        assert!(can_apply(&configuration, &set_threshold, heir));
        assert!(!can_apply(&configuration, &set_threshold, owner));

        configuration.apply_token_configuration_item(admin);
        let rules = threshold_rules(&configuration);
        assert_eq!(
            *rules.authorized_to_make_change_action_takers(),
            AuthorizedActionTakers::Identity(heir)
        );
        assert_eq!(
            *rules.admin_action_takers(),
            AuthorizedActionTakers::Identity(heir)
        );
        assert_eq!(
            configuration.max_supply_change_rules(),
            &max_supply_rules_before
        );
    }

    #[test]
    fn should_hand_every_other_item_of_a_pooled_token_to_its_v0_rules() {
        let owner = Identifier::from([1; 32]);
        let groups = BTreeMap::new();
        let mut configuration = pooled();
        configuration.set_max_supply_change_rules(rules(AuthorizedActionTakers::ContractOwner));
        let change = TokenConfigurationChangeItem::MaxSupply(Some(5_000));
        assert!(configuration.can_apply_token_configuration_item(
            &change,
            &owner,
            None,
            &groups,
            &ActionTaker::SingleIdentity(owner),
            ActionGoal::ActionCompletion,
        ));
        configuration.apply_token_configuration_item(change);
        assert_eq!(configuration.max_supply(), Some(5_000));
        assert!(configuration.has_shielded_pool());
    }

    #[test]
    fn should_count_the_threshold_rules_among_the_tokens_rules_and_groups() {
        let mut configuration = pooled();
        let TokenConfiguration::V1(v1) = &mut configuration else {
            panic!("a pooled configuration is V1");
        };
        v1.minimum_pool_notes_for_outgoing_change_rules = rules(AuthorizedActionTakers::Group(4));
        let (group_positions, _) = configuration.all_used_group_positions();
        assert!(group_positions.contains(&4));
        assert!(configuration
            .all_change_control_rules()
            .iter()
            .any(
                |(name, rules)| *name == "minimum_pool_notes_for_outgoing_change_rules"
                    && *rules.authorized_to_make_change_action_takers()
                        == AuthorizedActionTakers::Group(4)
            ));
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod minimum_pool_notes_json_tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
    use crate::serialization::JsonConvertible;

    /// A pooled configuration written before the threshold existed, or by a client that leaves
    /// it out, reads with no threshold and no one allowed to change it.
    #[test]
    fn should_read_a_pooled_configuration_without_the_threshold_keys_as_none() {
        let mut configuration =
            TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
        configuration.set_has_shielded_pool(true);
        let mut json = configuration.to_json().expect("to_json");
        let object = json.as_object_mut().expect("configuration object");
        assert!(object.remove("minimumPoolNotesForOutgoing").is_some());
        assert!(object
            .remove("minimumPoolNotesForOutgoingChangeRules")
            .is_some());

        let decoded = TokenConfiguration::from_json(json).expect("from_json");
        assert_eq!(decoded, configuration);
        assert_eq!(decoded.minimum_pool_notes_for_outgoing(), 0);
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod unknown_configuration_key_tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
    use crate::serialization::{JsonConvertible, ValueConvertible};

    /// A V0 configuration's JSON with `hasShieldedPool` bolted on: what a caller writes when
    /// they ask for a pool but leave the format version at 0.
    fn v0_json_asking_for_a_pool() -> serde_json::Value {
        let mut json = TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive())
            .to_json()
            .expect("to_json");
        json.as_object_mut()
            .expect("configuration object")
            .insert("hasShieldedPool".to_string(), serde_json::Value::Bool(true));
        json
    }

    /// The same fixture on the Value wire, which is the path a contract is ingested through.
    fn v0_value_asking_for_a_pool() -> platform_value::Value {
        let mut value = TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive())
            .to_object()
            .expect("to_object");
        value.as_map_mut().expect("configuration map").push((
            platform_value::Value::Text("hasShieldedPool".to_string()),
            platform_value::Value::Bool(true),
        ));
        value
    }

    #[test]
    fn should_refuse_a_pool_asked_for_at_format_version_0_rather_than_drop_it() {
        let json = v0_json_asking_for_a_pool();

        // The control. Take the pool request back out and the very same configuration decodes,
        // at format version 0 and without a pool. It is what makes the refusal below evidence
        // about the pool request rather than about a fixture that stopped building a valid V0.
        let mut without_the_request = json.clone();
        without_the_request
            .as_object_mut()
            .expect("configuration object")
            .remove("hasShieldedPool");
        let control = TokenConfiguration::from_json(without_the_request)
            .expect("the configuration decodes once the pool request is taken out");
        assert_eq!(control.format_version(), 0);
        assert!(!control.has_shielded_pool());

        // A configuration asking for a pool must not decode as a token that can never have one,
        // and the refusal has to name the key it refused: any other message means the decoder
        // tripped over something else and the pool request was never the reason.
        let error = TokenConfiguration::from_json(json).expect_err("the pool request is refused");
        assert!(
            error.to_string().contains("hasShieldedPool"),
            "the refusal must name the key it refused, got {error}"
        );
    }

    #[test]
    fn should_refuse_a_pool_asked_for_at_format_version_0_on_the_value_wire_too() {
        let value = v0_value_asking_for_a_pool();

        let mut without_the_request = value.clone();
        without_the_request
            .as_map_mut()
            .expect("configuration map")
            .retain(|(key, _)| key.as_text() != Some("hasShieldedPool"));
        let control = TokenConfiguration::from_object(without_the_request)
            .expect("the configuration decodes once the pool request is taken out");
        assert_eq!(control.format_version(), 0);
        assert!(!control.has_shielded_pool());

        let error =
            TokenConfiguration::from_object(value).expect_err("the pool request is refused");
        assert!(
            error.to_string().contains("hasShieldedPool"),
            "the refusal must name the key it refused, got {error}"
        );
    }

    /// Asking for a pool moves the configuration to format version 1, whose V0 fields sit at the
    /// top level of the wire through `serde(flatten)`. Both wires have to read that shape back.
    #[test]
    fn should_still_decode_a_pooled_configuration_whose_v0_fields_are_flattened() {
        let mut configuration =
            TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
        configuration.set_has_shielded_pool(true);

        let json = configuration.to_json().expect("to_json");
        assert_eq!(
            json.get("$formatVersion").and_then(|v| v.as_str()),
            Some("1")
        );
        assert_eq!(
            TokenConfiguration::from_json(json).expect("from_json"),
            configuration
        );

        let value = configuration.to_object().expect("to_object");
        assert_eq!(
            TokenConfiguration::from_object(value).expect("from_object"),
            configuration
        );
    }

    /// A key no format version carries is refused at format version 1, as it is at 0.
    ///
    /// Pinned because the failure it prevents is invisible: a dropped key lets a caller's typo
    /// reach the chain as a configuration quietly missing whatever they meant to set, and what
    /// a token can do is fixed when it is created.
    ///
    /// The refusal has to hold on both wires. `TokenConfigurationV1` takes version 0's fields
    /// through `serde(flatten)`, and an unknown key arriving through a flattened field is not
    /// offered to the inner struct at all, so version 0's own refusal cannot reach it — the
    /// attribute has to sit on the version that owns the flatten.
    #[test]
    fn should_refuse_an_unrecognized_key_at_format_version_1() {
        let mut configuration =
            TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
        configuration.set_has_shielded_pool(true);

        const UNRECOGNIZED_KEY: &str = "thisKeyIsInNoFormatVersion";

        let mut json = configuration.to_json().expect("to_json");
        json.as_object_mut()
            .expect("configuration object")
            .insert(UNRECOGNIZED_KEY.to_string(), serde_json::Value::Bool(true));
        let error = TokenConfiguration::from_json(json)
            .expect_err("an unrecognized key must be refused, not dropped")
            .to_string();
        assert!(
            error.contains(UNRECOGNIZED_KEY),
            "the refusal must name the key it refused, got {error}"
        );

        let mut value = configuration.to_object().expect("to_object");
        value.as_map_mut().expect("configuration map").push((
            platform_value::Value::Text(UNRECOGNIZED_KEY.to_string()),
            platform_value::Value::Bool(true),
        ));
        let error = TokenConfiguration::from_object(value)
            .expect_err("the contract ingest path must refuse it too")
            .to_string();
        assert!(
            error.contains(UNRECOGNIZED_KEY),
            "the refusal must name the key it refused, got {error}"
        );

        // The same configuration without the key still decodes, so the refusal is the key's
        // doing and not a fixture that stopped building.
        let clean = TokenConfiguration::from_json(configuration.to_json().expect("to_json"))
            .expect("the configuration itself still decodes");
        assert_eq!(clean, configuration);
        assert_eq!(clean.format_version(), 1);
        assert!(clean.has_shielded_pool());
    }
}
