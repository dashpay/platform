use dpp::identity::KeyID;

use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use dpp::state_transition::StateTransitionIdentitySigned;
use dpp::{identifier::Identifier, state_transition::StateTransitionLike};
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
        transition_type: transition.state_transition_type() as u8,
        identity_id: transition.identity_id(),
        recipient_id: transition.recipient_id(),
        amount: transition.amount(),
        ..ToObject::default()
    };

    if !options.skip_signature.unwrap_or(false) {
        let signature = Some(transition.signature().to_vec());
        if let Some(signature) = &signature {
            if !signature.is_empty() {
                to_object.signature_public_key_id = Some(transition.signature_public_key_id())
            }
        }
        to_object.signature = signature;
    }

    to_object
}
