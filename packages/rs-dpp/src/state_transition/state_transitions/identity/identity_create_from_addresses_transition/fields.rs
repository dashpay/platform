use crate::state_transition::state_transitions;

pub use state_transitions::common_fields::property_names::{
    SIGNATURE, STATE_TRANSITION_PROTOCOL_VERSION,
};
#[allow(unused_imports)] // Removing causes build failures; yet clippy insists it's unused
pub use state_transitions::identity::common_fields::property_names::{
    ASSET_LOCK_PROOF, PUBLIC_KEYS,
};
pub use state_transitions::identity::common_fields::property_names::{
    IDENTITY_ID, PUBLIC_KEYS_DATA, PUBLIC_KEYS_SIGNATURE,
};

pub const IDENTIFIER_FIELDS: [&str; 1] = [IDENTITY_ID];
pub const BINARY_FIELDS: [&str; 3] = [PUBLIC_KEYS_DATA, PUBLIC_KEYS_SIGNATURE, SIGNATURE];
pub const U32_FIELDS: [&str; 1] = [STATE_TRANSITION_PROTOCOL_VERSION];
