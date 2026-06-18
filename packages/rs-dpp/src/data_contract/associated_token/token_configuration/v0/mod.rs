mod accessors;

use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
use crate::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
use crate::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
use crate::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
use crate::data_contract::associated_token::token_keeps_history_rules::TokenKeepsHistoryRules;
use crate::data_contract::associated_token::token_marketplace_rules::v0::{
    TokenMarketplaceRulesV0, TokenTradeMode,
};
use crate::data_contract::associated_token::token_marketplace_rules::TokenMarketplaceRules;
use crate::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use crate::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use crate::data_contract::change_control_rules::ChangeControlRules;
use crate::data_contract::GroupContractPosition;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Defines the complete configuration for a version 0 token contract.
///
/// `TokenConfigurationV0` encapsulates all metadata, control rules, supply settings,
/// and governance constraints used to initialize and manage a token instance on Platform.
/// This structure serves as the core representation of a token's logic, permissions,
/// and capabilities.
///
/// This configuration is designed to be deterministic and versioned for compatibility
/// across protocol upgrades and validation environments.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationV0 {
    /// Metadata conventions, including decimals and localizations.
    pub conventions: TokenConfigurationConvention,

    /// Change control rules governing who can modify the conventions field.
    #[serde(default = "default_change_control_rules")]
    pub conventions_change_rules: ChangeControlRules,

    /// The initial token supply minted at creation.
    #[serde(default)]
    pub base_supply: TokenAmount,

    /// The maximum allowable supply of the token.
    ///
    /// If `None`, the supply is unbounded unless otherwise constrained by minting logic.
    #[serde(default)]
    pub max_supply: Option<TokenAmount>,

    /// Configuration governing which historical actions are recorded for this token.
    #[serde(default = "default_token_keeps_history_rules")]
    pub keeps_history: TokenKeepsHistoryRules,

    /// Indicates whether the token should start in a paused state.
    ///
    /// When `true`, transfers are disallowed until explicitly unpaused via an emergency action.
    #[serde(default = "default_starts_as_paused")]
    pub start_as_paused: bool,

    /// Allows minting and transferring to frozen token balances if enabled.
    #[serde(default = "default_allow_transfer_to_frozen_balance")]
    pub allow_transfer_to_frozen_balance: bool,

    /// Change control rules for updating the `max_supply`.
    ///
    /// Note: The `max_supply` can never be reduced below the `base_supply`.
    #[serde(default = "default_change_control_rules")]
    pub max_supply_change_rules: ChangeControlRules,

    /// Defines the token's distribution logic, including perpetual and pre-programmed distributions.
    #[serde(default = "default_token_distribution_rules")]
    pub distribution_rules: TokenDistributionRules,

    /// Defines the token's marketplace logic.
    #[serde(default = "default_token_marketplace_rules")]
    pub marketplace_rules: TokenMarketplaceRules,

    /// Rules controlling who is authorized to perform manual minting of tokens.
    #[serde(default = "default_contract_owner_change_control_rules")]
    pub manual_minting_rules: ChangeControlRules,

    /// Rules controlling who is authorized to perform manual burning of tokens.
    #[serde(default = "default_contract_owner_change_control_rules")]
    pub manual_burning_rules: ChangeControlRules,

    /// Rules governing who may freeze token balances.
    #[serde(default = "default_change_control_rules")]
    pub freeze_rules: ChangeControlRules,

    /// Rules governing who may unfreeze token balances.
    #[serde(default = "default_change_control_rules")]
    pub unfreeze_rules: ChangeControlRules,

    /// Rules governing who may destroy frozen funds.
    #[serde(default = "default_change_control_rules")]
    pub destroy_frozen_funds_rules: ChangeControlRules,

    /// Rules governing who may invoke emergency actions, such as pausing transfers.
    #[serde(default = "default_change_control_rules")]
    pub emergency_action_rules: ChangeControlRules,

    /// Optional reference to the group assigned as the token's main control group.
    #[serde(default)]
    pub main_control_group: Option<GroupContractPosition>,

    /// Defines whether and how the main control group assignment may be modified.
    #[serde(default)]
    pub main_control_group_can_be_modified: AuthorizedActionTakers,

    /// Optional textual description of the token's purpose, behavior, or metadata.
    #[serde(default)]
    pub description: Option<String>,
}

// Default function for `keeps_history`
fn default_keeps_history() -> bool {
    true // Default to `true` for keeps_history
}

// Default function for `starts_as_paused`
fn default_starts_as_paused() -> bool {
    false
}

// Default function for `allow_transfer_to_frozen_balance`
fn default_allow_transfer_to_frozen_balance() -> bool {
    true
}

fn default_token_keeps_history_rules() -> TokenKeepsHistoryRules {
    TokenKeepsHistoryRules::V0(TokenKeepsHistoryRulesV0 {
        keeps_transfer_history: true,
        keeps_freezing_history: true,
        keeps_minting_history: true,
        keeps_burning_history: true,
        keeps_direct_pricing_history: true,
        keeps_direct_purchase_history: true,
    })
}

fn default_token_distribution_rules() -> TokenDistributionRules {
    TokenDistributionRules::V0(TokenDistributionRulesV0 {
        perpetual_distribution: None,
        perpetual_distribution_rules: ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::NoOne,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        }),
        pre_programmed_distribution: None,
        new_tokens_destination_identity: None,
        new_tokens_destination_identity_rules: ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::NoOne,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        }),
        minting_allow_choosing_destination: true,
        minting_allow_choosing_destination_rules: ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::NoOne,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        }),
        change_direct_purchase_pricing_rules: ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::NoOne,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        }),
    })
}

fn default_token_marketplace_rules() -> TokenMarketplaceRules {
    TokenMarketplaceRules::V0(TokenMarketplaceRulesV0 {
        trade_mode: TokenTradeMode::NotTradeable,
        trade_mode_change_rules: ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::NoOne,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        }),
    })
}

fn default_change_control_rules() -> ChangeControlRules {
    ChangeControlRules::V0(ChangeControlRulesV0 {
        authorized_to_make_change: AuthorizedActionTakers::NoOne,
        admin_action_takers: AuthorizedActionTakers::NoOne,
        changing_authorized_action_takers_to_no_one_allowed: false,
        changing_admin_action_takers_to_no_one_allowed: false,
        self_changing_admin_action_takers_allowed: false,
    })
}

fn default_contract_owner_change_control_rules() -> ChangeControlRules {
    ChangeControlRules::V0(ChangeControlRulesV0 {
        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
        admin_action_takers: AuthorizedActionTakers::NoOne,
        changing_authorized_action_takers_to_no_one_allowed: false,
        changing_admin_action_takers_to_no_one_allowed: false,
        self_changing_admin_action_takers_allowed: false,
    })
}

impl fmt::Display for TokenConfigurationV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenConfigurationV0 {{\n  conventions: {:?},\n  conventions_change_rules: {:?},\n  base_supply: {},\n  max_supply: {:?},\n  keeps_history: {},\n  start_as_paused: {},\n  allow_transfer_to_frozen_balance: {},\n  max_supply_change_rules: {:?},\n  distribution_rules: {},\n  manual_minting_rules: {:?},\n  manual_burning_rules: {:?},\n  freeze_rules: {:?},\n  unfreeze_rules: {:?},\n  destroy_frozen_funds_rules: {:?},\n  emergency_action_rules: {:?},\n  main_control_group: {:?},\n  main_control_group_can_be_modified: {:?}\n}}",
            self.conventions,
            self.conventions_change_rules,
            self.base_supply,
            self.max_supply,
            self.keeps_history,
            self.start_as_paused,
            self.allow_transfer_to_frozen_balance,
            self.max_supply_change_rules,
            self.distribution_rules,
            self.manual_minting_rules,
            self.manual_burning_rules,
            self.freeze_rules,
            self.unfreeze_rules,
            self.destroy_frozen_funds_rules,
            self.emergency_action_rules,
            self.main_control_group,
            self.main_control_group_can_be_modified
        )
    }
}

/// Represents predefined capability levels for token control presets.
///
/// `TokenConfigurationPresetFeatures` defines a hierarchy of governance capabilities
/// that can be used to initialize rule sets for a token. Each variant enables a specific
/// scope of permitted actions, allowing for simple selection of common governance models.
///
/// These presets are intended to be used in conjunction with `TokenConfigurationPreset`
/// to simplify token setup and enforce governance constraints consistently.
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum TokenConfigurationPresetFeatures {
    /// No actions are permitted after initialization. All governance and control
    /// settings are immutable.
    ///
    /// Suitable for tokens that should remain fixed and tamper-proof.
    MostRestrictive,

    /// Only emergency actions (e.g., pausing the token) are permitted.
    ///
    /// Minting, burning, and advanced operations (such as freezing) are disallowed.
    /// This preset allows minimal control for critical situations without risking
    /// token supply or ownership manipulation.
    WithOnlyEmergencyAction,

    /// Allows minting and burning operations, but not advanced features such as freezing.
    ///
    /// Enables supply management without enabling full administrative capabilities.
    WithMintingAndBurningActions,

    /// Grants the ability to perform advanced actions, including freezing and unfreezing balances.
    ///
    /// Minting and burning are also permitted. Suitable for tokens that require
    /// moderate administrative control without total override capabilities.
    WithAllAdvancedActions,

    /// The action taker is a god, he can do everything, even taking away his own power.
    /// This grants unrestricted control to the action taker, including the ability to revoke
    /// their own permissions or transfer all governance.
    ///
    /// This includes minting, burning, freezing, emergency actions, and full rule modification.
    /// Should only be used with trusted or self-destructible authorities.
    WithExtremeActions,
}

/// A high-level preset representing common configurations for token governance and control.
///
/// `TokenConfigurationPreset` provides a simplified way to initialize a set of
/// predefined token rules (e.g., minting, burning, freezing, emergency actions)
/// by selecting a feature set (`features`) and defining the authorized actor (`action_taker`)
/// responsible for performing allowed actions.
///
/// This abstraction allows users to choose between common control configurations
/// ranging from immutable tokens to fully administrator-controlled assets.
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationPreset {
    /// Defines the set of capabilities enabled in this preset (e.g., whether minting,
    /// burning, freezing, or emergency actions are permitted).
    ///
    /// The selected feature set determines the default rule behavior for all change control
    /// and governance actions within the token configuration.
    pub features: TokenConfigurationPresetFeatures,

    /// The identity or group authorized to perform actions defined by the preset.
    ///
    /// This includes acting as the admin for various rule changes, executing allowed token
    /// operations, or performing emergency control (depending on the selected feature set).
    pub action_taker: AuthorizedActionTakers,
}

// Manual impls because the preset types are flat (not versioned V0/V1).
#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenConfigurationPresetFeatures {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenConfigurationPresetFeatures {}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for TokenConfigurationPreset {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for TokenConfigurationPreset {}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_preset {
    use super::*;
    use crate::serialization::{JsonConvertible, ValueConvertible};
    use platform_value::platform_value;
    use serde_json::json;

    fn fixture() -> TokenConfigurationPreset {
        TokenConfigurationPreset {
            features: TokenConfigurationPresetFeatures::WithAllAdvancedActions,
            action_taker: AuthorizedActionTakers::Group(7),
        }
    }

    #[test]
    fn preset_json_round_trip_with_full_wire_shape() {
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `features` is a unit-only enum (bare PascalCase string);
        // `actionTaker` uses AuthorizedActionTakers' internally-tagged shape.
        // `position` is u16 — JSON erases the size; the value path locks it.
        assert_eq!(
            json,
            json!({
                "features": "WithAllAdvancedActions",
                "actionTaker": {"$type": "group", "position": 7},
            })
        );
        let recovered = TokenConfigurationPreset::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn preset_value_round_trip_with_full_wire_shape() {
        let original = fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "features": "WithAllAdvancedActions",
                "actionTaker": {"$type": "group", "position": 7u16},
            })
        );
        let recovered = TokenConfigurationPreset::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn preset_features_round_trips_all_variants() {
        let cases = [
            (
                TokenConfigurationPresetFeatures::MostRestrictive,
                "MostRestrictive",
            ),
            (
                TokenConfigurationPresetFeatures::WithOnlyEmergencyAction,
                "WithOnlyEmergencyAction",
            ),
            (
                TokenConfigurationPresetFeatures::WithMintingAndBurningActions,
                "WithMintingAndBurningActions",
            ),
            (
                TokenConfigurationPresetFeatures::WithAllAdvancedActions,
                "WithAllAdvancedActions",
            ),
            (
                TokenConfigurationPresetFeatures::WithExtremeActions,
                "WithExtremeActions",
            ),
        ];
        for (original, expected) in cases {
            let json_v = original.to_json().expect("to_json");
            assert_eq!(json_v, json!(expected));
            assert_eq!(
                TokenConfigurationPresetFeatures::from_json(json_v).expect("from_json"),
                original
            );
            let value = original.to_object().expect("to_object");
            assert_eq!(value, platform_value!(expected));
            assert_eq!(
                TokenConfigurationPresetFeatures::from_object(value).expect("from_object"),
                original
            );
        }
    }
}

impl TokenConfigurationPreset {
    pub fn default_main_control_group_can_be_modified(&self) -> AuthorizedActionTakers {
        match self.features {
            TokenConfigurationPresetFeatures::MostRestrictive
            | TokenConfigurationPresetFeatures::WithOnlyEmergencyAction
            | TokenConfigurationPresetFeatures::WithMintingAndBurningActions
            | TokenConfigurationPresetFeatures::WithAllAdvancedActions => {
                AuthorizedActionTakers::NoOne
            }
            TokenConfigurationPresetFeatures::WithExtremeActions => self.action_taker,
        }
    }
    pub fn default_basic_change_control_rules_v0(&self) -> ChangeControlRulesV0 {
        match self.features {
            TokenConfigurationPresetFeatures::MostRestrictive
            | TokenConfigurationPresetFeatures::WithOnlyEmergencyAction => ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                admin_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            },
            TokenConfigurationPresetFeatures::WithMintingAndBurningActions
            | TokenConfigurationPresetFeatures::WithAllAdvancedActions => ChangeControlRulesV0 {
                authorized_to_make_change: self.action_taker,
                admin_action_takers: self.action_taker,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: true,
            },
            TokenConfigurationPresetFeatures::WithExtremeActions => ChangeControlRulesV0 {
                authorized_to_make_change: self.action_taker,
                admin_action_takers: self.action_taker,
                changing_authorized_action_takers_to_no_one_allowed: true,
                changing_admin_action_takers_to_no_one_allowed: true,
                self_changing_admin_action_takers_allowed: true,
            },
        }
    }

    pub fn default_advanced_change_control_rules_v0(&self) -> ChangeControlRulesV0 {
        match self.features {
            TokenConfigurationPresetFeatures::MostRestrictive
            | TokenConfigurationPresetFeatures::WithOnlyEmergencyAction
            | TokenConfigurationPresetFeatures::WithMintingAndBurningActions => {
                ChangeControlRulesV0 {
                    authorized_to_make_change: AuthorizedActionTakers::NoOne,
                    admin_action_takers: AuthorizedActionTakers::NoOne,
                    changing_authorized_action_takers_to_no_one_allowed: false,
                    changing_admin_action_takers_to_no_one_allowed: false,
                    self_changing_admin_action_takers_allowed: false,
                }
            }
            TokenConfigurationPresetFeatures::WithAllAdvancedActions => ChangeControlRulesV0 {
                authorized_to_make_change: self.action_taker,
                admin_action_takers: self.action_taker,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: true,
            },
            TokenConfigurationPresetFeatures::WithExtremeActions => ChangeControlRulesV0 {
                authorized_to_make_change: self.action_taker,
                admin_action_takers: self.action_taker,
                changing_authorized_action_takers_to_no_one_allowed: true,
                changing_admin_action_takers_to_no_one_allowed: true,
                self_changing_admin_action_takers_allowed: true,
            },
        }
    }

    pub fn default_emergency_action_change_control_rules_v0(&self) -> ChangeControlRulesV0 {
        match self.features {
            TokenConfigurationPresetFeatures::MostRestrictive => ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                admin_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            },
            TokenConfigurationPresetFeatures::WithAllAdvancedActions
            | TokenConfigurationPresetFeatures::WithMintingAndBurningActions
            | TokenConfigurationPresetFeatures::WithOnlyEmergencyAction => ChangeControlRulesV0 {
                authorized_to_make_change: self.action_taker,
                admin_action_takers: self.action_taker,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: true,
            },
            TokenConfigurationPresetFeatures::WithExtremeActions => ChangeControlRulesV0 {
                authorized_to_make_change: self.action_taker,
                admin_action_takers: self.action_taker,
                changing_authorized_action_takers_to_no_one_allowed: true,
                changing_admin_action_takers_to_no_one_allowed: true,
                self_changing_admin_action_takers_allowed: true,
            },
        }
    }

    pub fn default_distribution_rules_v0(
        &self,
        perpetual_distribution: Option<TokenPerpetualDistribution>,
        pre_programmed_distribution: Option<TokenPreProgrammedDistribution>,
        with_direct_pricing: bool,
    ) -> TokenDistributionRulesV0 {
        TokenDistributionRulesV0 {
            perpetual_distribution,
            perpetual_distribution_rules: self.default_advanced_change_control_rules_v0().into(),
            pre_programmed_distribution,
            new_tokens_destination_identity: None,
            new_tokens_destination_identity_rules: self
                .default_basic_change_control_rules_v0()
                .into(),
            minting_allow_choosing_destination: true,
            minting_allow_choosing_destination_rules: self
                .default_basic_change_control_rules_v0()
                .into(),
            change_direct_purchase_pricing_rules: if with_direct_pricing {
                self.default_basic_change_control_rules_v0().into()
            } else {
                ChangeControlRulesV0 {
                    authorized_to_make_change: AuthorizedActionTakers::NoOne,
                    admin_action_takers: AuthorizedActionTakers::NoOne,
                    changing_authorized_action_takers_to_no_one_allowed: false,
                    changing_admin_action_takers_to_no_one_allowed: false,
                    self_changing_admin_action_takers_allowed: false,
                }
                .into()
            },
        }
    }

    pub fn default_marketplace_rules_v0(&self) -> TokenMarketplaceRulesV0 {
        TokenMarketplaceRulesV0 {
            trade_mode: TokenTradeMode::NotTradeable,
            trade_mode_change_rules: self.default_basic_change_control_rules_v0().into(),
        }
    }

    pub fn token_configuration_v0(
        &self,
        conventions: TokenConfigurationConvention,
        base_supply: TokenAmount,
        max_supply: Option<TokenAmount>,
        keeps_all_history: bool,
        with_direct_pricing: bool,
    ) -> TokenConfigurationV0 {
        TokenConfigurationV0 {
            conventions,
            conventions_change_rules: self.default_basic_change_control_rules_v0().into(),
            base_supply,
            max_supply,
            keeps_history: TokenKeepsHistoryRulesV0::default_for_keeping_all_history(
                keeps_all_history,
            )
            .into(),
            start_as_paused: false,
            allow_transfer_to_frozen_balance: true,
            max_supply_change_rules: self.default_advanced_change_control_rules_v0().into(),
            distribution_rules: self
                .default_distribution_rules_v0(None, None, with_direct_pricing)
                .into(),
            marketplace_rules: self.default_marketplace_rules_v0().into(),
            manual_minting_rules: self.default_basic_change_control_rules_v0().into(),
            manual_burning_rules: self.default_basic_change_control_rules_v0().into(),
            freeze_rules: self.default_advanced_change_control_rules_v0().into(),
            unfreeze_rules: self.default_advanced_change_control_rules_v0().into(),
            destroy_frozen_funds_rules: self.default_advanced_change_control_rules_v0().into(),
            emergency_action_rules: self
                .default_emergency_action_change_control_rules_v0()
                .into(),
            main_control_group: None,
            main_control_group_can_be_modified: self.default_main_control_group_can_be_modified(),
            description: None,
        }
    }
}

impl TokenConfigurationV0 {
    pub fn default_most_restrictive() -> Self {
        TokenConfigurationPreset {
            features: TokenConfigurationPresetFeatures::MostRestrictive,
            action_taker: AuthorizedActionTakers::NoOne,
        }
        .token_configuration_v0(
            TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
                localizations: Default::default(),
                decimals: 8,
            }),
            100000,
            None,
            true,
            false,
        )
    }

    pub fn with_base_supply(mut self, base_supply: TokenAmount) -> Self {
        self.base_supply = base_supply;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::associated_token::token_configuration::accessors::v0::{
        TokenConfigurationV0Getters, TokenConfigurationV0Setters,
    };
    use platform_value::Identifier;

    fn preset(
        features: TokenConfigurationPresetFeatures,
        action_taker: AuthorizedActionTakers,
    ) -> TokenConfigurationPreset {
        TokenConfigurationPreset {
            features,
            action_taker,
        }
    }

    // --- default_main_control_group_can_be_modified ---

    #[test]
    fn preset_main_control_group_can_be_modified_most_restrictive_is_no_one() {
        let p = preset(
            TokenConfigurationPresetFeatures::MostRestrictive,
            AuthorizedActionTakers::ContractOwner,
        );
        assert_eq!(
            p.default_main_control_group_can_be_modified(),
            AuthorizedActionTakers::NoOne
        );
    }

    #[test]
    fn preset_main_control_group_can_be_modified_only_emergency_is_no_one() {
        let p = preset(
            TokenConfigurationPresetFeatures::WithOnlyEmergencyAction,
            AuthorizedActionTakers::ContractOwner,
        );
        assert_eq!(
            p.default_main_control_group_can_be_modified(),
            AuthorizedActionTakers::NoOne
        );
    }

    #[test]
    fn preset_main_control_group_can_be_modified_minting_burning_is_no_one() {
        let p = preset(
            TokenConfigurationPresetFeatures::WithMintingAndBurningActions,
            AuthorizedActionTakers::ContractOwner,
        );
        assert_eq!(
            p.default_main_control_group_can_be_modified(),
            AuthorizedActionTakers::NoOne
        );
    }

    #[test]
    fn preset_main_control_group_can_be_modified_advanced_is_no_one() {
        let p = preset(
            TokenConfigurationPresetFeatures::WithAllAdvancedActions,
            AuthorizedActionTakers::ContractOwner,
        );
        assert_eq!(
            p.default_main_control_group_can_be_modified(),
            AuthorizedActionTakers::NoOne
        );
    }

    #[test]
    fn preset_main_control_group_can_be_modified_extreme_is_action_taker() {
        let taker = AuthorizedActionTakers::Identity(Identifier::from([9u8; 32]));
        let p = preset(TokenConfigurationPresetFeatures::WithExtremeActions, taker);
        assert_eq!(p.default_main_control_group_can_be_modified(), taker);
    }

    // --- default_basic_change_control_rules_v0 ---

    #[test]
    fn preset_basic_rules_most_restrictive_is_no_one_locked() {
        let p = preset(
            TokenConfigurationPresetFeatures::MostRestrictive,
            AuthorizedActionTakers::ContractOwner,
        );
        let rules = p.default_basic_change_control_rules_v0();
        assert_eq!(
            rules.authorized_to_make_change,
            AuthorizedActionTakers::NoOne
        );
        assert_eq!(rules.admin_action_takers, AuthorizedActionTakers::NoOne);
        assert!(!rules.changing_authorized_action_takers_to_no_one_allowed);
        assert!(!rules.changing_admin_action_takers_to_no_one_allowed);
        assert!(!rules.self_changing_admin_action_takers_allowed);
    }

    #[test]
    fn preset_basic_rules_only_emergency_is_no_one_locked() {
        let p = preset(
            TokenConfigurationPresetFeatures::WithOnlyEmergencyAction,
            AuthorizedActionTakers::ContractOwner,
        );
        let rules = p.default_basic_change_control_rules_v0();
        assert_eq!(
            rules.authorized_to_make_change,
            AuthorizedActionTakers::NoOne
        );
    }

    #[test]
    fn preset_basic_rules_minting_burning_is_action_taker_self_mutable() {
        let taker = AuthorizedActionTakers::ContractOwner;
        let p = preset(
            TokenConfigurationPresetFeatures::WithMintingAndBurningActions,
            taker,
        );
        let rules = p.default_basic_change_control_rules_v0();
        assert_eq!(rules.authorized_to_make_change, taker);
        assert_eq!(rules.admin_action_takers, taker);
        assert!(rules.self_changing_admin_action_takers_allowed);
        // but not to no-one
        assert!(!rules.changing_authorized_action_takers_to_no_one_allowed);
    }

    #[test]
    fn preset_basic_rules_advanced_is_action_taker_self_mutable() {
        let taker = AuthorizedActionTakers::ContractOwner;
        let p = preset(
            TokenConfigurationPresetFeatures::WithAllAdvancedActions,
            taker,
        );
        let rules = p.default_basic_change_control_rules_v0();
        assert_eq!(rules.authorized_to_make_change, taker);
        assert!(rules.self_changing_admin_action_takers_allowed);
        assert!(!rules.changing_admin_action_takers_to_no_one_allowed);
    }

    #[test]
    fn preset_basic_rules_extreme_allows_no_one_transitions() {
        let taker = AuthorizedActionTakers::ContractOwner;
        let p = preset(TokenConfigurationPresetFeatures::WithExtremeActions, taker);
        let rules = p.default_basic_change_control_rules_v0();
        assert_eq!(rules.authorized_to_make_change, taker);
        assert!(rules.changing_authorized_action_takers_to_no_one_allowed);
        assert!(rules.changing_admin_action_takers_to_no_one_allowed);
        assert!(rules.self_changing_admin_action_takers_allowed);
    }

    // --- default_advanced_change_control_rules_v0 ---

    #[test]
    fn preset_advanced_rules_most_restrictive_is_locked() {
        let p = preset(
            TokenConfigurationPresetFeatures::MostRestrictive,
            AuthorizedActionTakers::ContractOwner,
        );
        let rules = p.default_advanced_change_control_rules_v0();
        assert_eq!(
            rules.authorized_to_make_change,
            AuthorizedActionTakers::NoOne
        );
        assert!(!rules.self_changing_admin_action_takers_allowed);
    }

    #[test]
    fn preset_advanced_rules_minting_burning_is_locked() {
        let p = preset(
            TokenConfigurationPresetFeatures::WithMintingAndBurningActions,
            AuthorizedActionTakers::ContractOwner,
        );
        // Minting/burning does NOT open up advanced operations -> advanced remains NoOne
        let rules = p.default_advanced_change_control_rules_v0();
        assert_eq!(
            rules.authorized_to_make_change,
            AuthorizedActionTakers::NoOne
        );
        assert_eq!(rules.admin_action_takers, AuthorizedActionTakers::NoOne);
    }

    #[test]
    fn preset_advanced_rules_only_emergency_is_locked() {
        let p = preset(
            TokenConfigurationPresetFeatures::WithOnlyEmergencyAction,
            AuthorizedActionTakers::ContractOwner,
        );
        let rules = p.default_advanced_change_control_rules_v0();
        assert_eq!(
            rules.authorized_to_make_change,
            AuthorizedActionTakers::NoOne
        );
    }

    #[test]
    fn preset_advanced_rules_advanced_allows_action_taker() {
        let taker = AuthorizedActionTakers::ContractOwner;
        let p = preset(
            TokenConfigurationPresetFeatures::WithAllAdvancedActions,
            taker,
        );
        let rules = p.default_advanced_change_control_rules_v0();
        assert_eq!(rules.authorized_to_make_change, taker);
        assert!(rules.self_changing_admin_action_takers_allowed);
        assert!(!rules.changing_authorized_action_takers_to_no_one_allowed);
    }

    #[test]
    fn preset_advanced_rules_extreme_allows_everything() {
        let taker = AuthorizedActionTakers::ContractOwner;
        let p = preset(TokenConfigurationPresetFeatures::WithExtremeActions, taker);
        let rules = p.default_advanced_change_control_rules_v0();
        assert!(rules.changing_authorized_action_takers_to_no_one_allowed);
        assert!(rules.changing_admin_action_takers_to_no_one_allowed);
        assert!(rules.self_changing_admin_action_takers_allowed);
    }

    // --- default_emergency_action_change_control_rules_v0 ---

    #[test]
    fn preset_emergency_rules_most_restrictive_is_no_one() {
        let p = preset(
            TokenConfigurationPresetFeatures::MostRestrictive,
            AuthorizedActionTakers::ContractOwner,
        );
        let rules = p.default_emergency_action_change_control_rules_v0();
        assert_eq!(
            rules.authorized_to_make_change,
            AuthorizedActionTakers::NoOne
        );
    }

    #[test]
    fn preset_emergency_rules_only_emergency_allows_action_taker() {
        let taker = AuthorizedActionTakers::ContractOwner;
        let p = preset(
            TokenConfigurationPresetFeatures::WithOnlyEmergencyAction,
            taker,
        );
        let rules = p.default_emergency_action_change_control_rules_v0();
        assert_eq!(rules.authorized_to_make_change, taker);
        assert!(rules.self_changing_admin_action_takers_allowed);
    }

    #[test]
    fn preset_emergency_rules_minting_burning_allows_action_taker() {
        let taker = AuthorizedActionTakers::ContractOwner;
        let p = preset(
            TokenConfigurationPresetFeatures::WithMintingAndBurningActions,
            taker,
        );
        let rules = p.default_emergency_action_change_control_rules_v0();
        assert_eq!(rules.authorized_to_make_change, taker);
        assert!(rules.self_changing_admin_action_takers_allowed);
    }

    #[test]
    fn preset_emergency_rules_advanced_allows_action_taker() {
        let taker = AuthorizedActionTakers::ContractOwner;
        let p = preset(
            TokenConfigurationPresetFeatures::WithAllAdvancedActions,
            taker,
        );
        let rules = p.default_emergency_action_change_control_rules_v0();
        assert_eq!(rules.authorized_to_make_change, taker);
    }

    #[test]
    fn preset_emergency_rules_extreme_allows_no_one_transitions() {
        let taker = AuthorizedActionTakers::ContractOwner;
        let p = preset(TokenConfigurationPresetFeatures::WithExtremeActions, taker);
        let rules = p.default_emergency_action_change_control_rules_v0();
        assert!(rules.changing_authorized_action_takers_to_no_one_allowed);
    }

    // --- default_distribution_rules_v0 with/without direct pricing ---

    #[test]
    fn preset_distribution_rules_with_direct_pricing_uses_basic_rules() {
        let taker = AuthorizedActionTakers::ContractOwner;
        let p = preset(TokenConfigurationPresetFeatures::WithExtremeActions, taker);
        let rules = p.default_distribution_rules_v0(None, None, true);
        // With direct pricing enabled, the rules match basic (extreme -> owner, all permissive)
        assert_eq!(
            rules
                .change_direct_purchase_pricing_rules
                .authorized_to_make_change_action_takers(),
            &taker
        );
    }

    #[test]
    fn preset_distribution_rules_without_direct_pricing_locks_it_down() {
        let taker = AuthorizedActionTakers::ContractOwner;
        let p = preset(TokenConfigurationPresetFeatures::WithExtremeActions, taker);
        let rules = p.default_distribution_rules_v0(None, None, false);
        // Without direct pricing, change_direct_purchase_pricing_rules is hard-coded to NoOne
        assert_eq!(
            rules
                .change_direct_purchase_pricing_rules
                .authorized_to_make_change_action_takers(),
            &AuthorizedActionTakers::NoOne
        );
    }

    #[test]
    fn preset_distribution_rules_minting_choosing_destination_defaults_true() {
        let p = preset(
            TokenConfigurationPresetFeatures::MostRestrictive,
            AuthorizedActionTakers::NoOne,
        );
        let rules = p.default_distribution_rules_v0(None, None, false);
        assert!(rules.minting_allow_choosing_destination);
        assert!(rules.new_tokens_destination_identity.is_none());
        assert!(rules.perpetual_distribution.is_none());
        assert!(rules.pre_programmed_distribution.is_none());
    }

    // --- default_marketplace_rules_v0 ---

    #[test]
    fn preset_marketplace_rules_default_is_not_tradeable() {
        let p = preset(
            TokenConfigurationPresetFeatures::MostRestrictive,
            AuthorizedActionTakers::NoOne,
        );
        let mp = p.default_marketplace_rules_v0();
        assert_eq!(mp.trade_mode, TokenTradeMode::NotTradeable);
    }

    // --- token_configuration_v0 full config ---

    #[test]
    fn preset_token_configuration_v0_populates_fields() {
        let taker = AuthorizedActionTakers::ContractOwner;
        let p = preset(TokenConfigurationPresetFeatures::WithExtremeActions, taker);
        let conventions = TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
            localizations: Default::default(),
            decimals: 4,
        });
        let config = p.token_configuration_v0(conventions, 1_000, Some(5_000), true, true);
        assert_eq!(config.base_supply, 1_000);
        assert_eq!(config.max_supply, Some(5_000));
        assert_eq!(
            config
                .manual_minting_rules
                .authorized_to_make_change_action_takers(),
            &taker
        );
        // start_as_paused is fixed false by constructor
        assert!(!config.start_as_paused);
        assert!(config.allow_transfer_to_frozen_balance);
        assert_eq!(config.main_control_group, None);
        // extreme => main_control_group_can_be_modified becomes taker
        assert_eq!(config.main_control_group_can_be_modified, taker);
        // description is none
        assert!(config.description.is_none());
    }

    #[test]
    fn preset_token_configuration_keeps_all_history_true() {
        let p = preset(
            TokenConfigurationPresetFeatures::MostRestrictive,
            AuthorizedActionTakers::NoOne,
        );
        let conventions = TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
            localizations: Default::default(),
            decimals: 8,
        });
        let cfg = p.token_configuration_v0(conventions, 100, None, true, false);
        // keeps_history is TokenKeepsHistoryRules::V0; all fields should be true
        match &cfg.keeps_history {
            TokenKeepsHistoryRules::V0(v0) => {
                assert!(v0.keeps_transfer_history);
                assert!(v0.keeps_freezing_history);
                assert!(v0.keeps_minting_history);
                assert!(v0.keeps_burning_history);
                assert!(v0.keeps_direct_pricing_history);
                assert!(v0.keeps_direct_purchase_history);
            }
        }
    }

    #[test]
    fn preset_token_configuration_keeps_all_history_false() {
        let p = preset(
            TokenConfigurationPresetFeatures::MostRestrictive,
            AuthorizedActionTakers::NoOne,
        );
        let conventions = TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
            localizations: Default::default(),
            decimals: 8,
        });
        let cfg = p.token_configuration_v0(conventions, 100, None, false, false);
        match &cfg.keeps_history {
            TokenKeepsHistoryRules::V0(v0) => {
                assert!(!v0.keeps_transfer_history);
                assert!(!v0.keeps_direct_purchase_history);
            }
        }
    }

    // --- default_most_restrictive + with_base_supply chaining ---

    #[test]
    fn token_configuration_v0_default_most_restrictive_has_no_max_supply() {
        let c = TokenConfigurationV0::default_most_restrictive();
        assert_eq!(c.base_supply, 100_000);
        assert!(c.max_supply.is_none());
        assert_eq!(
            c.main_control_group_can_be_modified,
            AuthorizedActionTakers::NoOne
        );
    }

    #[test]
    fn token_configuration_v0_with_base_supply_overrides_value() {
        let c = TokenConfigurationV0::default_most_restrictive().with_base_supply(42);
        assert_eq!(c.base_supply, 42);
    }

    // --- Display trait ---

    #[test]
    fn display_token_configuration_v0_contains_key_fields() {
        let c = TokenConfigurationV0::default_most_restrictive();
        let s = format!("{}", c);
        assert!(s.contains("TokenConfigurationV0"));
        assert!(s.contains("base_supply"));
        assert!(s.contains("main_control_group"));
    }

    // --- all_used_group_positions: the interesting branches ---

    #[test]
    fn all_used_group_positions_empty_when_no_groups_referenced() {
        let c = TokenConfigurationV0::default_most_restrictive();
        let (positions, uses_main) = c.all_used_group_positions();
        assert!(positions.is_empty());
        assert!(!uses_main);
    }

    #[test]
    fn all_used_group_positions_collects_from_group_variant_in_rules() {
        let mut c = TokenConfigurationV0::default_most_restrictive();
        c.freeze_rules = ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::Group(7),
            admin_action_takers: AuthorizedActionTakers::Group(9),
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        });
        let (positions, uses_main) = c.all_used_group_positions();
        assert!(positions.contains(&7));
        assert!(positions.contains(&9));
        assert!(!uses_main);
    }

    #[test]
    fn all_used_group_positions_flags_main_group_usage() {
        let mut c = TokenConfigurationV0::default_most_restrictive();
        c.emergency_action_rules = ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::MainGroup,
            admin_action_takers: AuthorizedActionTakers::NoOne,
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        });
        let (_, uses_main) = c.all_used_group_positions();
        assert!(uses_main);
    }

    #[test]
    fn all_used_group_positions_includes_main_control_group() {
        let mut c = TokenConfigurationV0::default_most_restrictive();
        c.main_control_group = Some(42);
        let (positions, _) = c.all_used_group_positions();
        assert!(positions.contains(&42));
    }

    #[test]
    fn all_used_group_positions_includes_positions_from_main_control_group_can_be_modified() {
        let mut c = TokenConfigurationV0::default_most_restrictive();
        c.main_control_group_can_be_modified = AuthorizedActionTakers::Group(11);
        let (positions, _) = c.all_used_group_positions();
        assert!(positions.contains(&11));
    }

    #[test]
    fn all_used_group_positions_ignores_contract_owner_and_identity_and_no_one() {
        let mut c = TokenConfigurationV0::default_most_restrictive();
        c.manual_minting_rules = ChangeControlRules::V0(ChangeControlRulesV0 {
            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
            admin_action_takers: AuthorizedActionTakers::Identity(Identifier::from([1u8; 32])),
            changing_authorized_action_takers_to_no_one_allowed: false,
            changing_admin_action_takers_to_no_one_allowed: false,
            self_changing_admin_action_takers_allowed: false,
        });
        let (positions, uses_main) = c.all_used_group_positions();
        assert!(positions.is_empty());
        assert!(!uses_main);
    }

    // --- all_change_control_rules ---

    #[test]
    fn all_change_control_rules_returns_expected_rule_names() {
        let c = TokenConfigurationV0::default_most_restrictive();
        let rules = c.all_change_control_rules();
        let names: Vec<&str> = rules.iter().map(|(name, _)| *name).collect();
        assert!(names.contains(&"max_supply_change_rules"));
        assert!(names.contains(&"conventions_change_rules"));
        assert!(names.contains(&"manual_minting_rules"));
        assert!(names.contains(&"manual_burning_rules"));
        assert!(names.contains(&"freeze_rules"));
        assert!(names.contains(&"unfreeze_rules"));
        assert!(names.contains(&"destroy_frozen_funds_rules"));
        assert!(names.contains(&"emergency_action_rules"));
        assert!(names.contains(&"trade_mode_change_rules"));
        // 13 rules total per the implementation
        assert_eq!(rules.len(), 13);
    }

    // --- setters exercise the right fields ---

    #[test]
    fn setters_set_description_max_supply_base_supply_main_control_group() {
        let mut c = TokenConfigurationV0::default_most_restrictive();
        c.set_description(Some("my token".to_string()));
        c.set_max_supply(Some(999));
        c.set_base_supply(77);
        c.set_main_control_group(Some(3));
        c.set_start_as_paused(true);
        c.allow_transfer_to_frozen_balance(false);
        c.set_main_control_group_can_be_modified(AuthorizedActionTakers::ContractOwner);
        assert_eq!(c.description(), &Some("my token".to_string()));
        assert_eq!(c.max_supply(), Some(999));
        assert_eq!(c.base_supply(), 77);
        assert_eq!(c.main_control_group(), Some(3));
        assert!(c.start_as_paused());
        assert!(!c.is_allowed_transfer_to_frozen_balance());
        assert_eq!(
            c.main_control_group_can_be_modified(),
            &AuthorizedActionTakers::ContractOwner
        );
    }
}
