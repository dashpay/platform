mod transformer;

use dpp::identifier::Identifier;

use dpp::dashcore::OutPoint;
use serde::{Deserialize, Serialize};

/// action v0
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityTopUpTransitionActionV0 {
    /// top up balance amount
    pub top_up_balance_amount: u64,
    /// identity id
    pub identity_id: Identifier,
    /// asset lock outpoint
    pub asset_lock_outpoint: OutPoint,
}
