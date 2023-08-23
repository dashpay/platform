use crate::state_transition::state_transitions;

pub use state_transitions::common_fields::property_names::{
    ENTROPY, SIGNATURE, SIGNATURE_PUBLIC_KEY_ID, STATE_TRANSITION_PROTOCOL_VERSION, TRANSITION_TYPE,
};
pub use state_transitions::contract::common_fields::property_names::{
    DATA_CONTRACT, DATA_CONTRACT_ENTROPY, DATA_CONTRACT_ID, DATA_CONTRACT_OWNER_ID,
    DATA_CONTRACT_PROTOCOL_VERSION,
};

pub const IDENTIFIER_FIELDS: [&str; 2] = [DATA_CONTRACT_ID, DATA_CONTRACT_OWNER_ID];

pub const BINARY_FIELDS: [&str; 3] = [ENTROPY, DATA_CONTRACT_ENTROPY, SIGNATURE];
pub const U32_FIELDS: [&str; 2] = [
    STATE_TRANSITION_PROTOCOL_VERSION,
    DATA_CONTRACT_PROTOCOL_VERSION,
];
