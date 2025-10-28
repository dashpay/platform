/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_credit_transfer_to_addresses::v0::IdentityCreditTransferToAddressesTransitionActionV0;
use derive_more::From;
use dpp::fee::Credits;
use dpp::identity::KeyOfType;
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

    /// Recipient keys
    pub fn recipient_keys(&self) -> &BTreeMap<KeyOfType, Credits> {
        match self {
            IdentityCreditTransferToAddressesTransitionAction::V0(transition) => {
                &transition.recipient_keys
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
