mod transformer;

use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use dpp::withdrawal::Pooling;

/// Shielded withdrawal transition action v0
#[derive(Debug, Clone)]
pub struct ShieldedWithdrawalTransitionActionV0 {
    /// Withdrawal amount in credits
    pub amount: Credits,
    /// Notes from the orchard bundle actions
    pub notes: Vec<ShieldedActionNote>,
    /// Merkle root used for spends
    pub anchor: [u8; 32],
    /// Core transaction fee rate
    pub core_fee_per_byte: u32,
    /// Withdrawal pooling strategy
    pub pooling: Pooling,
    /// Core address receiving funds
    pub output_script: CoreScript,
    /// Shielded fee paid to proposers, carved out of `amount` (the net amount
    /// withdrawn to Core is `amount - fee_amount`). Equals `compute_shielded_withdrawal_fee`
    /// (the base shielded minimum fee plus the flat Core withdrawal-document storage cost).
    pub fee_amount: Credits,
    /// Current total balance of the shielded pool
    pub current_total_balance: Credits,
    /// Pre-built withdrawal document (status: QUEUED)
    pub prepared_withdrawal_document: Document,
}
