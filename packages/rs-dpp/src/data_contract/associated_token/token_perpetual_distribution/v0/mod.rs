mod methods;

use bincode::{Encode, Decode};
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenPerpetualDistributionRecipient;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;

#[derive(Serialize, Deserialize, Decode, Encode, Debug, Clone, PartialEq, Eq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub struct TokenPerpetualDistributionV0 {
    /// The distribution type that the token will use
    pub distribution_type: RewardDistributionType,
    /// The recipient type
    pub distribution_recipient: TokenPerpetualDistributionRecipient,
    /// Is the release of the token automatic if the owner id has enough balance?
    pub automatic_release: bool,
}

impl fmt::Display for TokenPerpetualDistributionV0 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenPerpetualDistribution {{\n  distribution_type: {},\n  distribution_recipient: {},\n  automatic_release: {}\n}}",
            self.distribution_type,
            self.distribution_recipient,
            self.automatic_release
        )
    }
}
