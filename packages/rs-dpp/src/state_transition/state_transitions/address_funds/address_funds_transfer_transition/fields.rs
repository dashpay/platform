use crate::state_transition::state_transitions;

pub use state_transitions::common_fields::property_names::{
    IDENTITY_NONCE, SIGNATURE, SIGNATURE_PUBLIC_KEY_ID, STATE_TRANSITION_PROTOCOL_VERSION,
    TRANSITION_TYPE,
};

pub const IDENTIFIER_FIELDS: [&str; 0] = [];
pub const BINARY_FIELDS: [&str; 0] = [];
pub const U32_FIELDS: [&str; 1] = [STATE_TRANSITION_PROTOCOL_VERSION];
