use crate::state_transition::state_transitions;

use crate::state_transition::identity_credit_transfer_transition::fields::property_names::RECIPIENT_ID;
pub use state_transitions::common_fields::property_names::{
    ENTROPY, SIGNATURE, SIGNATURE_PUBLIC_KEY_ID, STATE_TRANSITION_PROTOCOL_VERSION, TRANSITION_TYPE,
};
pub use state_transitions::identity::common_fields::property_names::IDENTITY_ID;

pub(crate) mod property_names {
    pub const RECIPIENT_ID: &str = "recipientId";
}

pub const IDENTIFIER_FIELDS: [&str; 2] = [IDENTITY_ID, RECIPIENT_ID];
pub const BINARY_FIELDS: [&str; 1] = [SIGNATURE];
pub const U32_FIELDS: [&str; 1] = [STATE_TRANSITION_PROTOCOL_VERSION];
