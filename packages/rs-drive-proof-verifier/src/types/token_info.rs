use crate::types::RetrievedObjects;
use dpp::identifier::Identifier;
use dpp::tokens::info::IdentityTokenInfo;
use std::ops::Deref;

/// Information (i.e. balance frozen) about multiple tokens of one specific identity
#[derive(Debug, Default, Clone, derive_more::From)]
pub struct IdentityTokenInfos(
    /// Token ID to token info
    #[from]
    pub RetrievedObjects<Identifier, IdentityTokenInfo>,
);

impl Deref for IdentityTokenInfos {
    type Target = RetrievedObjects<Identifier, IdentityTokenInfo>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromIterator<(Identifier, Option<IdentityTokenInfo>)> for IdentityTokenInfos {
    fn from_iter<T: IntoIterator<Item = (Identifier, Option<IdentityTokenInfo>)>>(iter: T) -> Self {
        iter.into_iter()
            .collect::<RetrievedObjects<Identifier, IdentityTokenInfo>>()
            .into()
    }
}

/// Information (i.e. balance frozen) about one specific token of multiple identities
#[derive(Debug, Default, Clone, derive_more::From)]
pub struct IdentitiesTokenInfos(
    /// Identity ID to token info
    pub RetrievedObjects<Identifier, IdentityTokenInfo>,
);

impl Deref for IdentitiesTokenInfos {
    type Target = RetrievedObjects<Identifier, IdentityTokenInfo>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromIterator<(Identifier, Option<IdentityTokenInfo>)> for IdentitiesTokenInfos {
    fn from_iter<T: IntoIterator<Item = (Identifier, Option<IdentityTokenInfo>)>>(iter: T) -> Self {
        iter.into_iter()
            .collect::<RetrievedObjects<Identifier, IdentityTokenInfo>>()
            .into()
    }
}
