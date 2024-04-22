mod transformer;

use dpp::identifier::Identifier;

use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::platform_value::Bytes36;
use dpp::prelude::UserFeeIncrease;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;

/// action v0
#[derive(Debug, Clone)]
pub struct IdentityTopUpTransitionActionV0 {
    /// The state transition signable bytes hash
    pub signable_bytes_hasher: SignableBytesHasher,
    /// we top up the remaining amount of the asset lock value
    pub top_up_asset_lock_value: AssetLockValue,
    /// identity id
    pub identity_id: Identifier,
    /// asset lock outpoint
    pub asset_lock_outpoint: Bytes36,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
