use crate::state_transition::state_transitions;

pub use state_transitions::common_fields::property_names::STATE_TRANSITION_PROTOCOL_VERSION;
pub use state_transitions::identity::common_fields::property_names::PUBLIC_KEYS;
pub use state_transitions::identity::common_fields::property_names::{
    IDENTITY_ID, PUBLIC_KEYS_DATA, PUBLIC_KEYS_SIGNATURE,
};

pub const INPUTS: &str = "inputs";
pub const OUTPUTS: &str = "outputs";
pub const INPUT_SIGNATURES: &str = "inputSignatures";
pub const USER_FEE_INCREASE: &str = "userFeeIncrease";

pub const IDENTIFIER_FIELDS: [&str; 1] = [IDENTITY_ID];
pub const BINARY_FIELDS: [&str; 3] = [PUBLIC_KEYS_DATA, PUBLIC_KEYS_SIGNATURE, INPUT_SIGNATURES];
pub const U32_FIELDS: [&str; 1] = [STATE_TRANSITION_PROTOCOL_VERSION];
