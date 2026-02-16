mod transformer;

use dpp::document::Document;
use dpp::fee::Credits;
use dpp::identity::core_script::CoreScript;
use dpp::withdrawal::Pooling;

/// Shielded withdrawal transition action v0
#[derive(Debug, Clone)]
pub struct ShieldedWithdrawalTransitionActionV0 {
    /// Withdrawal amount in credits
    pub amount: Credits,
    /// Nullifiers from spent notes
    pub nullifiers: Vec<[u8; 32]>,
    /// Note commitments for change outputs
    pub note_commitments: Vec<[u8; 32]>,
    /// Encrypted change notes
    pub encrypted_notes: Vec<Vec<u8>>,
    /// Merkle root used for spends
    pub anchor: [u8; 32],
    /// Core transaction fee rate
    pub core_fee_per_byte: u32,
    /// Withdrawal pooling strategy
    pub pooling: Pooling,
    /// Core address receiving funds
    pub output_script: CoreScript,
    /// Fee amount (value_balance - amount), paid to proposers
    pub fee_amount: Credits,
    /// Current total balance of the shielded pool
    pub current_total_balance: Credits,
    /// Pre-built withdrawal document (status: QUEUED)
    pub prepared_withdrawal_document: Document,
}
