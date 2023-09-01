mod transformer;

use dpp::fee::Credits;
use dpp::platform_value::Identifier;
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
}
