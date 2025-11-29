mod transformer;

use dpp::identifier::Identifier;
use std::collections::BTreeMap;

use dpp::fee::Credits;
use dpp::identity::KeyOfType;
use dpp::prelude::{AddressNonce, UserFeeIncrease};

/// action v0
#[derive(Debug, Clone)]
pub struct IdentityTopUpFromAddressesTransitionActionV0 {
    /// inputs
    pub inputs_with_remaining_balance: BTreeMap<KeyOfType, (AddressNonce, Credits)>,
    /// identity id
    pub identity_id: Identifier,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
