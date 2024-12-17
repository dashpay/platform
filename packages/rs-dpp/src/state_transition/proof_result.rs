use crate::data_contract::DataContract;
use crate::document::Document;
use crate::identity::{Identity, PartialIdentity};
use crate::voting::votes::Vote;
use platform_value::Identifier;
use std::collections::BTreeMap;

#[derive(Debug, strum::Display, derive_more::TryInto)]
#[ferment_macro::export]
pub enum StateTransitionProofResult {
    VerifiedDataContract(DataContract),
    VerifiedIdentity(Identity),
    VerifiedPartialIdentity(PartialIdentity),
    VerifiedBalanceTransfer(PartialIdentity, PartialIdentity), //from/to
    VerifiedDocuments(BTreeMap<Identifier, Option<Document>>),
    VerifiedMasternodeVote(Vote),
}
