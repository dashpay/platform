use crate::state_transition::state_transitions;

pub use state_transitions::common_fields::property_names::{
    ENTROPY, SIGNATURE, SIGNATURE_PUBLIC_KEY_ID, STATE_TRANSITION_PROTOCOL_VERSION, TRANSITION_TYPE,
};
pub use state_transitions::identity::common_fields::property_names::{
    ASSET_LOCK_PROOF, IDENTITY_ID, PUBLIC_KEYS, PUBLIC_KEYS_DATA, PUBLIC_KEYS_SIGNATURE,
};

pub const IDENTIFIER_FIELDS: [&str; 1] = [IDENTITY_ID];
pub const BINARY_FIELDS: [&str; 1] = [SIGNATURE];
pub const U32_FIELDS: [&str; 1] = [STATE_TRANSITION_PROTOCOL_VERSION];
