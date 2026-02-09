use crate::state_transition::state_transitions;

pub use state_transitions::common_fields::property_names::{
    STATE_TRANSITION_PROTOCOL_VERSION, TRANSITION_TYPE,
};

pub const OUTPUT_SCRIPT: &str = "outputScript";

pub const IDENTIFIER_FIELDS: [&str; 0] = [];
pub const BINARY_FIELDS: [&str; 1] = [OUTPUT_SCRIPT];
pub const U32_FIELDS: [&str; 1] = [STATE_TRANSITION_PROTOCOL_VERSION];
