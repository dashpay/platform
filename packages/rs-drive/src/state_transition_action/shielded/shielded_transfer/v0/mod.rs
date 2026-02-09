mod transformer;

use dpp::fee::Credits;
use dpp::prelude::UserFeeIncrease;

/// Shielded transfer transition action v0
#[derive(Default, Debug, Clone)]
pub struct ShieldedTransferTransitionActionV0 {
    /// Nullifiers from spent notes
    pub nullifiers: Vec<[u8; 32]>,
    /// Note commitments for new output notes
    pub note_commitments: Vec<[u8; 32]>,
    /// Encrypted notes for new output notes
    pub encrypted_notes: Vec<Vec<u8>>,
    /// The anchor (root of the commitment tree) used for verification
    pub anchor: [u8; 32],
    /// Fee amount extracted from the value balance
    pub fee_amount: Credits,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}
