/// transformer
pub mod transformer;

use dpp::identifier::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

#[derive(Debug, Clone)]
/// Version 0 of the bump identity nonce action
/// This action is performed when we want to pay for validation of the state transition
/// but not execute it
pub struct BumpIdentityNonceActionV0 {
    /// The identity id
    pub identity_id: Identifier,
    /// The identity contract nonce, this is used to stop replay attacks
    pub identity_nonce: IdentityNonce,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}

/// document base transition action accessors v0
pub trait BumpIdentityNonceActionAccessorsV0 {
    /// The identity id
    fn identity_id(&self) -> Identifier;
    /// Identity contract nonce
    fn identity_nonce(&self) -> IdentityNonce;

    /// fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease;
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identifier::Identifier;

    fn make_v0() -> BumpIdentityNonceActionV0 {
        BumpIdentityNonceActionV0 {
            identity_id: Identifier::from([0xAA_u8; 32]),
            identity_nonce: 42,
            user_fee_increase: 5,
        }
    }

    #[test]
    fn test_v0_struct_fields() {
        let action = make_v0();
        assert_eq!(action.identity_id, Identifier::from([0xAA_u8; 32]));
        assert_eq!(action.identity_nonce, 42);
        assert_eq!(action.user_fee_increase, 5);
    }

    #[test]
    fn test_v0_debug_impl() {
        let action = make_v0();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("BumpIdentityNonceActionV0"));
    }

    #[test]
    fn test_v0_clone() {
        let action = make_v0();
        let cloned = action.clone();
        assert_eq!(cloned.identity_id, action.identity_id);
        assert_eq!(cloned.identity_nonce, action.identity_nonce);
        assert_eq!(cloned.user_fee_increase, action.user_fee_increase);
    }

    #[test]
    fn test_v0_fields_with_zero_values() {
        let action = BumpIdentityNonceActionV0 {
            identity_id: Identifier::from([0x00_u8; 32]),
            identity_nonce: 0,
            user_fee_increase: 0,
        };
        assert_eq!(action.identity_id, Identifier::from([0x00_u8; 32]));
        assert_eq!(action.identity_nonce, 0);
        assert_eq!(action.user_fee_increase, 0);
    }

    #[test]
    fn test_v0_fields_with_max_values() {
        let action = BumpIdentityNonceActionV0 {
            identity_id: Identifier::from([0xFF_u8; 32]),
            identity_nonce: u64::MAX,
            user_fee_increase: u16::MAX,
        };
        assert_eq!(action.identity_id, Identifier::from([0xFF_u8; 32]));
        assert_eq!(action.identity_nonce, u64::MAX);
        assert_eq!(action.user_fee_increase, u16::MAX);
    }
}
