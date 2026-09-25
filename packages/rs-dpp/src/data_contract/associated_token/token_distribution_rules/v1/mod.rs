mod accessors;

use crate::data_contract::associated_token::token_distribution_rules::v0::TokenDistributionRulesV0;
use crate::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;
use crate::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use crate::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::change_control_rules::v0::ChangeControlRulesV0;
use crate::data_contract::change_control_rules::ChangeControlRules;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use bincode::{DecodeUntrusted, Encode};
use platform_serialization::de::Decode;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Version 1 of the distribution rules: version 0 plus the once-per-identity distribution
/// (protocol version 14).
///
/// Version 0 stays the wire format of every token that does not use the once-per-identity
/// distribution, so contracts registered before protocol version 14 decode unchanged. A token
/// that sets `once_per_identity_distribution` serializes as version 1.
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, DecodeUntrusted)]
#[serde(rename_all = "camelCase")]
pub struct TokenDistributionRulesV1 {
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
    /// A fixed amount every identity may claim exactly once. Fixed at registration: there is no
    /// change control rule for it and no configuration update item changes it.
    #[serde(default)]
    pub once_per_identity_distribution: Option<TokenOncePerIdentityDistribution>,
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

impl From<TokenDistributionRulesV0> for TokenDistributionRulesV1 {
    fn from(v0: TokenDistributionRulesV0) -> Self {
        let TokenDistributionRulesV0 {
            perpetual_distribution,
            perpetual_distribution_rules,
            pre_programmed_distribution,
            new_tokens_destination_identity,
            new_tokens_destination_identity_rules,
            minting_allow_choosing_destination,
            minting_allow_choosing_destination_rules,
            change_direct_purchase_pricing_rules,
        } = v0;
        TokenDistributionRulesV1 {
            perpetual_distribution,
            perpetual_distribution_rules,
            pre_programmed_distribution,
            new_tokens_destination_identity,
            new_tokens_destination_identity_rules,
            minting_allow_choosing_destination,
            minting_allow_choosing_destination_rules,
            change_direct_purchase_pricing_rules,
            once_per_identity_distribution: None,
        }
    }
}

impl fmt::Display for TokenDistributionRulesV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenDistributionRulesV1 {{\n  \
            perpetual_distribution: {},\n  \
            perpetual_distribution_rules: {},\n  \
            pre_programmed_distribution: {},\n  \
            new_tokens_destination_identity: {},\n  \
            new_tokens_destination_identity_rules: {},\n  \
            minting_allow_choosing_destination: {},\n  \
            minting_allow_choosing_destination_rules: {},\n  \
            change_direct_purchase_pricing_rules: {},\n  \
            once_per_identity_distribution: {}\n\
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
            match &self.once_per_identity_distribution {
                Some(value) => format!("{}", value),
                None => "None".to_string(),
            },
        )
    }
}
