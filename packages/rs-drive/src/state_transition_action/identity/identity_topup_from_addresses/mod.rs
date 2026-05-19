/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
use derive_more::From;
use dpp::address_funds::fee_strategy::AddressFundsFeeStrategy;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action
#[derive(Debug, Clone, From)]
pub enum IdentityTopUpFromAddressesTransitionAction {
    /// v0
    V0(IdentityTopUpFromAddressesTransitionActionV0),
}

impl IdentityTopUpFromAddressesTransitionAction {
    /// Get inputs
    pub fn inputs_with_remaining_balance(
        &self,
    ) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            IdentityTopUpFromAddressesTransitionAction::V0(transition) => {
                &transition.inputs_with_remaining_balance
            }
        }
    }
    /// Get inputs
    pub fn inputs_with_remaining_balance_owned(
        self,
    ) -> BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            IdentityTopUpFromAddressesTransitionAction::V0(transition) => {
                transition.inputs_with_remaining_balance
            }
        }
    }

    /// Get output address and amount (if any)
    pub fn output(&self) -> Option<(PlatformAddress, Credits)> {
        match self {
            IdentityTopUpFromAddressesTransitionAction::V0(transition) => transition.output,
        }
    }

    /// Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityTopUpFromAddressesTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    /// Get the amount to add to the identity's balance
    pub fn topup_amount(&self) -> Credits {
        match self {
            IdentityTopUpFromAddressesTransitionAction::V0(transition) => transition.topup_amount,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            IdentityTopUpFromAddressesTransitionAction::V0(transition) => {
                transition.user_fee_increase
            }
        }
    }

    /// fee strategy
    pub fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            IdentityTopUpFromAddressesTransitionAction::V0(transition) => &transition.fee_strategy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategy;
    use dpp::address_funds::PlatformAddress;

    fn make_v0() -> IdentityTopUpFromAddressesTransitionActionV0 {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0x11; 20]), (1u32, 900u64));
        inputs.insert(PlatformAddress::P2sh([0x22; 20]), (2u32, 500u64));
        IdentityTopUpFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance: inputs,
            output: Some((PlatformAddress::P2pkh([0x33; 20]), 200)),
            fee_strategy: AddressFundsFeeStrategy::default(),
            identity_id: Identifier::from([0xAA; 32]),
            topup_amount: 1200,
            user_fee_increase: 3,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action: IdentityTopUpFromAddressesTransitionAction = v0.into();
        assert!(matches!(
            action,
            IdentityTopUpFromAddressesTransitionAction::V0(_)
        ));
    }

    #[test]
    fn test_inputs_with_remaining_balance() {
        let action = IdentityTopUpFromAddressesTransitionAction::V0(make_v0());
        let inputs = action.inputs_with_remaining_balance();
        assert_eq!(inputs.len(), 2);
    }

    #[test]
    fn test_inputs_with_remaining_balance_owned() {
        let action = IdentityTopUpFromAddressesTransitionAction::V0(make_v0());
        let inputs = action.inputs_with_remaining_balance_owned();
        assert_eq!(inputs.len(), 2);
    }

    #[test]
    fn test_output() {
        let action = IdentityTopUpFromAddressesTransitionAction::V0(make_v0());
        let output = action.output();
        assert!(output.is_some());
        let (addr, credits) = output.unwrap();
        assert_eq!(addr, PlatformAddress::P2pkh([0x33; 20]));
        assert_eq!(credits, 200);
    }

    #[test]
    fn test_output_none() {
        let mut v0 = make_v0();
        v0.output = None;
        let action = IdentityTopUpFromAddressesTransitionAction::V0(v0);
        assert!(action.output().is_none());
    }

    #[test]
    fn test_identity_id() {
        let action = IdentityTopUpFromAddressesTransitionAction::V0(make_v0());
        assert_eq!(action.identity_id(), Identifier::from([0xAA; 32]));
    }

    #[test]
    fn test_topup_amount() {
        let action = IdentityTopUpFromAddressesTransitionAction::V0(make_v0());
        assert_eq!(action.topup_amount(), 1200);
    }

    #[test]
    fn test_user_fee_increase() {
        let action = IdentityTopUpFromAddressesTransitionAction::V0(make_v0());
        assert_eq!(action.user_fee_increase(), 3);
    }

    #[test]
    fn test_fee_strategy() {
        let action = IdentityTopUpFromAddressesTransitionAction::V0(make_v0());
        let strategy = action.fee_strategy();
        assert!(strategy.is_empty()); // default is empty vec
    }
}
