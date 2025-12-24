mod transformer;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action v0
#[derive(Default, Debug, Clone)]
pub struct IdentityCreditTransferToAddressesTransitionActionV0 {
    /// recipient addresses
    pub recipient_addresses: BTreeMap<PlatformAddress, Credits>,
    /// identity id
    pub identity_id: Identifier,
    /// nonce
    pub nonce: IdentityNonce,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
