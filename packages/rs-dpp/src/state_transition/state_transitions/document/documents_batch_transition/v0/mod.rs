mod identity_signed;
#[cfg(feature = "json-object")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "platform-value")]
mod value_conversion;
#[cfg(feature = "cbor")]
mod cbor_conversion;

use crate::identity::KeyID;
use crate::platform_serialization::PlatformSignable;
use crate::serialization_traits::PlatformSerializable;
use crate::serialization_traits::{PlatformDeserializable, Signable};
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use crate::ProtocolError;
use bincode::{config, Decode, Encode};
use platform_serialization::{PlatformDeserialize, PlatformSerialize};
use platform_value::btreemap_extensions::{
    BTreeValueMapHelper, BTreeValueMapReplacementPathHelper,
};
use platform_value::string_encoding::Encoding;
use platform_value::{BinaryData, Identifier, Value};
use std::collections::{BTreeMap, HashMap};
use serde::{Serialize, Deserialize};

#[derive(
    Debug,
    Encode,
    Decode,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSignable,
)]
#[platform_error_type(ProtocolError)]
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
