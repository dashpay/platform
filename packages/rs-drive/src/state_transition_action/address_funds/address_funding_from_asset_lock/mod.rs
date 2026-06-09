/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::address_funds::address_funding_from_asset_lock::v0::AddressFundingFromAssetLockTransitionActionV0;
use derive_more::From;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::fee::Credits;
use dpp::platform_value::Bytes36;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action
#[derive(Debug, Clone, From)]
pub enum AddressFundingFromAssetLockTransitionAction {
    /// v0
    V0(AddressFundingFromAssetLockTransitionActionV0),
}

impl AddressFundingFromAssetLockTransitionAction {
    /// Get inputs with remaining balance
    pub fn inputs_with_remaining_balance(
        &self,
    ) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => {
                &transition.inputs_with_remaining_balance
            }
        }
    }

    /// Get outputs (Some = explicit amount, None = remainder recipient)
    pub fn outputs(&self) -> &BTreeMap<PlatformAddress, Option<Credits>> {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => &transition.outputs,
        }
    }

    /// Get asset lock value to be consumed
    pub fn asset_lock_value_to_be_consumed(&self) -> &AssetLockValue {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => {
                &transition.asset_lock_value_to_be_consumed
            }
        }
    }

    /// Get total credits contributed from address inputs
    pub fn input_contributions_total(&self) -> Credits {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => {
                transition.input_contributions_total
            }
        }
    }

    /// Get resolved outputs with remainder computed.
    /// Returns outputs where all Option<Credits> are resolved to concrete Credits values.
    pub fn resolved_outputs(&self) -> BTreeMap<PlatformAddress, Credits> {
        use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;

        let outputs = self.outputs();
        let asset_lock_balance = self
            .asset_lock_value_to_be_consumed()
            .remaining_credit_value();

        // Total available = asset lock + input contributions
        let total_available = asset_lock_balance + self.input_contributions_total();

        // Calculate the sum of explicit outputs
        let explicit_outputs_sum: Credits = outputs.values().flatten().sum();

        // Calculate remainder
        let remainder_balance = total_available.saturating_sub(explicit_outputs_sum);

        // Resolve all outputs
        outputs
            .iter()
            .map(|(address, balance_option)| {
                let balance = match balance_option {
                    Some(explicit_amount) => *explicit_amount,
                    None => remainder_balance,
                };
                (*address, balance)
            })
            .collect()
    }

    /// Returns owned copies of inputs, outputs, asset lock value, and input contributions total.
    #[allow(clippy::type_complexity)]
    pub fn inputs_with_remaining_balance_outputs_and_asset_lock_value_owned(
        self,
    ) -> (
        BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        BTreeMap<PlatformAddress, Option<Credits>>,
        AssetLockValue,
        Credits,
    ) {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => (
                transition.inputs_with_remaining_balance,
                transition.outputs,
                transition.asset_lock_value_to_be_consumed,
                transition.input_contributions_total,
            ),
        }
    }

    /// Asset Lock Outpoint
    pub fn asset_lock_outpoint(&self) -> Bytes36 {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(action) => action.asset_lock_outpoint,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => {
                transition.user_fee_increase
            }
        }
    }

    /// fee strategy
    pub fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => &transition.fee_strategy,
        }
    }

    /// Removes the remainder output (the one with None value) from the action.
    /// This should be called when total available funds exactly match explicit outputs.
    pub fn remove_remainder_output(&mut self) {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => {
                transition.outputs.retain(|_, v| v.is_some());
            }
        }
    }

    /// Restores each input's full balance ahead of the insufficient-funds penalty path.
    ///
    /// `inputs_with_remaining_balance` stores each input's balance *after* its intended spend
    /// (`actual - requested`), which is correct on the success path because the spend funds the
    /// outputs. On the penalty path the transition failed and no outputs are created, so the spend
    /// must not be consumed — only the penalty fee should be charged. This adds each input's
    /// requested spend back so the downstream `PartiallyUseAssetLockAction` deducts the fee from the
    /// full balance, leaving the user with `actual - fee`. Without this, the spend would be removed
    /// from the address without being credited anywhere.
    pub fn restore_input_spends_for_failed_transition(
        &mut self,
        requested_inputs: &BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => {
                for (address, (_nonce, balance)) in
                    transition.inputs_with_remaining_balance.iter_mut()
                {
                    if let Some((_, requested_spend)) = requested_inputs.get(address) {
                        *balance = balance.saturating_add(*requested_spend);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
    use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
    use dpp::platform_value::Bytes36;
    use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
    use platform_version::version::PlatformVersion;

    fn make_asset_lock_value(remaining: Credits) -> AssetLockValue {
        AssetLockValue::new(
            remaining + 1000, // initial > remaining
            vec![0xDE, 0xAD],
            remaining,
            vec![],
            PlatformVersion::latest(),
        )
        .expect("should create AssetLockValue")
    }

    fn make_action() -> AddressFundingFromAssetLockTransitionAction {
        let addr_input = PlatformAddress::P2pkh([0xAA; 20]);
        let addr_explicit = PlatformAddress::P2pkh([0xBB; 20]);
        let addr_remainder = PlatformAddress::P2sh([0xCC; 20]);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr_input, (1_u32, 2000_u64));

        let mut outputs = BTreeMap::new();
        outputs.insert(addr_explicit, Some(3000_u64));
        outputs.insert(addr_remainder, None); // remainder recipient

        let v0 = AddressFundingFromAssetLockTransitionActionV0 {
            signable_bytes_hasher: SignableBytesHasher::Bytes(vec![0xFF; 8]),
            asset_lock_value_to_be_consumed: make_asset_lock_value(5000),
            asset_lock_outpoint: Bytes36([0xDD; 36]),
            inputs_with_remaining_balance: inputs,
            outputs,
            input_contributions_total: 2000,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 5,
        };
        AddressFundingFromAssetLockTransitionAction::from(v0)
    }

    #[test]
    fn test_from_v0() {
        let action = make_action();
        assert!(matches!(
            action,
            AddressFundingFromAssetLockTransitionAction::V0(_)
        ));
    }

    #[test]
    fn test_inputs_with_remaining_balance() {
        let action = make_action();
        let inputs = action.inputs_with_remaining_balance();
        assert_eq!(inputs.len(), 1);
        let addr = PlatformAddress::P2pkh([0xAA; 20]);
        let (nonce, balance) = inputs.get(&addr).unwrap();
        assert_eq!(*nonce, 1);
        assert_eq!(*balance, 2000);
    }

    #[test]
    fn test_outputs() {
        let action = make_action();
        let outputs = action.outputs();
        assert_eq!(outputs.len(), 2);
        let addr_explicit = PlatformAddress::P2pkh([0xBB; 20]);
        let addr_remainder = PlatformAddress::P2sh([0xCC; 20]);
        assert_eq!(*outputs.get(&addr_explicit).unwrap(), Some(3000_u64));
        assert_eq!(*outputs.get(&addr_remainder).unwrap(), None);
    }

    #[test]
    fn test_asset_lock_value_to_be_consumed() {
        use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;

        let action = make_action();
        let alv = action.asset_lock_value_to_be_consumed();
        assert_eq!(alv.remaining_credit_value(), 5000);
    }

    #[test]
    fn test_input_contributions_total() {
        let action = make_action();
        assert_eq!(action.input_contributions_total(), 2000);
    }

    #[test]
    fn test_resolved_outputs() {
        let action = make_action();
        // total_available = 5000 (asset lock remaining) + 2000 (input contributions) = 7000
        // explicit_outputs_sum = 3000
        // remainder = 7000 - 3000 = 4000
        let resolved = action.resolved_outputs();
        assert_eq!(resolved.len(), 2);
        let addr_explicit = PlatformAddress::P2pkh([0xBB; 20]);
        let addr_remainder = PlatformAddress::P2sh([0xCC; 20]);
        assert_eq!(*resolved.get(&addr_explicit).unwrap(), 3000);
        assert_eq!(*resolved.get(&addr_remainder).unwrap(), 4000);
    }

    #[test]
    fn test_resolved_outputs_all_explicit() {
        // When there's no remainder recipient, all outputs should match their explicit values.
        let addr = PlatformAddress::P2pkh([0xBB; 20]);
        let mut outputs = BTreeMap::new();
        outputs.insert(addr, Some(7000_u64));

        let v0 = AddressFundingFromAssetLockTransitionActionV0 {
            signable_bytes_hasher: SignableBytesHasher::Bytes(vec![]),
            asset_lock_value_to_be_consumed: make_asset_lock_value(5000),
            asset_lock_outpoint: Bytes36([0x00; 36]),
            inputs_with_remaining_balance: BTreeMap::new(),
            outputs,
            input_contributions_total: 2000,
            fee_strategy: vec![],
            user_fee_increase: 0,
        };
        let action = AddressFundingFromAssetLockTransitionAction::from(v0);
        let resolved = action.resolved_outputs();
        assert_eq!(resolved.len(), 1);
        assert_eq!(*resolved.get(&addr).unwrap(), 7000);
    }

    #[test]
    fn test_asset_lock_outpoint() {
        let action = make_action();
        assert_eq!(action.asset_lock_outpoint(), Bytes36([0xDD; 36]));
    }

    #[test]
    fn test_user_fee_increase() {
        let action = make_action();
        assert_eq!(action.user_fee_increase(), 5);
    }

    #[test]
    fn test_fee_strategy() {
        let action = make_action();
        let strategy = action.fee_strategy();
        assert_eq!(strategy.len(), 1);
        assert!(matches!(
            strategy[0],
            AddressFundsFeeStrategyStep::DeductFromInput(0)
        ));
    }

    #[test]
    fn test_remove_remainder_output() {
        let mut action = make_action();
        assert_eq!(action.outputs().len(), 2);
        action.remove_remainder_output();
        // Only explicit (Some) outputs should remain.
        assert_eq!(action.outputs().len(), 1);
        let addr_explicit = PlatformAddress::P2pkh([0xBB; 20]);
        assert!(action.outputs().contains_key(&addr_explicit));
    }

    #[test]
    fn test_inputs_outputs_and_asset_lock_value_owned() {
        let action = make_action();
        let (inputs, outputs, alv, contributions) =
            action.inputs_with_remaining_balance_outputs_and_asset_lock_value_owned();

        assert_eq!(inputs.len(), 1);
        assert_eq!(outputs.len(), 2);
        assert_eq!(contributions, 2000);

        use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;
        assert_eq!(alv.remaining_credit_value(), 5000);
    }

    #[test]
    fn test_clone() {
        let action = make_action();
        let cloned = action.clone();
        assert_eq!(cloned.user_fee_increase(), 5);
        assert_eq!(cloned.input_contributions_total(), 2000);
        assert_eq!(cloned.outputs().len(), 2);
    }

    #[test]
    fn test_debug() {
        let action = make_action();
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("V0"));
    }
}
