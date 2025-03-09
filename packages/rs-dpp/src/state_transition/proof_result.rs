use crate::balances::credits::TokenAmount;
use crate::data_contract::DataContract;
use crate::document::Document;
use crate::identity::{Identity, PartialIdentity};
use crate::tokens::info::IdentityTokenInfo;
use crate::tokens::status::TokenStatus;
use crate::voting::votes::Vote;
use platform_value::Identifier;
use std::collections::BTreeMap;

#[derive(Debug, strum::Display, derive_more::TryInto)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub enum StateTransitionProofResult {
    VerifiedDataContract(DataContract),
    VerifiedIdentity(Identity),
    VerifiedTokenBalanceAbsence(Identifier),
    VerifiedTokenBalance(Identifier, TokenAmount),
    VerifiedTokenIdentityInfo(Identifier, IdentityTokenInfo),
    VerifiedTokenStatus(TokenStatus),
    VerifiedTokenIdentitiesBalances(BTreeMap<Identifier, TokenAmount>),
    VerifiedPartialIdentity(PartialIdentity),
    VerifiedBalanceTransfer(PartialIdentity, PartialIdentity), //from/to
    VerifiedDocuments(BTreeMap<Identifier, Option<Document>>),
    VerifiedTokenActionWithDocument(Document),
    VerifiedMasternodeVote(Vote),
}
