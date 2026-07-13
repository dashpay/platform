mod identity_signed;
mod state_transition_like;
mod types;
mod v0_methods;
mod v1_methods;
mod version;

use crate::identity::KeyID;

use crate::state_transition::batch_transition::batched_transition::BatchedTransition;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;

use crate::prelude::UserFeeIncrease;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use platform_value::{BinaryData, Identifier};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, PartialEq, Encode, Decode, PlatformSignable)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct BatchTransitionV1 {
    pub owner_id: Identifier,
    pub transitions: Vec<BatchedTransition>,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}
