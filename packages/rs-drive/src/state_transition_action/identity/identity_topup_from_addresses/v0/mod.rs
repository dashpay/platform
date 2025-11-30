mod transformer;

use dpp::address_funds::PlatformAddress;
use dpp::identifier::Identifier;
use std::collections::BTreeMap;

use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};

/// action v0
#[derive(Debug, Clone)]
pub struct IdentityTopUpFromAddressesTransitionActionV0 {
    /// inputs
    pub inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// optional output to send remaining credits to an address
    pub output: Option<(PlatformAddress, Credits)>,
    /// identity id
    pub identity_id: Identifier,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
