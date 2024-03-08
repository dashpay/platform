/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_create::v0::{
    IdentityCreateTransitionActionV0, IdentityFromIdentityCreateTransitionActionV0,
};
use derive_more::From;
use dpp::identity::{Identity, IdentityPublicKey, PartialIdentity};
use dpp::platform_value::{Bytes36, Identifier};
use dpp::prelude::UserFeeIncrease;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;

/// action
#[derive(Debug, Clone, From)]
pub enum IdentityCreateTransitionAction {
    /// v0
    V0(IdentityCreateTransitionActionV0),
}

/// action
impl IdentityCreateTransitionAction {
    /// Public Keys
    pub fn public_keys(&self) -> &Vec<IdentityPublicKey> {
        match self {
            IdentityCreateTransitionAction::V0(transition) => &transition.public_keys,
        }
    }

    /// Initial Balance Amount
    pub fn initial_balance_amount(&self) -> u64 {
        match self {
            IdentityCreateTransitionAction::V0(transition) => transition.initial_balance_amount,
        }
    }

    /// Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreateTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    /// Asset Lock Outpoint
    pub fn asset_lock_outpoint(&self) -> Bytes36 {
        match self {
            IdentityCreateTransitionAction::V0(action) => action.asset_lock_outpoint,
        }
    }

    /// fee multiplier
    pub fn fee_multiplier(&self) -> UserFeeIncrease {
        match self {
            IdentityCreateTransitionAction::V0(transition) => transition.fee_multiplier,
        }
    }
}

impl From<IdentityCreateTransitionAction> for PartialIdentity {
    fn from(value: IdentityCreateTransitionAction) -> Self {
        match value {
            IdentityCreateTransitionAction::V0(v0) => v0.into(),
        }
    }
}

impl From<&IdentityCreateTransitionAction> for PartialIdentity {
    fn from(value: &IdentityCreateTransitionAction) -> Self {
        match value {
            IdentityCreateTransitionAction::V0(v0) => v0.into(),
        }
    }
}

/// action
pub trait IdentityFromIdentityCreateTransitionAction {
    /// try from
    fn try_from_identity_create_transition_action(
        value: IdentityCreateTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
    /// try from borrowed
    fn try_from_borrowed_identity_create_transition_action(
        value: &IdentityCreateTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}

impl IdentityFromIdentityCreateTransitionAction for Identity {
    fn try_from_identity_create_transition_action(
        value: IdentityCreateTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityCreateTransitionAction::V0(v0) => {
                Identity::try_from_identity_create_transition_action_v0(v0, platform_version)
            }
        }
    }

    fn try_from_borrowed_identity_create_transition_action(
        value: &IdentityCreateTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match value {
            IdentityCreateTransitionAction::V0(v0) => {
                Identity::try_from_borrowed_identity_create_transition_action_v0(
                    v0,
                    platform_version,
                )
            }
        }
    }
}
