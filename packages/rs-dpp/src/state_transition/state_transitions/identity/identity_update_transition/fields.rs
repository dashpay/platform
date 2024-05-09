use crate::state_transition::state_transitions;

use crate::state_transition::identity_update_transition::fields::property_names::{
    ADD_PUBLIC_KEYS_DATA, ADD_PUBLIC_KEYS_SIGNATURE,
};
pub use state_transitions::common_fields::property_names::{
    IDENTITY_NONCE, REVISION, SIGNATURE, SIGNATURE_PUBLIC_KEY_ID,
    STATE_TRANSITION_PROTOCOL_VERSION, TRANSITION_TYPE,
};
pub use state_transitions::identity::common_fields::property_names::IDENTITY_ID;

pub mod property_names {
    pub const ADD_PUBLIC_KEYS_DATA: &str = "addPublicKeys[].data";
    pub const ADD_PUBLIC_KEYS_SIGNATURE: &str = "addPublicKeys[].signature";
    pub const ADD_PUBLIC_KEYS: &str = "addPublicKeys";
    pub const DISABLE_PUBLIC_KEYS: &str = "disablePublicKeys";
}

pub const IDENTIFIER_FIELDS: [&str; 1] = [IDENTITY_ID];
pub const BINARY_FIELDS: [&str; 3] = [ADD_PUBLIC_KEYS_DATA, ADD_PUBLIC_KEYS_SIGNATURE, SIGNATURE];
pub const U32_FIELDS: [&str; 1] = [STATE_TRANSITION_PROTOCOL_VERSION];
