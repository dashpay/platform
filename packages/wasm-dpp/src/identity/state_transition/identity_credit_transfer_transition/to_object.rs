use dpp::identity::KeyID;
use dpp::state_transition::StateTransitionIdentitySignedV0;
use dpp::{
    identifier::Identifier,
    identity::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition,
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
    pub identity_id: Identifier,
    pub recipient_id: Identifier,
    pub amount: u64,
    pub signature: Option<Vec<u8>>,
    pub signature_public_key_id: Option<KeyID>,
}

pub fn to_object_struct(
    transition: &IdentityCreditTransferTransition,
    options: ToObjectOptions,
) -> ToObject {
    let mut to_object = ToObject {
        transition_type: transition.get_type() as u8,
        protocol_version: transition.get_protocol_version(),
        identity_id: *transition.get_identity_id(),
        recipient_id: *transition.get_recipient_id(),
        amount: transition.get_amount(),
        ..ToObject::default()
    };

    if !options.skip_signature.unwrap_or(false) {
        let signature = Some(transition.get_signature().to_vec());
        if let Some(signature) = &signature {
            if !signature.is_empty() {
                to_object.signature_public_key_id = transition.get_signature_public_key_id()
            }
        }
        to_object.signature = signature;
    }

    to_object
}
