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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_v0() -> PenalizeShieldedPoolActionV0 {
        PenalizeShieldedPoolActionV0 {
            penalty_amount: 500,
            nullifiers: vec![[0xAA_u8; 32], [0xBB_u8; 32]],
            current_total_balance: 100_000,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action = PenalizeShieldedPoolAction::from(v0);
        assert!(matches!(action, PenalizeShieldedPoolAction::V0(_)));
    }

    #[test]
    fn test_into_conversion() {
        let v0 = make_v0();
        let action: PenalizeShieldedPoolAction = v0.into();
        assert!(matches!(action, PenalizeShieldedPoolAction::V0(_)));
    }

    #[test]
    fn test_enum_accessor_penalty_amount() {
        let action: PenalizeShieldedPoolAction = make_v0().into();
        assert_eq!(action.penalty_amount(), 500);
    }

    #[test]
    fn test_enum_accessor_nullifiers() {
        let action: PenalizeShieldedPoolAction = make_v0().into();
        let nullifiers = action.nullifiers();
        assert_eq!(nullifiers.len(), 2);
        assert_eq!(nullifiers[0], [0xAA_u8; 32]);
        assert_eq!(nullifiers[1], [0xBB_u8; 32]);
    }

    #[test]
    fn test_enum_accessor_current_total_balance() {
        let action: PenalizeShieldedPoolAction = make_v0().into();
        assert_eq!(action.current_total_balance(), 100_000);
    }

    #[test]
    fn test_enum_debug() {
        let action: PenalizeShieldedPoolAction = make_v0().into();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }

    #[test]
    fn test_enum_clone_preserves_values() {
        let action: PenalizeShieldedPoolAction = make_v0().into();
        let cloned = action.clone();
        assert_eq!(cloned.penalty_amount(), action.penalty_amount());
        assert_eq!(cloned.nullifiers(), action.nullifiers());
        assert_eq!(cloned.current_total_balance(), action.current_total_balance());
    }

    #[test]
    fn test_enum_with_empty_nullifiers() {
        let v0 = PenalizeShieldedPoolActionV0 {
            penalty_amount: 0,
            nullifiers: vec![],
            current_total_balance: 0,
        };
        let action: PenalizeShieldedPoolAction = v0.into();
        assert_eq!(action.penalty_amount(), 0);
        assert!(action.nullifiers().is_empty());
        assert_eq!(action.current_total_balance(), 0);
    }

    #[test]
    fn test_enum_with_max_values() {
        let v0 = PenalizeShieldedPoolActionV0 {
            penalty_amount: u64::MAX,
            nullifiers: vec![[0xFF_u8; 32]],
            current_total_balance: u64::MAX,
        };
        let action: PenalizeShieldedPoolAction = v0.into();
        assert_eq!(action.penalty_amount(), u64::MAX);
        assert_eq!(action.nullifiers().len(), 1);
        assert_eq!(action.current_total_balance(), u64::MAX);
    }

    #[test]
    fn test_enum_nullifiers_returns_slice() {
        let action: PenalizeShieldedPoolAction = make_v0().into();
        // Verify the return type is a slice reference
        let nullifiers: &[[u8; 32]] = action.nullifiers();
        assert_eq!(nullifiers.len(), 2);
    }
}
