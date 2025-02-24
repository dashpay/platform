mod methods;

use bincode::{Encode, Decode};
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct TokenPerpetualDistributionV0 {
    /// The distribution type that the token will use
    pub distribution_type: RewardDistributionType,
    /// The recipient type
    pub distribution_recipient: TokenDistributionRecipient,
}

impl fmt::Display for TokenPerpetualDistributionV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenPerpetualDistribution {{\n  distribution_type: {},\n  distribution_recipient: {}\n}}",
            self.distribution_type,
            self.distribution_recipient,
        )
    }
}
