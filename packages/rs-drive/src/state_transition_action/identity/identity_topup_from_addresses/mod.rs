/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
use derive_more::From;
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
    pub fn inputs_with_remaining_balance(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
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

    /// Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityTopUpFromAddressesTransitionAction::V0(transition) => transition.identity_id,
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
}
