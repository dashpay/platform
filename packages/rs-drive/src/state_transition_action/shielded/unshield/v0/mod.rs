mod transformer;

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

/// Unshield transition action v0
#[derive(Debug, Clone)]
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
    /// Fee amount (value_balance - amount), paid to proposers
    pub fee_amount: Credits,
    /// Current total balance of the shielded pool
    pub current_total_balance: Credits,
}
