mod transformer;

use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action v0
#[derive(Debug, Clone)]
pub struct AddressCreditWithdrawalTransitionActionV0 {
    /// inputs with remaining balance after withdrawal
    pub inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// optional output for change
    pub output: Option<(PlatformAddress, Credits)>,
    /// fee strategy
    pub fee_strategy: AddressFundsFeeStrategy,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    /// prepared withdrawal document
    pub prepared_withdrawal_document: Document,
    /// the total amount to withdraw
    pub amount: Credits,
}
