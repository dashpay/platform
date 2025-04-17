mod accessors;

use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
use crate::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
use crate::data_contract::associated_token::token_distribution_rules::TokenDistributionRules;
use crate::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
use crate::data_contract::associated_token::token_keeps_history_rules::TokenKeepsHistoryRules;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use crate::data_contract::change_control_rules::ChangeControlRules;
use crate::data_contract::GroupContractPosition;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfigurationV0 {
    pub conventions: TokenConfigurationConvention,
    /// Who can change the conventions
    #[serde(default = "default_change_control_rules")]
    pub conventions_change_rules: ChangeControlRules,
    /// The supply at the creation of the token
    #[serde(default)]
    pub base_supply: TokenAmount,
    /// The maximum supply the token can ever have
    #[serde(default)]
    pub max_supply: Option<TokenAmount>,
    /// The rules for keeping history.
    #[serde(default = "default_token_keeps_history_rules")]
    pub keeps_history: TokenKeepsHistoryRules,
    /// Do we start off as paused, meaning that we can not transfer till we unpause.
    #[serde(default = "default_starts_as_paused")]
    pub start_as_paused: bool,
    /// Allow to transfer and mint tokens to frozen identity token balances
    #[serde(default = "default_allow_transfer_to_frozen_balance")]
    pub allow_transfer_to_frozen_balance: bool,
    /// Who can change the max supply
    /// Even if set no one can ever change this under the base supply
    #[serde(default = "default_change_control_rules")]
    pub max_supply_change_rules: ChangeControlRules,
    /// The distribution rules for the token
    #[serde(default = "default_token_distribution_rules")]
    pub distribution_rules: TokenDistributionRules,
    #[serde(default = "default_contract_owner_change_control_rules")]
    pub manual_minting_rules: ChangeControlRules,
    #[serde(default = "default_contract_owner_change_control_rules")]
    pub manual_burning_rules: ChangeControlRules,
    #[serde(default = "default_change_control_rules")]
    pub freeze_rules: ChangeControlRules,
    #[serde(default = "default_change_control_rules")]
    pub unfreeze_rules: ChangeControlRules,
    #[serde(default = "default_change_control_rules")]
    pub destroy_frozen_funds_rules: ChangeControlRules,
    #[serde(default = "default_change_control_rules")]
    pub emergency_action_rules: ChangeControlRules,
    #[serde(default)]
    pub main_control_group: Option<GroupContractPosition>,
    #[serde(default)]
    pub main_control_group_can_be_modified: AuthorizedActionTakers,
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

impl TokenConfigurationV0 {
    pub fn default_most_restrictive() -> Self {
        Self {
            conventions: TokenConfigurationConvention::V0(TokenConfigurationConventionV0 {
                localizations: Default::default(),
                decimals: 8,
            }),
            conventions_change_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                admin_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            base_supply: 100000,
            max_supply: None,
            keeps_history: TokenKeepsHistoryRules::V0(TokenKeepsHistoryRulesV0 {
                keeps_transfer_history: true,
                keeps_freezing_history: true,
                keeps_minting_history: true,
                keeps_burning_history: true,
                keeps_direct_pricing_history: true,
                keeps_direct_purchase_history: true,
            }),
            start_as_paused: false,
            allow_transfer_to_frozen_balance: true,
            max_supply_change_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                admin_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            distribution_rules: TokenDistributionRules::V0(TokenDistributionRulesV0 {
                perpetual_distribution: None,
                perpetual_distribution_rules: ChangeControlRulesV0 {
                    authorized_to_make_change: AuthorizedActionTakers::NoOne,
                    admin_action_takers: AuthorizedActionTakers::NoOne,
                    changing_authorized_action_takers_to_no_one_allowed: false,
                    changing_admin_action_takers_to_no_one_allowed: false,
                    self_changing_admin_action_takers_allowed: false,
                }
                .into(),
                pre_programmed_distribution: None,
                new_tokens_destination_identity: None,
                new_tokens_destination_identity_rules: ChangeControlRulesV0 {
                    authorized_to_make_change: AuthorizedActionTakers::NoOne,
                    admin_action_takers: AuthorizedActionTakers::NoOne,
                    changing_authorized_action_takers_to_no_one_allowed: false,
                    changing_admin_action_takers_to_no_one_allowed: false,
                    self_changing_admin_action_takers_allowed: false,
                }
                .into(),
                minting_allow_choosing_destination: true,
                minting_allow_choosing_destination_rules: ChangeControlRulesV0 {
                    authorized_to_make_change: AuthorizedActionTakers::NoOne,
                    admin_action_takers: AuthorizedActionTakers::NoOne,
                    changing_authorized_action_takers_to_no_one_allowed: false,
                    changing_admin_action_takers_to_no_one_allowed: false,
                    self_changing_admin_action_takers_allowed: false,
                }
                .into(),
                change_direct_purchase_pricing_rules: ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::NoOne,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ),
            }),
            manual_minting_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                admin_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            manual_burning_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                admin_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            freeze_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                admin_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            unfreeze_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                admin_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            destroy_frozen_funds_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                admin_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            emergency_action_rules: ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::NoOne,
                admin_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }
            .into(),
            main_control_group: None,
            main_control_group_can_be_modified: AuthorizedActionTakers::NoOne,
            description: None,
        }
    }

    pub fn with_base_supply(mut self, base_supply: TokenAmount) -> Self {
        self.base_supply = base_supply;
        self
    }
}
