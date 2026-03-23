use derive_more::From;
use dpp::address_funds::fee_strategy::AddressFundsFeeStrategy;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use std::collections::BTreeMap;

use dpp::prelude::{AddressNonce, UserFeeIncrease};

/// transformer module
pub mod transformer;
mod v0;

pub use v0::*;

/// bump address_input nonce action
#[derive(Debug, Clone, From)]
pub enum BumpAddressInputNoncesAction {
    /// v0
    V0(BumpAddressInputNoncesActionV0),
}

impl BumpAddressInputNonceActionAccessorsV0 for BumpAddressInputNoncesAction {
    fn inputs_with_remaining_balance(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            BumpAddressInputNoncesAction::V0(v0) => &v0.inputs_with_remaining_balance,
        }
    }

    fn inputs_with_remaining_balance_and_outputs_owned(
        self,
    ) -> (
        BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        BTreeMap<PlatformAddress, Credits>,
    ) {
        match self {
            BumpAddressInputNoncesAction::V0(v0) => {
                (v0.inputs_with_remaining_balance, BTreeMap::new())
            }
        }
    }

    fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            BumpAddressInputNoncesAction::V0(v0) => &v0.fee_strategy,
        }
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            BumpAddressInputNoncesAction::V0(v0) => v0.user_fee_increase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;

    fn make_v0() -> BumpAddressInputNoncesActionV0 {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xBB_u8; 20]), (10_u32, 5000_u64));
        BumpAddressInputNoncesActionV0 {
            inputs_with_remaining_balance: inputs,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 3,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action = BumpAddressInputNoncesAction::from(v0);
        assert!(matches!(action, BumpAddressInputNoncesAction::V0(_)));
    }

    #[test]
    fn test_into_conversion() {
        let v0 = make_v0();
        let action: BumpAddressInputNoncesAction = v0.into();
        assert!(matches!(action, BumpAddressInputNoncesAction::V0(_)));
    }

    #[test]
    fn test_enum_accessor_inputs_with_remaining_balance() {
        let action: BumpAddressInputNoncesAction = make_v0().into();
        let inputs = action.inputs_with_remaining_balance();
        assert_eq!(inputs.len(), 1);
        let addr = PlatformAddress::P2pkh([0xBB_u8; 20]);
        let (nonce, credits) = inputs.get(&addr).unwrap();
        assert_eq!(*nonce, 10);
        assert_eq!(*credits, 5000);
    }

    #[test]
    fn test_enum_accessor_fee_strategy() {
        let action: BumpAddressInputNoncesAction = make_v0().into();
        let strategy = action.fee_strategy();
        assert_eq!(strategy.len(), 1);
        assert_eq!(strategy[0], AddressFundsFeeStrategyStep::DeductFromInput(0));
    }

    #[test]
    fn test_enum_accessor_user_fee_increase() {
        let action: BumpAddressInputNoncesAction = make_v0().into();
        assert_eq!(action.user_fee_increase(), 3);
    }

    #[test]
    fn test_enum_inputs_with_remaining_balance_and_outputs_owned() {
        let action: BumpAddressInputNoncesAction = make_v0().into();
        let (inputs, outputs) = action.inputs_with_remaining_balance_and_outputs_owned();
        assert_eq!(inputs.len(), 1);
        let addr = PlatformAddress::P2pkh([0xBB_u8; 20]);
        let (nonce, credits) = inputs.get(&addr).unwrap();
        assert_eq!(*nonce, 10);
        assert_eq!(*credits, 5000);
        assert!(outputs.is_empty());
    }

    #[test]
    fn test_enum_debug() {
        let action: BumpAddressInputNoncesAction = make_v0().into();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }

    #[test]
    fn test_enum_clone_preserves_values() {
        let action: BumpAddressInputNoncesAction = make_v0().into();
        let cloned = action.clone();
        assert_eq!(
            cloned.inputs_with_remaining_balance(),
            action.inputs_with_remaining_balance()
        );
        assert_eq!(cloned.fee_strategy(), action.fee_strategy());
        assert_eq!(cloned.user_fee_increase(), action.user_fee_increase());
    }

    #[test]
    fn test_enum_owned_method_consumes_action() {
        let action: BumpAddressInputNoncesAction = make_v0().into();
        let cloned = action.clone();
        // Call the owned method on the clone (consumes it)
        let (inputs, outputs) = cloned.inputs_with_remaining_balance_and_outputs_owned();
        assert_eq!(inputs.len(), 1);
        assert!(outputs.is_empty());
        // Original is still valid
        assert_eq!(action.user_fee_increase(), 3);
    }

    #[test]
    fn test_enum_with_empty_inputs() {
        let v0 = BumpAddressInputNoncesActionV0 {
            inputs_with_remaining_balance: BTreeMap::new(),
            fee_strategy: vec![],
            user_fee_increase: 0,
        };
        let action: BumpAddressInputNoncesAction = v0.into();
        assert!(action.inputs_with_remaining_balance().is_empty());
        assert!(action.fee_strategy().is_empty());
        assert_eq!(action.user_fee_increase(), 0);
        let (inputs, outputs) = action.inputs_with_remaining_balance_and_outputs_owned();
        assert!(inputs.is_empty());
        assert!(outputs.is_empty());
    }
}
