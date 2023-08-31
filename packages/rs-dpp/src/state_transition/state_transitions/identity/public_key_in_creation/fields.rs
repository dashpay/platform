use crate::state_transition::state_transitions;

pub use state_transitions::common_fields::property_names::SIGNATURE;

pub const DATA: &str = "data";

pub const BINARY_DATA_FIELDS: [&str; 2] = [DATA, SIGNATURE];
