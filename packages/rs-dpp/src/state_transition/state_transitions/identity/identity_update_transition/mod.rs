mod fields;
mod identity_signed;
#[cfg(feature = "json-object")]
mod json_conversion;
mod state_transition_like;
mod v0;
mod v0_methods;
#[cfg(feature = "platform-value")]
mod value_conversion;

use fields::*;

use crate::serialization_traits::{PlatformDeserializable, PlatformSerializable, Signable};
use crate::state_transition::identity_update_transition::fields::property_names::ADD_PUBLIC_KEYS_SIGNATURE;
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::StateTransitionFieldTypes;
use crate::ProtocolError;
pub use action::IdentityUpdateTransitionAction;
use bincode::{config, Decode, Encode};
use derive_more::From;
use platform_serialization::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_versioning::{PlatformSerdeVersionedDeserialize, PlatformVersioned};
use serde::Serialize;

#[derive(
    Debug,
    Clone,
    PlatformDeserialize,
    PlatformSerialize,
    Serialize,
    PlatformSerdeVersionedDeserialize,
    PlatformSignable,
    PlatformVersioned,
    Encode,
    Decode,
    From,
    PartialEq,
)]
#[platform_error_type(ProtocolError)]
#[platform_serialize(platform_version_path = state_transitions.identity_state_transition)]
#[serde(untagged)]
pub enum IdentityUpdateTransition {
    #[versioned(0)]
    V0(IdentityUpdateTransitionV0),
}

impl StateTransitionFieldTypes for IdentityUpdateTransition {
    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, ADD_PUBLIC_KEYS_SIGNATURE]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![IDENTITY_ID]
    }

    fn signature_property_paths() -> Vec<&'static str> {
        vec![
            SIGNATURE,
            SIGNATURE_PUBLIC_KEY_ID,
            ADD_PUBLIC_KEYS_SIGNATURE,
        ]
    }
}
