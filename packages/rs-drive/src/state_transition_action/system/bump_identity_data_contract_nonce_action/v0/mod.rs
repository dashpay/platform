/// transformer
pub mod transformer;

use dpp::identifier::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

#[derive(Debug, Clone)]
/// Version 0 of the bump identity data contract nonce action
/// This action is performed when we want to pay for the state transition
pub struct BumpIdentityDataContractNonceActionV0 {
    /// The identity id
    pub identity_id: Identifier,
    /// The contract id
    pub data_contract_id: Identifier,
    /// The identity contract nonce, this is used to stop replay attacks
    pub identity_contract_nonce: IdentityNonce,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}

/// document base transition action accessors v0
pub trait BumpIdentityDataContractNonceActionAccessorsV0 {
    /// The identity id
    fn identity_id(&self) -> Identifier;
    /// The contract id
    fn data_contract_id(&self) -> Identifier;
    /// Identity contract nonce
    fn identity_contract_nonce(&self) -> IdentityNonce;
    /// Fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease;
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identifier::Identifier;

    fn make_v0() -> BumpIdentityDataContractNonceActionV0 {
        BumpIdentityDataContractNonceActionV0 {
            identity_id: Identifier::from([0xAA_u8; 32]),
            data_contract_id: Identifier::from([0xBB_u8; 32]),
            identity_contract_nonce: 99,
            user_fee_increase: 7,
        }
    }

    #[test]
    fn test_v0_struct_fields() {
        let action = make_v0();
        assert_eq!(action.identity_id, Identifier::from([0xAA_u8; 32]));
        assert_eq!(action.data_contract_id, Identifier::from([0xBB_u8; 32]));
        assert_eq!(action.identity_contract_nonce, 99);
        assert_eq!(action.user_fee_increase, 7);
    }

    #[test]
    fn test_v0_debug_impl() {
        let action = make_v0();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("BumpIdentityDataContractNonceActionV0"));
    }

    #[test]
    fn test_v0_clone() {
        let action = make_v0();
        let cloned = action.clone();
        assert_eq!(cloned.identity_id, action.identity_id);
        assert_eq!(cloned.data_contract_id, action.data_contract_id);
        assert_eq!(
            cloned.identity_contract_nonce,
            action.identity_contract_nonce
        );
        assert_eq!(cloned.user_fee_increase, action.user_fee_increase);
    }

    #[test]
    fn test_v0_fields_with_zero_values() {
        let action = BumpIdentityDataContractNonceActionV0 {
            identity_id: Identifier::from([0x00_u8; 32]),
            data_contract_id: Identifier::from([0x00_u8; 32]),
            identity_contract_nonce: 0,
            user_fee_increase: 0,
        };
        assert_eq!(action.identity_id, Identifier::from([0x00_u8; 32]));
        assert_eq!(action.data_contract_id, Identifier::from([0x00_u8; 32]));
        assert_eq!(action.identity_contract_nonce, 0);
        assert_eq!(action.user_fee_increase, 0);
    }

    #[test]
    fn test_v0_fields_with_max_values() {
        let action = BumpIdentityDataContractNonceActionV0 {
            identity_id: Identifier::from([0xFF_u8; 32]),
            data_contract_id: Identifier::from([0xFF_u8; 32]),
            identity_contract_nonce: u64::MAX,
            user_fee_increase: u16::MAX,
        };
        assert_eq!(action.identity_id, Identifier::from([0xFF_u8; 32]));
        assert_eq!(action.data_contract_id, Identifier::from([0xFF_u8; 32]));
        assert_eq!(action.identity_contract_nonce, u64::MAX);
        assert_eq!(action.user_fee_increase, u16::MAX);
    }

    #[test]
    fn test_v0_different_identity_and_contract_ids() {
        let action = BumpIdentityDataContractNonceActionV0 {
            identity_id: Identifier::from([0x11_u8; 32]),
            data_contract_id: Identifier::from([0x22_u8; 32]),
            identity_contract_nonce: 1,
            user_fee_increase: 0,
        };
        assert_ne!(action.identity_id, action.data_contract_id);
    }
}
