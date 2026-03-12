/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_create::v0::{
    IdentityCreateTransitionActionV0, IdentityFromIdentityCreateTransitionActionV0,
};
use derive_more::From;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
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

    /// Asset lock value to be consumed
    /// The initial balance is equal to the remaining credit value in the asset lock value
    pub fn asset_lock_value_to_be_consumed(&self) -> &AssetLockValue {
        match self {
            IdentityCreateTransitionAction::V0(transition) => {
                &transition.asset_lock_value_to_be_consumed
            }
        }
    }

    /// Asset lock value to be consumed
    /// The initial balance is equal to the remaining credit value in the asset lock value
    pub fn asset_lock_value_to_be_consumed_owned(self) -> AssetLockValue {
        match self {
            IdentityCreateTransitionAction::V0(transition) => {
                transition.asset_lock_value_to_be_consumed
            }
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
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            IdentityCreateTransitionAction::V0(transition) => transition.user_fee_increase,
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
    fn try_from_identity_create_transition_action_returning_asset_lock_value(
        value: IdentityCreateTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, AssetLockValue), ProtocolError>
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
    fn try_from_identity_create_transition_action_returning_asset_lock_value(
        value: IdentityCreateTransitionAction,
        platform_version: &PlatformVersion,
    ) -> Result<(Self, AssetLockValue), ProtocolError> {
        match value {
            IdentityCreateTransitionAction::V0(v0) => {
                Identity::try_from_identity_create_transition_action_returning_asset_lock_value_v0(
                    v0,
                    platform_version,
                )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::identity::identity_create::v0::IdentityCreateTransitionActionV0;
    use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValue, AssetLockValueGettersV0};
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::IdentityPublicKey;
    use dpp::platform_value::{Bytes32, Bytes36};
    use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
    use dpp::version::PlatformVersion;

    fn make_asset_lock_value() -> AssetLockValue {
        let platform_version = PlatformVersion::latest();
        AssetLockValue::new(1000, vec![1, 2, 3], 500, vec![], platform_version)
            .expect("expected asset lock value")
    }

    fn make_v0() -> IdentityCreateTransitionActionV0 {
        let platform_version = PlatformVersion::latest();
        let (key, _private) =
            IdentityPublicKey::random_masternode_transfer_key(1, Some(42), platform_version)
                .expect("expected a random key");
        IdentityCreateTransitionActionV0 {
            signable_bytes_hasher: SignableBytesHasher::PreHashed(Bytes32([0xCC; 32])),
            public_keys: vec![key],
            asset_lock_value_to_be_consumed: make_asset_lock_value(),
            identity_id: Identifier::from([0xAA; 32]),
            asset_lock_outpoint: Bytes36([0xDD; 36]),
            user_fee_increase: 5,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action: IdentityCreateTransitionAction = v0.into();
        assert!(matches!(action, IdentityCreateTransitionAction::V0(_)));
    }

    #[test]
    fn test_public_keys() {
        let action = IdentityCreateTransitionAction::V0(make_v0());
        assert_eq!(action.public_keys().len(), 1);
    }

    #[test]
    fn test_asset_lock_value_to_be_consumed() {
        let action = IdentityCreateTransitionAction::V0(make_v0());
        let alv = action.asset_lock_value_to_be_consumed();
        assert_eq!(alv.remaining_credit_value(), 500);
        assert_eq!(alv.initial_credit_value(), 1000);
    }

    #[test]
    fn test_asset_lock_value_to_be_consumed_owned() {
        let action = IdentityCreateTransitionAction::V0(make_v0());
        let alv = action.asset_lock_value_to_be_consumed_owned();
        assert_eq!(alv.remaining_credit_value(), 500);
    }

    #[test]
    fn test_identity_id() {
        let action = IdentityCreateTransitionAction::V0(make_v0());
        assert_eq!(action.identity_id(), Identifier::from([0xAA; 32]));
    }

    #[test]
    fn test_asset_lock_outpoint() {
        let action = IdentityCreateTransitionAction::V0(make_v0());
        assert_eq!(action.asset_lock_outpoint(), Bytes36([0xDD; 36]));
    }

    #[test]
    fn test_user_fee_increase() {
        let action = IdentityCreateTransitionAction::V0(make_v0());
        assert_eq!(action.user_fee_increase(), 5);
    }

    #[test]
    fn test_into_partial_identity_owned() {
        let action = IdentityCreateTransitionAction::V0(make_v0());
        let partial: PartialIdentity = action.into();
        assert_eq!(partial.id, Identifier::from([0xAA; 32]));
        assert_eq!(partial.balance, Some(500));
        assert!(partial.revision.is_none());
    }

    #[test]
    fn test_into_partial_identity_borrowed() {
        let action = IdentityCreateTransitionAction::V0(make_v0());
        let partial: PartialIdentity = (&action).into();
        assert_eq!(partial.id, Identifier::from([0xAA; 32]));
        assert_eq!(partial.balance, Some(500));
    }

    #[test]
    fn test_try_from_identity_create_transition_action_returning_asset_lock_value() {
        let platform_version = PlatformVersion::latest();
        let action = IdentityCreateTransitionAction::V0(make_v0());
        let (identity, alv) =
            Identity::try_from_identity_create_transition_action_returning_asset_lock_value(
                action,
                platform_version,
            )
            .expect("expected identity");
        assert_eq!(identity.id(), Identifier::from([0xAA; 32]));
        assert_eq!(alv.remaining_credit_value(), 500);
    }

    #[test]
    fn test_try_from_borrowed_identity_create_transition_action() {
        let platform_version = PlatformVersion::latest();
        let action = IdentityCreateTransitionAction::V0(make_v0());
        let identity = Identity::try_from_borrowed_identity_create_transition_action(
            &action,
            platform_version,
        )
        .expect("expected identity");
        assert_eq!(identity.id(), Identifier::from([0xAA; 32]));
    }
}
