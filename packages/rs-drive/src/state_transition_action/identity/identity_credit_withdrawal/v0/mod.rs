mod transformer;

use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::prelude::Revision;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreditWithdrawalTransitionActionV0 {
    pub identity_id: Identifier,
    pub revision: Revision,
    pub prepared_withdrawal_document: Document,
}
