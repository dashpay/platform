use crate::balances::credits::TokenAmount;

pub trait TokenOncePerIdentityDistributionV0Methods {
    /// The amount minted to an identity on its single claim.
    fn amount(&self) -> TokenAmount;

    fn set_amount(&mut self, amount: TokenAmount);
}
