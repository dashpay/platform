/// Penalize the shielded pool when proof verification fails for pool-spending transitions
pub mod v0;

use derive_more::From;
use v0::PenalizeShieldedPoolActionV0;

/// Action to deduct a penalty from the shielded pool and record nullifiers as spent
#[derive(Debug, Clone, From)]
pub enum PenalizeShieldedPoolAction {
    /// V0
    V0(PenalizeShieldedPoolActionV0),
}

/// Accessors for PenalizeShieldedPoolAction
pub trait PenalizeShieldedPoolActionAccessorsV0 {
    /// The penalty amount to deduct from the pool
    fn penalty_amount(&self) -> u64;
    /// The nullifiers to record as spent (prevents replay)
    fn nullifiers(&self) -> &[[u8; 32]];
    /// Current total pool balance
    fn current_total_balance(&self) -> u64;
}

impl PenalizeShieldedPoolActionAccessorsV0 for PenalizeShieldedPoolAction {
    fn penalty_amount(&self) -> u64 {
        match self {
            PenalizeShieldedPoolAction::V0(v0) => v0.penalty_amount,
        }
    }
    fn nullifiers(&self) -> &[[u8; 32]] {
        match self {
            PenalizeShieldedPoolAction::V0(v0) => &v0.nullifiers,
        }
    }
    fn current_total_balance(&self) -> u64 {
        match self {
            PenalizeShieldedPoolAction::V0(v0) => v0.current_total_balance,
        }
    }
}
