mod transformer;

use dpp::address_funds::fee_strategy::AddressFundsFeeStrategy;
use dpp::address_funds::PlatformAddress;
use dpp::identifier::Identifier;
use std::collections::BTreeMap;

use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};

/// action v0
#[derive(Debug, Clone)]
pub struct IdentityTopUpFromAddressesTransitionActionV0 {
    /// inputs with remaining balance after transfer
    pub inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// optional output to send remaining credits to an address
    pub output: Option<(PlatformAddress, Credits)>,
    /// fee strategy for determining order of fee deduction
    pub fee_strategy: AddressFundsFeeStrategy,
    /// identity id
    pub identity_id: Identifier,
    /// the amount to add to the identity's balance
    pub topup_amount: Credits,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
