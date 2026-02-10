mod transformer;

use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::fee::Credits;
use dpp::platform_value::Bytes36;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use std::collections::BTreeMap;

/// action v0
#[derive(Debug, Clone)]
pub struct AddressFundingFromAssetLockTransitionActionV0 {
    /// The state transition signable bytes hash
    pub signable_bytes_hasher: SignableBytesHasher,
    /// the initial balance amount is equal to the remaining asset lock value
    pub asset_lock_value_to_be_consumed: AssetLockValue,
    /// asset lock outpoint
    pub asset_lock_outpoint: Bytes36,
    /// inputs with remaining balance (may be empty if no existing addresses are used)
    pub inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// outputs (Some = explicit amount, None = remainder recipient)
    pub outputs: BTreeMap<PlatformAddress, Option<Credits>>,
    /// Total credits contributed from address inputs (sum of amounts being spent)
    pub input_contributions_total: Credits,
    /// fee strategy
    pub fee_strategy: AddressFundsFeeStrategy,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
