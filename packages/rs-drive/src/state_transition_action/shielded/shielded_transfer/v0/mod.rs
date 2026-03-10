mod transformer;

use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;

/// Shielded transfer transition action v0
#[derive(Debug, Clone)]
pub struct ShieldedTransferTransitionActionV0 {
    /// Notes from the orchard bundle actions
    pub notes: Vec<ShieldedActionNote>,
    /// The anchor (root of the commitment tree) used for verification
    pub anchor: [u8; 32],
    /// Fee amount extracted from the value balance
    pub fee_amount: Credits,
    /// Current total balance of the shielded pool
    pub current_total_balance: Credits,
}
