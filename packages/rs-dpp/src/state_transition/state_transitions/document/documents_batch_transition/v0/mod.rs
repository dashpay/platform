#[cfg(feature = "state-transition-cbor-conversion")]
mod cbor_conversion;
mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::identity::KeyID;
use crate::serialization::PlatformSerializable;
use crate::serialization::{PlatformDeserializable, Signable};
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use crate::ProtocolError;
use bincode::{config, Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_value::btreemap_extensions::{
    BTreeValueMapHelper, BTreeValueMapReplacementPathHelper,
};
use platform_value::string_encoding::Encoding;
use platform_value::{BinaryData, Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq, PlatformDeserialize, PlatformSerialize, PlatformSignable)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize)
)]

pub struct DocumentsBatchTransitionV0 {
    pub owner_id: Identifier,
    pub transitions: Vec<DocumentTransition>,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl Default for DocumentsBatchTransitionV0 {
    fn default() -> Self {
        DocumentsBatchTransitionV0 {
            owner_id: Identifier::default(),
            transitions: vec![],
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        }
    }
}
