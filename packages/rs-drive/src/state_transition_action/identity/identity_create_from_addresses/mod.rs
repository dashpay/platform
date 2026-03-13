/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_create_from_addresses::v0::{
    IdentityCreateFromAddressesTransitionActionV0,
    IdentityFromIdentityCreateFromAddressesTransitionActionV0,
};
use derive_more::From;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::{Identity, IdentityPublicKey, PartialIdentity};
use dpp::platform_value::Identifier;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use std::collections::BTreeMap;

/// action
#[derive(Debug, Clone, From)]
pub enum IdentityCreateFromAddressesTransitionAction {
    /// v0
    V0(IdentityCreateFromAddressesTransitionActionV0),
}

/// action
impl IdentityCreateFromAddressesTransitionAction {
    /// Get inputs
    pub fn inputs_with_remaining_balance(
        &self,
    ) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            IdentityCreateFromAddressesTransitionAction::V0(transition) => {
                &transition.inputs_with_remaining_balance
            }
        }
    }
    /// Get inputs
    pub fn inputs_with_remaining_balance_owned(
        self,
    ) -> BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            IdentityCreateFromAddressesTransitionAction::V0(transition) => {
                transition.inputs_with_remaining_balance
            }
        }
    }
    /// Public Keys
    pub fn public_keys(&self) -> &Vec<IdentityPublicKey> {
        match self {
            IdentityCreateFromAddressesTransitionAction::V0(transition) => &transition.public_keys,
        }
    }

    /// Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreateFromAddressesTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            IdentityCreateFromAddressesTransitionAction::V0(transition) => {
                transition.user_fee_increase
            }
        }
    }

    /// Get output
    pub fn output(&self) -> Option<(PlatformAddress, Credits)> {
        match self {
            IdentityCreateFromAddressesTransitionAction::V0(transition) => transition.output,
        }
    }

    /// Get fee strategy
    pub fn fee_strategy(&self) -> &dpp::address_funds::fee_strategy::AddressFundsFeeStrategy {
        match self {
            IdentityCreateFromAddressesTransitionAction::V0(transition) => &transition.fee_strategy,
        }
    }
}

impl From<IdentityCreateFromAddressesTransitionAction> for PartialIdentity {
    fn from(value: IdentityCreateFromAddressesTransitionAction) -> Self {
        match value {
            IdentityCreateFromAddressesTransitionAction::V0(v0) => v0.into(),
        }
    }
}

impl From<&IdentityCreateFromAddressesTransitionAction> for PartialIdentity {
    fn from(value: &IdentityCreateFromAddressesTransitionAction) -> Self {
        match value {
            IdentityCreateFromAddressesTransitionAction::V0(v0) => v0.into(),
        }
    }
}

/// action
pub trait IdentityFromIdentityCreateFromAddressesTransitionAction {
    /// try from borrowed
    fn try_from_borrowed_identity_create_from_addresses_transition_action(
        value: &IdentityCreateFromAddressesTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl IdentityFromIdentityCreateFromAddressesTransitionAction for Identity {
    fn try_from_borrowed_identity_create_from_addresses_transition_action(
        value: &IdentityCreateFromAddressesTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityCreateFromAddressesTransitionAction::V0(v0) => {
                Identity::try_from_borrowed_identity_create_from_addresses_transition_action_v0(
                    v0,
                    platform_version,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategy;
    use dpp::address_funds::PlatformAddress;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::IdentityPublicKey;
    use dpp::version::PlatformVersion;

    fn make_v0() -> IdentityCreateFromAddressesTransitionActionV0 {
        let platform_version = PlatformVersion::latest();
        let (key, _private) =
            IdentityPublicKey::random_masternode_transfer_key(1, Some(42), platform_version)
                .expect("expected a random key");
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0x11; 20]), (1u32, 900u64));
        IdentityCreateFromAddressesTransitionActionV0 {
            inputs_with_remaining_balance: inputs,
            output: Some((PlatformAddress::P2sh([0x22; 20]), 100)),
            fee_strategy: AddressFundsFeeStrategy::default(),
            public_keys: vec![key],
            identity_id: Identifier::from([0xAA; 32]),
            fund_identity_amount: 800,
            user_fee_increase: 2,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action: IdentityCreateFromAddressesTransitionAction = v0.into();
        assert!(matches!(
            action,
            IdentityCreateFromAddressesTransitionAction::V0(_)
        ));
    }

    #[test]
    fn test_inputs_with_remaining_balance() {
        let action = IdentityCreateFromAddressesTransitionAction::V0(make_v0());
        let inputs = action.inputs_with_remaining_balance();
        assert_eq!(inputs.len(), 1);
        let (nonce, credits) = inputs
            .get(&PlatformAddress::P2pkh([0x11; 20]))
            .expect("expected input");
        assert_eq!(*nonce, 1);
        assert_eq!(*credits, 900);
    }

    #[test]
    fn test_inputs_with_remaining_balance_owned() {
        let action = IdentityCreateFromAddressesTransitionAction::V0(make_v0());
        let inputs = action.inputs_with_remaining_balance_owned();
        assert_eq!(inputs.len(), 1);
    }

    #[test]
    fn test_public_keys() {
        let action = IdentityCreateFromAddressesTransitionAction::V0(make_v0());
        assert_eq!(action.public_keys().len(), 1);
    }

    #[test]
    fn test_identity_id() {
        let action = IdentityCreateFromAddressesTransitionAction::V0(make_v0());
        assert_eq!(action.identity_id(), Identifier::from([0xAA; 32]));
    }

    #[test]
    fn test_user_fee_increase() {
        let action = IdentityCreateFromAddressesTransitionAction::V0(make_v0());
        assert_eq!(action.user_fee_increase(), 2);
    }

    #[test]
    fn test_output() {
        let action = IdentityCreateFromAddressesTransitionAction::V0(make_v0());
        let output = action.output();
        assert!(output.is_some());
        let (addr, credits) = output.unwrap();
        assert_eq!(addr, PlatformAddress::P2sh([0x22; 20]));
        assert_eq!(credits, 100);
    }

    #[test]
    fn test_fee_strategy() {
        let action = IdentityCreateFromAddressesTransitionAction::V0(make_v0());
        let strategy = action.fee_strategy();
        assert!(strategy.is_empty()); // default is empty vec
    }

    #[test]
    fn test_into_partial_identity_owned() {
        let action = IdentityCreateFromAddressesTransitionAction::V0(make_v0());
        let partial: PartialIdentity = action.into();
        assert_eq!(partial.id, Identifier::from([0xAA; 32]));
        assert_eq!(partial.balance, Some(800));
        assert!(partial.revision.is_none());
    }

    #[test]
    fn test_into_partial_identity_borrowed() {
        let action = IdentityCreateFromAddressesTransitionAction::V0(make_v0());
        let partial: PartialIdentity = (&action).into();
        assert_eq!(partial.id, Identifier::from([0xAA; 32]));
        assert_eq!(partial.balance, Some(800));
    }

    #[test]
    fn test_try_from_borrowed_identity_create_from_addresses_transition_action() {
        let platform_version = PlatformVersion::latest();
        let action = IdentityCreateFromAddressesTransitionAction::V0(make_v0());
        let identity =
            Identity::try_from_borrowed_identity_create_from_addresses_transition_action(
                &action,
                platform_version,
            )
            .expect("expected identity");
        assert_eq!(identity.id(), Identifier::from([0xAA; 32]));
    }
}
