#[cfg(feature = "state-transition-transformers")]
mod transformer;

use crate::contracts::withdrawals_contract;
use crate::document::{generate_document_id, Document, DocumentV0};
use crate::identifier::Identifier;
use crate::prelude::Revision;

use platform_value::platform_value;
use serde::{Deserialize, Serialize};


#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreditWithdrawalTransitionActionV0 {
    pub identity_id: Identifier,
    pub revision: Revision,
    pub prepared_withdrawal_document: Document,
}
