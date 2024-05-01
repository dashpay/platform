use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
use dpp::identity::state_transition::AssetLockProved;
use dpp::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
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
    pub asset_lock_proof: AssetLockProof,
    pub signature: Option<Vec<u8>>,
}

pub fn to_object_struct(
    transition: &IdentityTopUpTransition,
    options: &ToObjectOptions,
) -> ToObject {
    let mut to_object = ToObject {
        transition_type: transition.state_transition_type() as u8,
        identity_id: *transition.identity_id(),
        asset_lock_proof: transition.asset_lock_proof().to_owned(),
        signature: None,
    };

    if !options.skip_signature.unwrap_or(false) {
        to_object.signature = Some(transition.signature().to_vec());
    }

    to_object
}
