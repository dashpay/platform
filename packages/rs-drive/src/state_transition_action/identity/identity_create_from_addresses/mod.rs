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
    pub fn inputs_with_remaining_balance(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
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
