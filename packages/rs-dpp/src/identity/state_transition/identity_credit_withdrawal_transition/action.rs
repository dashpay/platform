use crate::document::Document;
use crate::identifier::Identifier;
use serde::{Deserialize, Serialize};

pub const IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_VERSION: u32 = 0;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreditWithdrawalTransitionAction {
    pub version: u32,
    pub identity_id: Identifier,
    pub prepared_withdrawal_document: Document,
}

impl IdentityCreditWithdrawalTransitionAction {
    pub fn current_version() -> u32 {
        IDENTITY_CREDIT_WITHDRAWAL_TRANSITION_VERSION
    }
}
