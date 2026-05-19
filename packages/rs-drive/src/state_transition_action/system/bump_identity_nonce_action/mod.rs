use derive_more::From;
use dpp::platform_value::Identifier;

use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// transformer module
pub mod transformer;
mod v0;

pub use v0::*;

/// bump identity nonce action
#[derive(Debug, Clone, From)]
pub enum BumpIdentityNonceAction {
    /// v0
    V0(BumpIdentityNonceActionV0),
}

impl BumpIdentityNonceActionAccessorsV0 for BumpIdentityNonceAction {
    fn identity_id(&self) -> Identifier {
        match self {
            BumpIdentityNonceAction::V0(v0) => v0.identity_id,
        }
    }

    fn identity_nonce(&self) -> IdentityNonce {
        match self {
            BumpIdentityNonceAction::V0(v0) => v0.identity_nonce,
        }
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            BumpIdentityNonceAction::V0(transition) => transition.user_fee_increase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_v0() -> BumpIdentityNonceActionV0 {
        BumpIdentityNonceActionV0 {
            identity_id: Identifier::from([0xAA_u8; 32]),
            identity_nonce: 42,
            user_fee_increase: 5,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action: BumpIdentityNonceAction = BumpIdentityNonceAction::from(v0.clone());
        assert!(matches!(action, BumpIdentityNonceAction::V0(_)));
    }

    #[test]
    fn test_into_conversion() {
        let v0 = make_v0();
        let action: BumpIdentityNonceAction = v0.into();
        assert!(matches!(action, BumpIdentityNonceAction::V0(_)));
    }

    #[test]
    fn test_enum_accessor_identity_id() {
        let action: BumpIdentityNonceAction = make_v0().into();
        assert_eq!(action.identity_id(), Identifier::from([0xAA_u8; 32]));
    }

    #[test]
    fn test_enum_accessor_identity_nonce() {
        let action: BumpIdentityNonceAction = make_v0().into();
        assert_eq!(action.identity_nonce(), 42);
    }

    #[test]
    fn test_enum_accessor_user_fee_increase() {
        let action: BumpIdentityNonceAction = make_v0().into();
        assert_eq!(action.user_fee_increase(), 5);
    }

    #[test]
    fn test_enum_debug() {
        let action: BumpIdentityNonceAction = make_v0().into();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }

    #[test]
    fn test_enum_clone_preserves_values() {
        let action: BumpIdentityNonceAction = make_v0().into();
        let cloned = action.clone();
        assert_eq!(cloned.identity_id(), action.identity_id());
        assert_eq!(cloned.identity_nonce(), action.identity_nonce());
        assert_eq!(cloned.user_fee_increase(), action.user_fee_increase());
    }

    #[test]
    fn test_enum_accessors_with_zero_values() {
        let v0 = BumpIdentityNonceActionV0 {
            identity_id: Identifier::from([0x00_u8; 32]),
            identity_nonce: 0,
            user_fee_increase: 0,
        };
        let action: BumpIdentityNonceAction = v0.into();
        assert_eq!(action.identity_id(), Identifier::from([0x00_u8; 32]));
        assert_eq!(action.identity_nonce(), 0);
        assert_eq!(action.user_fee_increase(), 0);
    }
}
