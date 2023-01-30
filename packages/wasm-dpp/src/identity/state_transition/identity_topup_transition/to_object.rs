use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
use dpp::{
    identifier::Identifier,
    identity::state_transition::identity_topup_transition::IdentityTopUpTransition,
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
    pub asset_lock_proof: AssetLockProof,
    pub signature: Option<Vec<u8>>,
}

pub fn to_object_struct(
    transition: &IdentityTopUpTransition,
    options: ToObjectOptions,
) -> ToObject {
    let mut to_object = ToObject {
        transition_type: transition.get_type() as u8,
        protocol_version: transition.get_protocol_version(),
        identity_id: transition.get_identity_id().clone(),
        asset_lock_proof: transition.get_asset_lock_proof().to_owned(),
        signature: None,
    };

    if !options.skip_signature.unwrap_or(false) {
        to_object.signature = Some(transition.get_signature().to_owned());
    }

    to_object
}
