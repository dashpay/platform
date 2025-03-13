use crate::types::RetrievedObjects;
use dpp::balances::credits::TokenAmount;
use dpp::identifier::Identifier;
use std::ops::Deref;

/// Multiple token balances of one specific identity
#[derive(Debug, Default, Clone, derive_more::From)]
pub struct IdentityTokenBalances(
    /// Token ID to token balance
    #[from]
    pub RetrievedObjects<Identifier, TokenAmount>,
);

impl Deref for IdentityTokenBalances {
    type Target = RetrievedObjects<Identifier, TokenAmount>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromIterator<(Identifier, Option<TokenAmount>)> for IdentityTokenBalances {
    fn from_iter<T: IntoIterator<Item = (Identifier, Option<TokenAmount>)>>(iter: T) -> Self {
        iter.into_iter()
            .collect::<RetrievedObjects<Identifier, TokenAmount>>()
            .into()
    }
}

/// One specific token balance of multiple identities
#[derive(Debug, Default, Clone, derive_more::From)]
pub struct IdentitiesTokenBalances(
    /// Identity ID to token balance
    #[from]
    pub RetrievedObjects<Identifier, TokenAmount>,
);

impl Deref for IdentitiesTokenBalances {
    type Target = RetrievedObjects<Identifier, TokenAmount>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromIterator<(Identifier, Option<TokenAmount>)> for IdentitiesTokenBalances {
    fn from_iter<T: IntoIterator<Item = (Identifier, Option<TokenAmount>)>>(iter: T) -> Self {
        iter.into_iter()
            .collect::<RetrievedObjects<Identifier, TokenAmount>>()
            .into()
    }
}
