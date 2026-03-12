use derive_more::From;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::platform_value::{Bytes32, Bytes36};
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

mod transformer;
mod v0;

pub use v0::{PartiallyUseAssetLockActionAccessorsV0, PartiallyUseAssetLockActionV0};

#[derive(Debug, Clone, From)]
/// An action expressing that an asset lock should be partially used
pub enum PartiallyUseAssetLockAction {
    /// v0
    V0(PartiallyUseAssetLockActionV0),
}

impl PartiallyUseAssetLockActionAccessorsV0 for PartiallyUseAssetLockAction {
    fn asset_lock_outpoint(&self) -> Bytes36 {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => transition.asset_lock_outpoint,
        }
    }

    fn initial_credit_value(&self) -> Credits {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => transition.initial_credit_value,
        }
    }

    fn asset_lock_script(&self) -> &Vec<u8> {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => &transition.asset_lock_script,
        }
    }

    fn asset_lock_script_owned(self) -> Vec<u8> {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => transition.asset_lock_script,
        }
    }

    fn remaining_credit_value(&self) -> Credits {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => transition.remaining_credit_value,
        }
    }

    fn used_credits(&self) -> Credits {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => transition.used_credits,
        }
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => transition.user_fee_increase,
        }
    }

    fn previous_transaction_hashes_ref(&self) -> &Vec<Bytes32> {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => &transition.previous_transaction_hashes,
        }
    }

    fn inputs_with_remaining_balance(
        &self,
    ) -> Option<&BTreeMap<PlatformAddress, (AddressNonce, Credits)>> {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => {
                transition.inputs_with_remaining_balance.as_ref()
            }
        }
    }

    fn fee_strategy(&self) -> Option<&AddressFundsFeeStrategy> {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => transition.fee_strategy.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;

    fn make_v0() -> PartiallyUseAssetLockActionV0 {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xBB_u8; 20]), (5_u32, 3000_u64));

        PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new([0xCC_u8; 36]),
            initial_credit_value: 10000,
            previous_transaction_hashes: vec![Bytes32([0xDD_u8; 32])],
            asset_lock_script: vec![0x76, 0xA9, 0x14],
            remaining_credit_value: 7000,
            used_credits: 3000,
            user_fee_increase: 2,
            inputs_with_remaining_balance: Some(inputs),
            fee_strategy: Some(vec![AddressFundsFeeStrategyStep::DeductFromInput(0)]),
        }
    }

    fn make_v0_minimal() -> PartiallyUseAssetLockActionV0 {
        PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new([0xAA_u8; 36]),
            initial_credit_value: 5000,
            previous_transaction_hashes: vec![],
            asset_lock_script: vec![],
            remaining_credit_value: 5000,
            used_credits: 0,
            user_fee_increase: 0,
            inputs_with_remaining_balance: None,
            fee_strategy: None,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action = PartiallyUseAssetLockAction::from(v0);
        assert!(matches!(action, PartiallyUseAssetLockAction::V0(_)));
    }

    #[test]
    fn test_into_conversion() {
        let v0 = make_v0();
        let action: PartiallyUseAssetLockAction = v0.into();
        assert!(matches!(action, PartiallyUseAssetLockAction::V0(_)));
    }

    #[test]
    fn test_enum_accessor_asset_lock_outpoint() {
        let action: PartiallyUseAssetLockAction = make_v0().into();
        assert_eq!(action.asset_lock_outpoint(), Bytes36::new([0xCC_u8; 36]));
    }

    #[test]
    fn test_enum_accessor_initial_credit_value() {
        let action: PartiallyUseAssetLockAction = make_v0().into();
        assert_eq!(action.initial_credit_value(), 10000);
    }

    #[test]
    fn test_enum_accessor_asset_lock_script() {
        let action: PartiallyUseAssetLockAction = make_v0().into();
        assert_eq!(action.asset_lock_script(), &vec![0x76_u8, 0xA9, 0x14]);
    }

    #[test]
    fn test_enum_accessor_asset_lock_script_owned() {
        let action: PartiallyUseAssetLockAction = make_v0().into();
        let script = action.asset_lock_script_owned();
        assert_eq!(script, vec![0x76_u8, 0xA9, 0x14]);
    }

    #[test]
    fn test_enum_accessor_remaining_credit_value() {
        let action: PartiallyUseAssetLockAction = make_v0().into();
        assert_eq!(action.remaining_credit_value(), 7000);
    }

    #[test]
    fn test_enum_accessor_used_credits() {
        let action: PartiallyUseAssetLockAction = make_v0().into();
        assert_eq!(action.used_credits(), 3000);
    }

    #[test]
    fn test_enum_accessor_user_fee_increase() {
        let action: PartiallyUseAssetLockAction = make_v0().into();
        assert_eq!(action.user_fee_increase(), 2);
    }

    #[test]
    fn test_enum_accessor_previous_transaction_hashes_ref() {
        let action: PartiallyUseAssetLockAction = make_v0().into();
        let hashes = action.previous_transaction_hashes_ref();
        assert_eq!(hashes.len(), 1);
        assert_eq!(hashes[0], Bytes32([0xDD_u8; 32]));
    }

    #[test]
    fn test_enum_accessor_inputs_with_remaining_balance_some() {
        let action: PartiallyUseAssetLockAction = make_v0().into();
        let inputs = action.inputs_with_remaining_balance();
        assert!(inputs.is_some());
        let inputs = inputs.unwrap();
        assert_eq!(inputs.len(), 1);
        let addr = PlatformAddress::P2pkh([0xBB_u8; 20]);
        let (nonce, credits) = inputs.get(&addr).unwrap();
        assert_eq!(*nonce, 5);
        assert_eq!(*credits, 3000);
    }

    #[test]
    fn test_enum_accessor_inputs_with_remaining_balance_none() {
        let action: PartiallyUseAssetLockAction = make_v0_minimal().into();
        assert!(action.inputs_with_remaining_balance().is_none());
    }

    #[test]
    fn test_enum_accessor_fee_strategy_some() {
        let action: PartiallyUseAssetLockAction = make_v0().into();
        let strategy = action.fee_strategy();
        assert!(strategy.is_some());
        let strategy = strategy.unwrap();
        assert_eq!(strategy.len(), 1);
        assert_eq!(strategy[0], AddressFundsFeeStrategyStep::DeductFromInput(0));
    }

    #[test]
    fn test_enum_accessor_fee_strategy_none() {
        let action: PartiallyUseAssetLockAction = make_v0_minimal().into();
        assert!(action.fee_strategy().is_none());
    }

    #[test]
    fn test_enum_debug() {
        let action: PartiallyUseAssetLockAction = make_v0().into();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }

    #[test]
    fn test_enum_clone_preserves_values() {
        let action: PartiallyUseAssetLockAction = make_v0().into();
        let cloned = action.clone();
        assert_eq!(cloned.asset_lock_outpoint(), action.asset_lock_outpoint());
        assert_eq!(cloned.initial_credit_value(), action.initial_credit_value());
        assert_eq!(cloned.remaining_credit_value(), action.remaining_credit_value());
        assert_eq!(cloned.used_credits(), action.used_credits());
        assert_eq!(cloned.user_fee_increase(), action.user_fee_increase());
        assert_eq!(cloned.asset_lock_script(), action.asset_lock_script());
        assert_eq!(
            cloned.previous_transaction_hashes_ref(),
            action.previous_transaction_hashes_ref()
        );
    }

    #[test]
    fn test_enum_asset_lock_script_owned_consumes() {
        let action: PartiallyUseAssetLockAction = make_v0().into();
        let cloned = action.clone();
        // owned method consumes the value
        let script = cloned.asset_lock_script_owned();
        assert_eq!(script, vec![0x76_u8, 0xA9, 0x14]);
        // original still valid
        assert_eq!(action.asset_lock_script(), &vec![0x76_u8, 0xA9, 0x14]);
    }

    #[test]
    fn test_enum_with_empty_script_and_no_hashes() {
        let action: PartiallyUseAssetLockAction = make_v0_minimal().into();
        assert!(action.asset_lock_script().is_empty());
        assert!(action.previous_transaction_hashes_ref().is_empty());
        let script = action.asset_lock_script_owned();
        assert!(script.is_empty());
    }

    #[test]
    fn test_enum_with_multiple_previous_hashes() {
        let v0 = PartiallyUseAssetLockActionV0 {
            previous_transaction_hashes: vec![
                Bytes32([0x11_u8; 32]),
                Bytes32([0x22_u8; 32]),
                Bytes32([0x33_u8; 32]),
            ],
            ..PartiallyUseAssetLockActionV0::default()
        };
        let action: PartiallyUseAssetLockAction = v0.into();
        let hashes = action.previous_transaction_hashes_ref();
        assert_eq!(hashes.len(), 3);
        assert_eq!(hashes[0], Bytes32([0x11_u8; 32]));
        assert_eq!(hashes[1], Bytes32([0x22_u8; 32]));
        assert_eq!(hashes[2], Bytes32([0x33_u8; 32]));
    }
}
