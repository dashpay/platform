use crate::data_contract::DataContract;
use crate::document::Document;
use crate::identity::{Identity, PartialIdentity};
use platform_value::types::identifier::Identifier;
use std::collections::BTreeMap;

#[derive(Debug)]
#[ferment_macro::export]
pub enum StateTransitionProofResult {
    VerifiedDataContract(DataContract),
    VerifiedIdentity(Identity),
    VerifiedPartialIdentity(PartialIdentity),
    VerifiedBalanceTransfer(PartialIdentity, PartialIdentity), //from/to
    VerifiedDocuments(BTreeMap<Identifier, Option<Document>>),
}
