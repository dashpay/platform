mod transformer;

use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

/// Unshield transition action v0
#[derive(Debug, Clone)]
pub struct UnshieldTransitionActionV0 {
    /// The address receiving unshielded funds
    pub output_address: PlatformAddress,
    /// Amount being unshielded
    pub amount: Credits,
    /// Notes from the orchard bundle actions
    pub notes: Vec<ShieldedActionNote>,
    /// The anchor used for verification
    pub anchor: [u8; 32],
    /// Fee amount (value_balance - amount), paid to proposers
    pub fee_amount: Credits,
    /// Current total balance of the shielded pool
    pub current_total_balance: Credits,
}
