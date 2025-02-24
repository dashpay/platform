use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use crate::prelude::TimestampMillis;
use platform_value::Identifier;
use std::collections::BTreeMap;
use v0::TokenPreProgrammedDistributionV0Methods;

pub mod v0;

impl TokenPreProgrammedDistributionV0Methods for TokenPreProgrammedDistribution {
    fn distributions(&self) -> &BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>> {
        match self {
            TokenPreProgrammedDistribution::V0(v0) => &v0.distributions,
        }
    }

    fn set_distributions(
        &mut self,
        distributions: BTreeMap<TimestampMillis, BTreeMap<Identifier, TokenAmount>>,
    ) {
        match self {
            TokenPreProgrammedDistribution::V0(v0) => v0.distributions = distributions,
        }
    }

    fn add_distribution(
        &mut self,
        time: TimestampMillis,
        recipient: Identifier,
        amount: TokenAmount,
    ) {
        match self {
            TokenPreProgrammedDistribution::V0(v0) => {
                v0.distributions
                    .entry(time)
                    .or_default()
                    .insert(recipient, amount);
            }
        }
    }
}
