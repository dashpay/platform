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
