/// transformer
pub mod transformer;

use dpp::address_funds::fee_strategy::AddressFundsFeeStrategy;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
/// Version 0 of the bump address input nonce action
/// This action is performed when we want to pay for validation of the state transition
/// but not execute it
pub struct BumpAddressInputNoncesActionV0 {
    /// inputs
    pub inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    /// fee strategy for how fees should be deducted
    pub fee_strategy: AddressFundsFeeStrategy,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
}

/// document base transition action accessors v0
pub trait BumpAddressInputNonceActionAccessorsV0 {
    /// Get inputs
    fn inputs_with_remaining_balance(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)>;

    /// Returns owned copies of inputs and outputs.
    fn inputs_with_remaining_balance_and_outputs_owned(
        self,
    ) -> (
        BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        BTreeMap<PlatformAddress, Credits>,
    );

    /// Get fee strategy
    fn fee_strategy(&self) -> &AddressFundsFeeStrategy;

    /// fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease;
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
    use dpp::address_funds::PlatformAddress;

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
    fn test_v0_struct_fields() {
        let action = make_v0();
        assert_eq!(action.inputs_with_remaining_balance.len(), 1);
        let addr = PlatformAddress::P2pkh([0xBB_u8; 20]);
        let (nonce, credits) = action.inputs_with_remaining_balance.get(&addr).unwrap();
        assert_eq!(*nonce, 10);
        assert_eq!(*credits, 5000);
        assert_eq!(action.fee_strategy.len(), 1);
        assert_eq!(action.user_fee_increase, 3);
    }

    #[test]
    fn test_v0_debug_impl() {
        let action = make_v0();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("BumpAddressInputNoncesActionV0"));
    }

    #[test]
    fn test_v0_clone() {
        let action = make_v0();
        let cloned = action.clone();
        assert_eq!(
            cloned.inputs_with_remaining_balance,
            action.inputs_with_remaining_balance
        );
        assert_eq!(cloned.fee_strategy, action.fee_strategy);
        assert_eq!(cloned.user_fee_increase, action.user_fee_increase);
    }

    #[test]
    fn test_v0_empty_inputs() {
        let action = BumpAddressInputNoncesActionV0 {
            inputs_with_remaining_balance: BTreeMap::new(),
            fee_strategy: vec![],
            user_fee_increase: 0,
        };
        assert!(action.inputs_with_remaining_balance.is_empty());
        assert!(action.fee_strategy.is_empty());
        assert_eq!(action.user_fee_increase, 0);
    }

    #[test]
    fn test_v0_multiple_inputs() {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xAA_u8; 20]), (1_u32, 1000_u64));
        inputs.insert(PlatformAddress::P2sh([0xCC_u8; 20]), (2_u32, 2000_u64));

        let action = BumpAddressInputNoncesActionV0 {
            inputs_with_remaining_balance: inputs,
            fee_strategy: vec![
                AddressFundsFeeStrategyStep::DeductFromInput(0),
                AddressFundsFeeStrategyStep::ReduceOutput(1),
            ],
            user_fee_increase: 10,
        };
        assert_eq!(action.inputs_with_remaining_balance.len(), 2);
        assert_eq!(action.fee_strategy.len(), 2);
    }
}
