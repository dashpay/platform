mod transformer;

use dpp::identifier::Identifier;

use dpp::platform_value::Bytes36;
use dpp::prelude::UserFeeIncrease;
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
    pub asset_lock_outpoint: Bytes36,
    /// fee multiplier
    pub fee_multiplier: UserFeeIncrease,
}
