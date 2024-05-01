use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::identity_update_transition::IdentityUpdateTransition;
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;

use crate::version::PlatformVersion;
use crate::{
    identity::{KeyType, Purpose, SecurityLevel},
    tests::utils::generate_random_identifier_struct,
};
use platform_value::string_encoding::Encoding;
use platform_value::BinaryData;

pub fn get_identity_update_transition_fixture(
    platform_version: PlatformVersion,
) -> IdentityUpdateTransition {
    match platform_version
        .dpp
        .state_transition_serialization_versions
        .identity_update_state_transition
        .default_current_version
    {
        0 => IdentityUpdateTransitionV0 {
            signature: BinaryData::new(vec![0; 65]),
            signature_public_key_id: 0,
            identity_id: generate_random_identifier_struct(),
            revision: 0,
            add_public_keys: vec![IdentityPublicKeyInCreationV0 {
                id: 3,
                key_type: KeyType::ECDSA_SECP256K1,
                purpose: Purpose::AUTHENTICATION,
                read_only: false,
                data: BinaryData::from_string(
                    "AkVuTKyF3YgKLAQlLEtaUL2HTditwGILfWUVqjzYnIgH",
                    Encoding::Base64,
                )
                .unwrap(),
                security_level: SecurityLevel::MASTER,
                signature: BinaryData::new(vec![0; 65]),
                contract_bounds: None,
            }
            .into()],
            disable_public_keys: vec![0],
            ..Default::default()
        }
        .into(),
        _ => unimplemented!("not yet implemented get_identity_update_transition_fixture"),
    }
}
