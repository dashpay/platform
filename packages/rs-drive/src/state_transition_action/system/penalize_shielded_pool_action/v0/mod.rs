use dpp::fee::Credits;

/// V0 implementation of penalize shielded pool action
#[derive(Debug, Clone)]
pub struct PenalizeShieldedPoolActionV0 {
    /// The penalty amount to deduct from the pool
    pub penalty_amount: Credits,
    /// Nullifiers to record as spent (prevents exact replay of the same invalid proof)
    pub nullifiers: Vec<[u8; 32]>,
    /// The current total balance of the pool before penalty
    pub current_total_balance: Credits,
}
