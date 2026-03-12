/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_credit_transfer_to_addresses::v0::IdentityCreditTransferToAddressesTransitionActionV0;
use derive_more::From;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action
#[derive(Debug, Clone, From)]
pub enum IdentityCreditTransferToAddressesTransitionAction {
    /// v0
    V0(IdentityCreditTransferToAddressesTransitionActionV0),
}

impl IdentityCreditTransferToAddressesTransitionAction {
    /// Nonce
    pub fn nonce(&self) -> IdentityNonce {
        match self {
            IdentityCreditTransferToAddressesTransitionAction::V0(transition) => transition.nonce,
        }
    }

    /// Recipient addresses
    pub fn recipient_addresses(&self) -> &BTreeMap<PlatformAddress, Credits> {
        match self {
            IdentityCreditTransferToAddressesTransitionAction::V0(transition) => {
                &transition.recipient_addresses
            }
        }
    }

    /// Recipient addresses
    pub fn recipient_addresses_owned(self) -> BTreeMap<PlatformAddress, Credits> {
        match self {
            IdentityCreditTransferToAddressesTransitionAction::V0(transition) => {
                transition.recipient_addresses
            }
        }
    }

    /// Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreditTransferToAddressesTransitionAction::V0(transition) => {
                transition.identity_id
            }
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            IdentityCreditTransferToAddressesTransitionAction::V0(transition) => {
                transition.user_fee_increase
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::identity::identity_credit_transfer_to_addresses::v0::IdentityCreditTransferToAddressesTransitionActionV0;
    use dpp::address_funds::PlatformAddress;

    fn make_v0() -> IdentityCreditTransferToAddressesTransitionActionV0 {
        let mut recipients = BTreeMap::new();
        recipients.insert(PlatformAddress::P2pkh([0x11; 20]), 1000);
        recipients.insert(PlatformAddress::P2sh([0x22; 20]), 2000);
        IdentityCreditTransferToAddressesTransitionActionV0 {
            recipient_addresses: recipients,
            identity_id: Identifier::from([0xAA; 32]),
            nonce: 55,
            user_fee_increase: 1,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action: IdentityCreditTransferToAddressesTransitionAction = v0.into();
        assert!(matches!(
            action,
            IdentityCreditTransferToAddressesTransitionAction::V0(_)
        ));
    }

    #[test]
    fn test_nonce() {
        let action = IdentityCreditTransferToAddressesTransitionAction::V0(make_v0());
        assert_eq!(action.nonce(), 55);
    }

    #[test]
    fn test_recipient_addresses() {
        let action = IdentityCreditTransferToAddressesTransitionAction::V0(make_v0());
        let addrs = action.recipient_addresses();
        assert_eq!(addrs.len(), 2);
        assert_eq!(
            addrs.get(&PlatformAddress::P2pkh([0x11; 20])),
            Some(&1000)
        );
        assert_eq!(addrs.get(&PlatformAddress::P2sh([0x22; 20])), Some(&2000));
    }

    #[test]
    fn test_recipient_addresses_owned() {
        let action = IdentityCreditTransferToAddressesTransitionAction::V0(make_v0());
        let addrs = action.recipient_addresses_owned();
        assert_eq!(addrs.len(), 2);
    }

    #[test]
    fn test_identity_id() {
        let action = IdentityCreditTransferToAddressesTransitionAction::V0(make_v0());
        assert_eq!(action.identity_id(), Identifier::from([0xAA; 32]));
    }

    #[test]
    fn test_user_fee_increase() {
        let action = IdentityCreditTransferToAddressesTransitionAction::V0(make_v0());
        assert_eq!(action.user_fee_increase(), 1);
    }
}
