mod transformer;

use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// action v0
#[derive(Debug, Clone)]
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
    pub user_fee_increase: UserFeeIncrease,
}
