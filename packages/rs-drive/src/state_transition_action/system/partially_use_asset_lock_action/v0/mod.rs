use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::platform_value::{Bytes32, Bytes36};
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;
mod transformer;

/// Partially use asset lock action v0
#[derive(Default, Debug, Clone)]
pub struct PartiallyUseAssetLockActionV0 {
    /// asset lock outpoint
    pub asset_lock_outpoint: Bytes36,
    /// initial credit value
    pub initial_credit_value: Credits,
    /// the previous transaction signable bytes hashes that tried to used this asset lock, but failed
    pub previous_transaction_hashes: Vec<Bytes32>,
    /// asset lock script
    pub asset_lock_script: Vec<u8>,
    /// remaining credit value AFTER used credits are deducted
    pub remaining_credit_value: Credits,
    /// the used credits for processing, this is what will go to Evonodes for processing
    /// this is after applying the user fee increase
    pub used_credits: Credits,
    /// fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    /// Optional inputs with their remaining balances (for address funding transitions)
    /// The nonce is the current nonce, and Credits is the remaining balance after deducting input amount
    pub inputs_with_remaining_balance: Option<BTreeMap<PlatformAddress, (AddressNonce, Credits)>>,
    /// Optional fee strategy (for address funding transitions)
    /// Specifies the order in which fees should be deducted from inputs/outputs
    pub fee_strategy: Option<AddressFundsFeeStrategy>,
}

/// document base transition action accessors v0
pub trait PartiallyUseAssetLockActionAccessorsV0 {
    /// asset lock outpoint
    fn asset_lock_outpoint(&self) -> Bytes36;
    /// initial credit value
    fn initial_credit_value(&self) -> Credits;
    /// asset lock script used to very that the asset lock can be used
    fn asset_lock_script(&self) -> &Vec<u8>;
    /// asset lock script used to very that the asset lock can be used, this consumes the action
    fn asset_lock_script_owned(self) -> Vec<u8>;
    /// remaining credit value AFTER used credits are deducted
    fn remaining_credit_value(&self) -> Credits;
    /// the used credits for processing, this is what will go to Evonodes for processing
    fn used_credits(&self) -> Credits;
    /// fee multiplier
    fn user_fee_increase(&self) -> UserFeeIncrease;

    /// the previous transaction signable bytes hashes that tried to used this asset lock, but failed
    fn previous_transaction_hashes_ref(&self) -> &Vec<Bytes32>;

    /// Optional inputs with their remaining balances (for address funding transitions)
    fn inputs_with_remaining_balance(
        &self,
    ) -> Option<&BTreeMap<PlatformAddress, (AddressNonce, Credits)>>;

    /// Optional fee strategy (for address funding transitions)
    fn fee_strategy(&self) -> Option<&AddressFundsFeeStrategy>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockAction;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
    use dpp::address_funds::PlatformAddress;
    use dpp::platform_value::{Bytes32, Bytes36};

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

    /// Helper to wrap a V0 struct in the enum so we can call trait methods
    /// through the `PartiallyUseAssetLockActionAccessorsV0` trait interface.
    fn wrap_as_action(v0: PartiallyUseAssetLockActionV0) -> PartiallyUseAssetLockAction {
        PartiallyUseAssetLockAction::V0(v0)
    }

    #[test]
    fn test_v0_default() {
        let action = PartiallyUseAssetLockActionV0::default();
        assert_eq!(action.asset_lock_outpoint, Bytes36::default());
        assert_eq!(action.initial_credit_value, 0);
        assert!(action.previous_transaction_hashes.is_empty());
        assert!(action.asset_lock_script.is_empty());
        assert_eq!(action.remaining_credit_value, 0);
        assert_eq!(action.used_credits, 0);
        assert_eq!(action.user_fee_increase, 0);
        assert!(action.inputs_with_remaining_balance.is_none());
        assert!(action.fee_strategy.is_none());
    }

    #[test]
    fn test_v0_struct_fields() {
        let action = make_v0();
        assert_eq!(action.asset_lock_outpoint, Bytes36::new([0xCC_u8; 36]));
        assert_eq!(action.initial_credit_value, 10000);
        assert_eq!(action.previous_transaction_hashes.len(), 1);
        assert_eq!(
            action.previous_transaction_hashes[0],
            Bytes32([0xDD_u8; 32])
        );
        assert_eq!(action.asset_lock_script, vec![0x76, 0xA9, 0x14]);
        assert_eq!(action.remaining_credit_value, 7000);
        assert_eq!(action.used_credits, 3000);
        assert_eq!(action.user_fee_increase, 2);
        assert!(action.inputs_with_remaining_balance.is_some());
        assert!(action.fee_strategy.is_some());
    }

    #[test]
    fn test_v0_debug_impl() {
        let action = make_v0();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("PartiallyUseAssetLockActionV0"));
    }

    #[test]
    fn test_v0_clone() {
        let action = make_v0();
        let cloned = action.clone();
        assert_eq!(cloned.asset_lock_outpoint, action.asset_lock_outpoint);
        assert_eq!(cloned.initial_credit_value, action.initial_credit_value);
        assert_eq!(
            cloned.previous_transaction_hashes,
            action.previous_transaction_hashes
        );
        assert_eq!(cloned.asset_lock_script, action.asset_lock_script);
        assert_eq!(cloned.remaining_credit_value, action.remaining_credit_value);
        assert_eq!(cloned.used_credits, action.used_credits);
        assert_eq!(cloned.user_fee_increase, action.user_fee_increase);
    }

    #[test]
    fn test_v0_minimal_no_optional_fields() {
        let action = make_v0_minimal();
        assert!(action.inputs_with_remaining_balance.is_none());
        assert!(action.fee_strategy.is_none());
        assert!(action.previous_transaction_hashes.is_empty());
        assert!(action.asset_lock_script.is_empty());
    }

    #[test]
    fn test_v0_multiple_previous_hashes() {
        let action = PartiallyUseAssetLockActionV0 {
            previous_transaction_hashes: vec![
                Bytes32([0x11_u8; 32]),
                Bytes32([0x22_u8; 32]),
                Bytes32([0x33_u8; 32]),
            ],
            ..PartiallyUseAssetLockActionV0::default()
        };
        assert_eq!(action.previous_transaction_hashes.len(), 3);
    }

    #[test]
    fn test_v0_optional_inputs_content() {
        let action = make_v0();
        let inputs = action.inputs_with_remaining_balance.as_ref().unwrap();
        let addr = PlatformAddress::P2pkh([0xBB_u8; 20]);
        let (nonce, credits) = inputs.get(&addr).unwrap();
        assert_eq!(*nonce, 5);
        assert_eq!(*credits, 3000);
    }

    #[test]
    fn test_v0_optional_fee_strategy_content() {
        let action = make_v0();
        let strategy = action.fee_strategy.as_ref().unwrap();
        assert_eq!(strategy.len(), 1);
        assert_eq!(strategy[0], AddressFundsFeeStrategyStep::DeductFromInput(0));
    }

    // --- Tests exercising the PartiallyUseAssetLockActionAccessorsV0 trait interface ---

    #[test]
    fn test_trait_asset_lock_outpoint() {
        let action = wrap_as_action(make_v0());
        assert_eq!(action.asset_lock_outpoint(), Bytes36::new([0xCC_u8; 36]));
    }

    #[test]
    fn test_trait_initial_credit_value() {
        let action = wrap_as_action(make_v0());
        assert_eq!(action.initial_credit_value(), 10000);
    }

    #[test]
    fn test_trait_asset_lock_script() {
        let action = wrap_as_action(make_v0());
        assert_eq!(action.asset_lock_script(), &vec![0x76_u8, 0xA9, 0x14]);
    }

    #[test]
    fn test_trait_asset_lock_script_owned() {
        let action = wrap_as_action(make_v0());
        let script = action.asset_lock_script_owned();
        assert_eq!(script, vec![0x76_u8, 0xA9, 0x14]);
    }

    #[test]
    fn test_trait_remaining_credit_value() {
        let action = wrap_as_action(make_v0());
        assert_eq!(action.remaining_credit_value(), 7000);
    }

    #[test]
    fn test_trait_used_credits() {
        let action = wrap_as_action(make_v0());
        assert_eq!(action.used_credits(), 3000);
    }

    #[test]
    fn test_trait_user_fee_increase() {
        let action = wrap_as_action(make_v0());
        assert_eq!(action.user_fee_increase(), 2);
    }

    #[test]
    fn test_trait_previous_transaction_hashes_ref() {
        let action = wrap_as_action(make_v0());
        let hashes = action.previous_transaction_hashes_ref();
        assert_eq!(hashes.len(), 1);
        assert_eq!(hashes[0], Bytes32([0xDD_u8; 32]));
    }

    #[test]
    fn test_trait_inputs_with_remaining_balance_some() {
        let action = wrap_as_action(make_v0());
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
    fn test_trait_inputs_with_remaining_balance_none() {
        let action = wrap_as_action(make_v0_minimal());
        assert!(action.inputs_with_remaining_balance().is_none());
    }

    #[test]
    fn test_trait_fee_strategy_some() {
        let action = wrap_as_action(make_v0());
        let strategy = action.fee_strategy();
        assert!(strategy.is_some());
        let strategy = strategy.unwrap();
        assert_eq!(strategy.len(), 1);
        assert_eq!(strategy[0], AddressFundsFeeStrategyStep::DeductFromInput(0));
    }

    #[test]
    fn test_trait_fee_strategy_none() {
        let action = wrap_as_action(make_v0_minimal());
        assert!(action.fee_strategy().is_none());
    }

    #[test]
    fn test_trait_accessors_on_default_values() {
        let action = wrap_as_action(PartiallyUseAssetLockActionV0::default());
        assert_eq!(action.asset_lock_outpoint(), Bytes36::default());
        assert_eq!(action.initial_credit_value(), 0);
        assert!(action.asset_lock_script().is_empty());
        assert_eq!(action.remaining_credit_value(), 0);
        assert_eq!(action.used_credits(), 0);
        assert_eq!(action.user_fee_increase(), 0);
        assert!(action.previous_transaction_hashes_ref().is_empty());
        assert!(action.inputs_with_remaining_balance().is_none());
        assert!(action.fee_strategy().is_none());
    }
}
