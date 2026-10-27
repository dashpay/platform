use crate::balances::credits::TokenAmount;
use crate::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;
use v0::TokenOncePerIdentityDistributionV0Methods;

pub mod v0;

impl TokenOncePerIdentityDistributionV0Methods for TokenOncePerIdentityDistribution {
    fn amount(&self) -> TokenAmount {
        match self {
            TokenOncePerIdentityDistribution::V0(v0) => v0.amount,
        }
    }

    fn set_amount(&mut self, amount: TokenAmount) {
        match self {
            TokenOncePerIdentityDistribution::V0(v0) => v0.amount = amount,
        }
    }
}
