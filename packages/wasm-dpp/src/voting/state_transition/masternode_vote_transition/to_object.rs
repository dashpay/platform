use dpp::identity::KeyID;

use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::state_transition::StateTransitionIdentitySigned;
use dpp::voting::votes::Vote;
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
    pub pro_tx_hash: Identifier,
    pub vote: Vote,
    pub signature: Option<Vec<u8>>,
    pub signature_public_key_id: Option<KeyID>,
}

pub fn to_object_struct(
    transition: &MasternodeVoteTransition,
    options: ToObjectOptions,
) -> ToObject {
    let mut to_object = ToObject {
        transition_type: transition.state_transition_type() as u8,
        vote: transition.vote().clone(),
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
