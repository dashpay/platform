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
    fn test_v0_struct_fields() {
        let action = make_v0();
        assert_eq!(action.penalty_amount, 500);
        assert_eq!(action.nullifiers.len(), 2);
        assert_eq!(action.nullifiers[0], [0xAA_u8; 32]);
        assert_eq!(action.nullifiers[1], [0xBB_u8; 32]);
        assert_eq!(action.current_total_balance, 100_000);
    }

    #[test]
    fn test_v0_debug_impl() {
        let action = make_v0();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("PenalizeShieldedPoolActionV0"));
    }

    #[test]
    fn test_v0_clone() {
        let action = make_v0();
        let cloned = action.clone();
        assert_eq!(cloned.penalty_amount, action.penalty_amount);
        assert_eq!(cloned.nullifiers, action.nullifiers);
        assert_eq!(cloned.current_total_balance, action.current_total_balance);
    }

    #[test]
    fn test_v0_with_zero_values() {
        let action = PenalizeShieldedPoolActionV0 {
            penalty_amount: 0,
            nullifiers: vec![],
            current_total_balance: 0,
        };
        assert_eq!(action.penalty_amount, 0);
        assert!(action.nullifiers.is_empty());
        assert_eq!(action.current_total_balance, 0);
    }

    #[test]
    fn test_v0_with_max_penalty() {
        let action = PenalizeShieldedPoolActionV0 {
            penalty_amount: u64::MAX,
            nullifiers: vec![[0xFF_u8; 32]],
            current_total_balance: u64::MAX,
        };
        assert_eq!(action.penalty_amount, u64::MAX);
        assert_eq!(action.current_total_balance, u64::MAX);
    }

    #[test]
    fn test_v0_single_nullifier() {
        let action = PenalizeShieldedPoolActionV0 {
            penalty_amount: 100,
            nullifiers: vec![[0x01_u8; 32]],
            current_total_balance: 1000,
        };
        assert_eq!(action.nullifiers.len(), 1);
        assert_eq!(action.nullifiers[0], [0x01_u8; 32]);
    }

    #[test]
    fn test_v0_many_nullifiers() {
        let nullifiers: Vec<[u8; 32]> = (0..10).map(|i| [i as u8; 32]).collect();
        let action = PenalizeShieldedPoolActionV0 {
            penalty_amount: 1000,
            nullifiers: nullifiers.clone(),
            current_total_balance: 50_000,
        };
        assert_eq!(action.nullifiers.len(), 10);
        for (i, nf) in action.nullifiers.iter().enumerate() {
            assert_eq!(*nf, [i as u8; 32]);
        }
    }
}
