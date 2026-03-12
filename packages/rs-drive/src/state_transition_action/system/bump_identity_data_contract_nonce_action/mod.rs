use derive_more::From;
use dpp::platform_value::Identifier;

use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// transformer module
pub mod transformer;
mod v0;

pub use v0::*;

/// bump identity data contract nonce action
#[derive(Debug, Clone, From)]
pub enum BumpIdentityDataContractNonceAction {
    /// v0
    V0(BumpIdentityDataContractNonceActionV0),
}

impl BumpIdentityDataContractNonceActionAccessorsV0 for BumpIdentityDataContractNonceAction {
    fn identity_id(&self) -> Identifier {
        match self {
            BumpIdentityDataContractNonceAction::V0(v0) => v0.identity_id,
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            BumpIdentityDataContractNonceAction::V0(v0) => v0.data_contract_id,
        }
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            BumpIdentityDataContractNonceAction::V0(v0) => v0.identity_contract_nonce,
        }
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            BumpIdentityDataContractNonceAction::V0(transition) => transition.user_fee_increase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_v0() -> BumpIdentityDataContractNonceActionV0 {
        BumpIdentityDataContractNonceActionV0 {
            identity_id: Identifier::from([0xAA_u8; 32]),
            data_contract_id: Identifier::from([0xBB_u8; 32]),
            identity_contract_nonce: 99,
            user_fee_increase: 7,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action = BumpIdentityDataContractNonceAction::from(v0);
        assert!(matches!(action, BumpIdentityDataContractNonceAction::V0(_)));
    }

    #[test]
    fn test_into_conversion() {
        let v0 = make_v0();
        let action: BumpIdentityDataContractNonceAction = v0.into();
        assert!(matches!(action, BumpIdentityDataContractNonceAction::V0(_)));
    }

    #[test]
    fn test_enum_accessor_identity_id() {
        let action: BumpIdentityDataContractNonceAction = make_v0().into();
        assert_eq!(action.identity_id(), Identifier::from([0xAA_u8; 32]));
    }

    #[test]
    fn test_enum_accessor_data_contract_id() {
        let action: BumpIdentityDataContractNonceAction = make_v0().into();
        assert_eq!(action.data_contract_id(), Identifier::from([0xBB_u8; 32]));
    }

    #[test]
    fn test_enum_accessor_identity_contract_nonce() {
        let action: BumpIdentityDataContractNonceAction = make_v0().into();
        assert_eq!(action.identity_contract_nonce(), 99);
    }

    #[test]
    fn test_enum_accessor_user_fee_increase() {
        let action: BumpIdentityDataContractNonceAction = make_v0().into();
        assert_eq!(action.user_fee_increase(), 7);
    }

    #[test]
    fn test_enum_debug() {
        let action: BumpIdentityDataContractNonceAction = make_v0().into();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }

    #[test]
    fn test_enum_clone_preserves_values() {
        let action: BumpIdentityDataContractNonceAction = make_v0().into();
        let cloned = action.clone();
        assert_eq!(cloned.identity_id(), action.identity_id());
        assert_eq!(cloned.data_contract_id(), action.data_contract_id());
        assert_eq!(cloned.identity_contract_nonce(), action.identity_contract_nonce());
        assert_eq!(cloned.user_fee_increase(), action.user_fee_increase());
    }

    #[test]
    fn test_enum_accessors_with_zero_values() {
        let v0 = BumpIdentityDataContractNonceActionV0 {
            identity_id: Identifier::from([0x00_u8; 32]),
            data_contract_id: Identifier::from([0x00_u8; 32]),
            identity_contract_nonce: 0,
            user_fee_increase: 0,
        };
        let action: BumpIdentityDataContractNonceAction = v0.into();
        assert_eq!(action.identity_id(), Identifier::from([0x00_u8; 32]));
        assert_eq!(action.data_contract_id(), Identifier::from([0x00_u8; 32]));
        assert_eq!(action.identity_contract_nonce(), 0);
        assert_eq!(action.user_fee_increase(), 0);
    }
}
