mod transformer;

use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// Shield transition action v0
#[derive(Default, Debug, Clone)]
pub struct ShieldTransitionActionV0 {
    /// inputs with remaining balance after shielding
    pub inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// The amount being shielded (sent into the shielded pool)
    pub shield_amount: Credits,
    /// Note commitments from the orchard bundle (cmx values)
    pub note_commitments: Vec<[u8; 32]>,
    /// Encrypted notes from the orchard bundle
    pub encrypted_notes: Vec<Vec<u8>>,
    /// fee strategy
    pub fee_strategy: AddressFundsFeeStrategy,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    /// Current checkpoint ID counter read from shielded pool params
    pub current_checkpoint_id: u64,
    /// Current total balance of the shielded pool
    pub current_total_balance: Credits,
}
