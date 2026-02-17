mod transformer;

use dpp::fee::Credits;
use dpp::prelude::UserFeeIncrease;

/// Shield from asset lock transition action v0
#[derive(Debug, Clone)]
pub struct ShieldFromAssetLockTransitionActionV0 {
    /// Asset lock outpoint bytes (txid + vout)
    pub asset_lock_outpoint: [u8; 36],
    /// Remaining asset lock value to be consumed
    pub asset_lock_value_to_be_consumed: Credits,
    /// SHA256(signable_bytes) for replay protection
    pub signable_bytes_hasher: [u8; 32],
    /// Amount going into shielded pool (|value_balance|)
    pub shield_amount: Credits,
    /// Nullifiers from the orchard bundle actions (needed for Rho derivation in trial decryption)
    pub nullifiers: Vec<[u8; 32]>,
    /// Note commitments from the orchard bundle (cmx values)
    pub note_commitments: Vec<[u8; 32]>,
    /// Encrypted notes from the orchard bundle
    pub encrypted_notes: Vec<Vec<u8>>,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    /// Current total balance of the shielded pool
    pub current_total_balance: Credits,
}
