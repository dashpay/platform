mod transformer;

use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::prelude::{UserFeeIncrease, IdentityNonce};

use serde::{Deserialize, Serialize};

/// action v0
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreditWithdrawalTransitionActionV0 {
    /// identity id
    pub identity_id: Identifier,
    /// nonce
    pub nonce: IdentityNonce,
    /// prepared withdrawal document
    pub prepared_withdrawal_document: Document,
    /// amount
    pub amount: u64,
    /// fee multiplier
    pub fee_multiplier: UserFeeIncrease,
}
