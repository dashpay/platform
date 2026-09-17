use crate::state_transition::state_transitions;

pub use state_transitions::common_fields::property_names::{
    IDENTITY_NONCE, REVISION, SIGNATURE, SIGNATURE_PUBLIC_KEY_ID,
    STATE_TRANSITION_PROTOCOL_VERSION, TRANSITION_TYPE,
};
pub use state_transitions::identity::common_fields::property_names::IDENTITY_ID;

pub mod property_names {
    pub const KEY_ID: &str = "keyId";
    pub const TOTAL_BUDGET: &str = "totalBudget";
    pub const EXPIRES_AT: &str = "expiresAt";
}

pub const IDENTIFIER_FIELDS: [&str; 1] = [IDENTITY_ID];
pub const BINARY_FIELDS: [&str; 1] = [SIGNATURE];
pub const U32_FIELDS: [&str; 1] = [STATE_TRANSITION_PROTOCOL_VERSION];
