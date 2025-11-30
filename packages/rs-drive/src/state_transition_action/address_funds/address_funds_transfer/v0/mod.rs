mod transformer;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action v0
#[derive(Default, Debug, Clone)]
pub struct AddressFundsTransferTransitionActionV0 {
    /// inputs
    pub inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// outputs
    pub outputs: BTreeMap<PlatformAddress, Credits>,
    /// fee multiplier, this is already taken into account in the action
    pub user_fee_increase: UserFeeIncrease,
}
