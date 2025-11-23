mod transformer;

use dpp::fee::Credits;
use dpp::identity::KeyOfType;
use dpp::prelude::{KeyOfTypeNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action v0
#[derive(Default, Debug, Clone)]
pub struct AddressFundsTransferTransitionActionV0 {
    /// inputs
    pub inputs_with_remaining_balance: BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>,
    /// outputs
    pub outputs: BTreeMap<KeyOfType, Credits>,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
