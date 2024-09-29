use dpp::identity::KeyID;

use dpp::identity::core_script::CoreScript;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::state_transition::StateTransitionIdentitySigned;
use dpp::withdrawal::Pooling;
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
    pub amount: u64,
    pub core_fee_per_byte: u32,
    pub pooling: Pooling,
    pub output_script: Option<CoreScript>,
    pub nonce: IdentityNonce,
    pub signature: Option<Vec<u8>>,
    pub signature_public_key_id: Option<KeyID>,
}

pub fn to_object_struct(
    transition: &IdentityCreditWithdrawalTransition,
    options: ToObjectOptions,
) -> ToObject {
    let mut to_object = ToObject {
        transition_type: transition.state_transition_type() as u8,
        identity_id: transition.identity_id(),
        amount: transition.amount(),
        core_fee_per_byte: transition.core_fee_per_byte(),
        pooling: transition.pooling(),
        output_script: transition.output_script(),
        nonce: transition.nonce(),
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
