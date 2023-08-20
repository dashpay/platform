mod transformer;

use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreditTransferTransitionActionV0 {
    pub transfer_amount: Credits,
    pub recipient_id: Identifier,
    pub identity_id: Identifier,
}
