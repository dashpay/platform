mod transformer;

use dpp::fee::Credits;
use dpp::identity::KeyOfType;
use dpp::platform_value::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action v0
#[derive(Default, Debug, Clone)]
pub struct IdentityCreditTransferToAddressesTransitionActionV0 {
    /// recipient keys
    pub recipient_keys: BTreeMap<KeyOfType, Credits>,
    /// identity id
    pub identity_id: Identifier,
    /// nonce
    pub nonce: IdentityNonce,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
