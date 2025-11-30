mod transformer;

use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action v0
#[derive(Default, Debug, Clone)]
pub struct AddressFundingFromAssetLockTransitionActionV0 {
    /// inputs with remaining balance (may be empty if no existing addresses are used)
    pub inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// outputs
    pub outputs: BTreeMap<PlatformAddress, Credits>,
    /// fee strategy
    pub fee_strategy: AddressFundsFeeStrategy,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
