use crate::balances::credits::TokenAmount;
use crate::data_contract::group::GroupSumPower;
use crate::data_contract::DataContract;
use crate::document::Document;
use crate::group::group_action_status::GroupActionStatus;
use crate::identity::{Identity, PartialIdentity};
use crate::tokens::info::IdentityTokenInfo;
use crate::tokens::status::TokenStatus;
use crate::tokens::token_pricing_schedule::TokenPricingSchedule;
use crate::voting::votes::Vote;
use platform_value::Identifier;
use std::collections::BTreeMap;

#[derive(Debug, strum::Display, derive_more::TryInto)]
pub enum StateTransitionProofResult {
    VerifiedDataContract(DataContract),
    VerifiedIdentity(Identity),
    VerifiedTokenBalanceAbsence(Identifier),
    VerifiedTokenBalance(Identifier, TokenAmount),
    VerifiedTokenIdentityInfo(Identifier, IdentityTokenInfo),
    VerifiedTokenPricingSchedule(Identifier, Option<TokenPricingSchedule>),
    VerifiedTokenStatus(TokenStatus),
    VerifiedTokenIdentitiesBalances(BTreeMap<Identifier, TokenAmount>),
    VerifiedPartialIdentity(PartialIdentity),
    VerifiedBalanceTransfer(PartialIdentity, PartialIdentity), //from/to
    VerifiedDocuments(BTreeMap<Identifier, Option<Document>>),
    VerifiedTokenActionWithDocument(Document),
    VerifiedTokenGroupActionWithDocument(GroupSumPower, Option<Document>),
    VerifiedTokenGroupActionWithTokenBalance(GroupSumPower, GroupActionStatus, Option<TokenAmount>),
    VerifiedTokenGroupActionWithTokenIdentityInfo(
        GroupSumPower,
        GroupActionStatus,
        Option<IdentityTokenInfo>,
    ),
    VerifiedTokenGroupActionWithTokenPricingSchedule(
        GroupSumPower,
        GroupActionStatus,
        Option<TokenPricingSchedule>,
    ),
    VerifiedMasternodeVote(Vote),
    VerifiedNextDistribution(Vote),
}
