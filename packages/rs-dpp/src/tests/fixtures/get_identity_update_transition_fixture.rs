use crate::{
    identity::{
        state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition,
        KeyType, Purpose, SecurityLevel,
    },
    prelude::IdentityPublicKey,
    state_transition::StateTransitionType,
    tests::utils::generate_random_identifier_struct,
    version::LATEST_VERSION,
};

pub fn get_identity_update_transition_fixture() -> IdentityUpdateTransition {
    IdentityUpdateTransition {
        protocol_version: LATEST_VERSION,
        transition_type: StateTransitionType::IdentityUpdate,
        identity_id: generate_random_identifier_struct(),
        revision: 0,
        add_public_keys: vec![IdentityPublicKey {
            id: 3,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            read_only: false,
            data: base64::decode("AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH").unwrap(),
            security_level: SecurityLevel::MASTER,
            disabled_at: None,
        }],
        disable_public_keys: vec![0],
        public_keys_disabled_at: Some(1234567),
        ..Default::default()
    }
}
