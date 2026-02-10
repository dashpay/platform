mod transformer;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::UserFeeIncrease;

/// Unshield transition action v0
#[derive(Default, Debug, Clone)]
pub struct UnshieldTransitionActionV0 {
    /// The address receiving unshielded funds
    pub output_address: PlatformAddress,
    /// Amount being unshielded
    pub amount: Credits,
    /// Nullifiers from spent notes
    pub nullifiers: Vec<[u8; 32]>,
    /// Note commitments for change outputs
    pub note_commitments: Vec<[u8; 32]>,
    /// Encrypted notes for change outputs
    pub encrypted_notes: Vec<Vec<u8>>,
    /// The anchor used for verification
    pub anchor: [u8; 32],
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    /// Current checkpoint ID counter read from shielded pool params
    pub current_checkpoint_id: u64,
    /// Current total balance of the shielded pool
    pub current_total_balance: Credits,
}
