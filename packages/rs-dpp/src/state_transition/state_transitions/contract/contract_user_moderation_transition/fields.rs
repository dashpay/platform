use crate::state_transition::state_transitions;

pub use state_transitions::common_fields::property_names::{
    SIGNATURE, SIGNATURE_PUBLIC_KEY_ID, STATE_TRANSITION_PROTOCOL_VERSION, TRANSITION_TYPE,
};

pub mod property_names {
    pub const OWNER_ID: &str = "ownerId";
    pub const DATA_CONTRACT_ID: &str = "dataContractId";
    pub const IDENTITY_CONTRACT_NONCE: &str = "identityContractNonce";
    pub const ACTION: &str = "action";
}

pub use property_names::{DATA_CONTRACT_ID, OWNER_ID};

pub const IDENTIFIER_FIELDS: [&str; 2] = [OWNER_ID, DATA_CONTRACT_ID];
pub const BINARY_FIELDS: [&str; 1] = [SIGNATURE];
pub const U32_FIELDS: [&str; 1] = [STATE_TRANSITION_PROTOCOL_VERSION];
