use crate::state_transition_action::identity::identity_update::v0::IdentityUpdateTransitionActionV0;
use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;

impl From<IdentityUpdateTransitionV0> for IdentityUpdateTransitionActionV0 {
    fn from(value: IdentityUpdateTransitionV0) -> Self {
        let IdentityUpdateTransitionV0 {
            identity_id,
            add_public_keys,
            disable_public_keys,
            public_keys_disabled_at,
            revision,
            nonce,
            ..
        } = value;
        IdentityUpdateTransitionActionV0 {
            add_public_keys: add_public_keys.into_iter().map(|a| a.into()).collect(),
            disable_public_keys,
            public_keys_disabled_at,
            identity_id,
            revision,
            nonce,
        }
    }
}

impl From<&IdentityUpdateTransitionV0> for IdentityUpdateTransitionActionV0 {
    fn from(value: &IdentityUpdateTransitionV0) -> Self {
        let IdentityUpdateTransitionV0 {
            identity_id,
            add_public_keys,
            disable_public_keys,
            public_keys_disabled_at,
            revision,
            nonce,
            ..
        } = value;
        IdentityUpdateTransitionActionV0 {
            add_public_keys: add_public_keys
                .iter()
                .map(|key| key.clone().into())
                .collect(),
            disable_public_keys: disable_public_keys.clone(),
            public_keys_disabled_at: *public_keys_disabled_at,
            identity_id: *identity_id,
            revision: *revision,
            nonce: *nonce,
        }
    }
}
