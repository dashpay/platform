use crate::state_transition::state_transitions;

pub use state_transitions::common_fields::property_names::{
    SIGNATURE, STATE_TRANSITION_PROTOCOL_VERSION,
};
pub use state_transitions::identity::common_fields::property_names::{
    ASSET_LOCK_PROOF, IDENTITY_ID, PUBLIC_KEYS, PUBLIC_KEYS_DATA, PUBLIC_KEYS_SIGNATURE,
};

pub const IDENTIFIER_FIELDS: [&str; 1] = [IDENTITY_ID];
pub const BINARY_FIELDS: [&str; 3] = [PUBLIC_KEYS_DATA, PUBLIC_KEYS_SIGNATURE, SIGNATURE];
pub const U32_FIELDS: [&str; 1] = [STATE_TRANSITION_PROTOCOL_VERSION];
