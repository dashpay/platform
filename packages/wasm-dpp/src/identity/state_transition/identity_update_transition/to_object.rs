use dpp::identity::KeyID;
use dpp::state_transition::StateTransitionIdentitySignedV0;
use dpp::{
    identifier::Identifier,
    identity::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreation,
    identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition,
    state_transition::StateTransitionLike,
};
use serde::Deserialize;
use std::default::Default;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToObjectOptions {
    pub skip_signature: Option<bool>,
}

#[derive(Default)]
pub struct ToObject {
    pub transition_type: u8,
    pub protocol_version: u32,
    pub revision: u32,
    pub signature: Option<Vec<u8>>,
    pub signature_public_key_id: Option<KeyID>,
    pub public_keys_disabled_at: Option<u64>,
    pub public_keys_to_add: Option<Vec<IdentityPublicKeyInCreation>>,
    pub public_key_ids_to_disable: Option<Vec<KeyID>>,
    pub identity_id: Identifier,
}

pub fn to_object_struct(
    transition: &IdentityUpdateTransition,
    options: ToObjectOptions,
) -> ToObject {
    let mut to_object = ToObject {
        transition_type: transition.state_transition_type() as u8,
        protocol_version: transition.state_transition_protocol_version(),
        revision: transition.get_revision() as u32,
        identity_id: transition.get_identity_id().to_owned(),
        ..ToObject::default()
    };

    if !options.skip_signature.unwrap_or(false) {
        let signature = Some(transition.signature().to_vec());
        if let Some(signature) = &signature {
            if !signature.is_empty() {
                to_object.signature_public_key_id = transition.get_signature_public_key_id()
            }
        }
        to_object.signature = signature;
    }

    to_object.public_keys_disabled_at = transition.get_public_keys_disabled_at();

    let public_keys_to_add = transition.get_public_keys_to_add();
    if !public_keys_to_add.is_empty() {
        to_object.public_keys_to_add = Some(public_keys_to_add.to_owned());
    }

    let public_key_ids_to_disable = transition.get_public_key_ids_to_disable();
    if !public_key_ids_to_disable.is_empty() {
        to_object.public_key_ids_to_disable = Some(public_key_ids_to_disable.to_owned());
    }

    to_object
}
