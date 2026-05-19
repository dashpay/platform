mod transformer;

use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// Shield transition action v0
#[derive(Debug, Clone)]
pub struct ShieldTransitionActionV0 {
    /// inputs with remaining balance after shielding
    pub inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// The amount being shielded (sent into the shielded pool)
    pub shield_amount: Credits,
    /// Notes from the orchard bundle actions
    pub notes: Vec<ShieldedActionNote>,
    /// fee strategy
    pub fee_strategy: AddressFundsFeeStrategy,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    /// Current total balance of the shielded pool
    pub current_total_balance: Credits,
}
