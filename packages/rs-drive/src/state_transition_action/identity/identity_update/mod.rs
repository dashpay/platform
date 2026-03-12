/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_update::v0::IdentityUpdateTransitionActionV0;
use derive_more::From;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::platform_value::Identifier;
use dpp::prelude::{IdentityNonce, Revision, UserFeeIncrease};

/// action
#[derive(Debug, Clone, From)]
pub enum IdentityUpdateTransitionAction {
    /// v0
    V0(IdentityUpdateTransitionActionV0),
}

impl IdentityUpdateTransitionAction {
    /// Public Keys
    pub fn public_keys_to_add(&self) -> &Vec<IdentityPublicKey> {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => &transition.add_public_keys,
        }
    }
    /// Disable Public Keys
    pub fn public_keys_to_disable(&self) -> &Vec<KeyID> {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => &transition.disable_public_keys,
        }
    }

    /// Public Keys to Add and Disable Owned
    pub fn public_keys_to_add_and_disable_owned(self) -> (Vec<IdentityPublicKey>, Vec<KeyID>) {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => {
                (transition.add_public_keys, transition.disable_public_keys)
            }
        }
    }

    /// Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    /// Revision
    pub fn revision(&self) -> Revision {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => transition.revision,
        }
    }

    /// Nonce
    pub fn nonce(&self) -> IdentityNonce {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => transition.nonce,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            IdentityUpdateTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::identity::identity_update::v0::IdentityUpdateTransitionActionV0;
    use dpp::identity::IdentityPublicKey;
    use dpp::version::PlatformVersion;

    fn make_v0() -> IdentityUpdateTransitionActionV0 {
        let platform_version = PlatformVersion::latest();
        let (key, _private) =
            IdentityPublicKey::random_masternode_transfer_key(1, Some(42), platform_version)
                .expect("expected a random key");
        IdentityUpdateTransitionActionV0 {
            add_public_keys: vec![key],
            disable_public_keys: vec![5, 10],
            identity_id: Identifier::from([0xAA; 32]),
            revision: 7,
            nonce: 99,
            user_fee_increase: 2,
        }
    }

    #[test]
    fn test_from_v0() {
        let v0 = make_v0();
        let action: IdentityUpdateTransitionAction = v0.into();
        assert!(matches!(action, IdentityUpdateTransitionAction::V0(_)));
    }

    #[test]
    fn test_public_keys_to_add() {
        let action = IdentityUpdateTransitionAction::V0(make_v0());
        assert_eq!(action.public_keys_to_add().len(), 1);
    }

    #[test]
    fn test_public_keys_to_disable() {
        let action = IdentityUpdateTransitionAction::V0(make_v0());
        assert_eq!(action.public_keys_to_disable(), &vec![5u32, 10]);
    }

    #[test]
    fn test_public_keys_to_add_and_disable_owned() {
        let action = IdentityUpdateTransitionAction::V0(make_v0());
        let (add, disable) = action.public_keys_to_add_and_disable_owned();
        assert_eq!(add.len(), 1);
        assert_eq!(disable, vec![5u32, 10]);
    }

    #[test]
    fn test_identity_id() {
        let action = IdentityUpdateTransitionAction::V0(make_v0());
        assert_eq!(action.identity_id(), Identifier::from([0xAA; 32]));
    }

    #[test]
    fn test_revision() {
        let action = IdentityUpdateTransitionAction::V0(make_v0());
        assert_eq!(action.revision(), 7);
    }

    #[test]
    fn test_nonce() {
        let action = IdentityUpdateTransitionAction::V0(make_v0());
        assert_eq!(action.nonce(), 99);
    }

    #[test]
    fn test_user_fee_increase() {
        let action = IdentityUpdateTransitionAction::V0(make_v0());
        assert_eq!(action.user_fee_increase(), 2);
    }
}
