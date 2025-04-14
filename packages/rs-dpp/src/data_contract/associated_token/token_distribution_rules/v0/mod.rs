mod accessors;

use crate::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use crate::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use crate::data_contract::change_control_rules::ChangeControlRules;
use bincode::Encode;
use platform_serialization::de::Decode;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenDistributionRulesV0 {
    #[serde(default)]
    pub perpetual_distribution: Option<TokenPerpetualDistribution>,
    #[serde(default = "default_change_control_rules")]
    pub perpetual_distribution_rules: ChangeControlRules,
    #[serde(default)]
    pub pre_programmed_distribution: Option<TokenPreProgrammedDistribution>,
    #[serde(default)]
    pub new_tokens_destination_identity: Option<Identifier>,
    #[serde(default = "default_change_control_rules")]
    pub new_tokens_destination_identity_rules: ChangeControlRules,
    #[serde(default = "default_minting_allow_choosing_destination")]
    pub minting_allow_choosing_destination: bool,
    #[serde(default = "default_change_control_rules")]
    pub minting_allow_choosing_destination_rules: ChangeControlRules,
    #[serde(default = "default_change_control_rules")]
    pub change_direct_purchase_pricing_rules: ChangeControlRules,
}

// Default function for `minting_allow_choosing_destination` to return `true`
fn default_minting_allow_choosing_destination() -> bool {
    true
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

impl fmt::Display for TokenDistributionRulesV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenDistributionRulesV0 {{\n  \
            perpetual_distribution: {},\n  \
            perpetual_distribution_rules: {},\n  \
            pre_programmed_distribution: {},\n  \
            new_tokens_destination_identity: {},\n  \
            new_tokens_destination_identity_rules: {},\n  \
            minting_allow_choosing_destination: {},\n  \
            minting_allow_choosing_destination_rules: {},\n  \
            change_direct_purchase_pricing_rules:: {}\n\
            }}",
            match &self.perpetual_distribution {
                Some(value) => format!("{}", value),
                None => "None".to_string(),
            },
            self.perpetual_distribution_rules,
            match &self.pre_programmed_distribution {
                Some(value) => format!("{}", value),
                None => "None".to_string(),
            },
            match &self.new_tokens_destination_identity {
                Some(value) => format!("{}", value),
                None => "None".to_string(),
            },
            self.new_tokens_destination_identity_rules,
            self.minting_allow_choosing_destination,
            self.minting_allow_choosing_destination_rules,
            self.change_direct_purchase_pricing_rules,
        )
    }
}
