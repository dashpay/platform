mod transformer;

use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::prelude::{UserFeeMultiplier, IdentityNonce};
use serde::{Deserialize, Serialize};

/// action v0
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreditTransferTransitionActionV0 {
    /// transfer amount
    pub transfer_amount: Credits,
    /// recipient id
    pub recipient_id: Identifier,
    /// identity id
    pub identity_id: Identifier,
    /// nonce
    pub nonce: IdentityNonce,
    /// fee multiplier
    pub fee_multiplier: UserFeeMultiplier,
}
