use crate::state_transition::state_transitions;

use crate::identity::SecurityLevel;
use crate::state_transition::batch_transition::fields::property_names::{
    OWNER_ID, TRANSITIONS_DATA_CONTRACT_ID, TRANSITIONS_ID,
};
pub use state_transitions::common_fields::property_names::{
    IDENTITY_NONCE, SIGNATURE, SIGNATURE_PUBLIC_KEY_ID, STATE_TRANSITION_PROTOCOL_VERSION,
    TRANSITION_TYPE,
};

pub mod property_names {
    pub const STATE_TRANSITION_PROTOCOL_VERSION: &str = "$version";
    pub const DATA_CONTRACT_ID: &str = "$dataContractId";
    pub const DOCUMENT_TYPE: &str = "$type";
    pub const TRANSITIONS: &str = "transitions";
    pub const TRANSITIONS_ID: &str = "transitions[].$id";
    pub const TRANSITIONS_DATA_CONTRACT_ID: &str = "transitions[].$dataContractId";
    pub const OWNER_ID: &str = "ownerId";
    pub const SECURITY_LEVEL_REQUIREMENT: &str = "signatureSecurityLevelRequirement";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
}

pub const IDENTIFIER_FIELDS: [&str; 3] = [OWNER_ID, TRANSITIONS_ID, TRANSITIONS_DATA_CONTRACT_ID];
pub const U16_FIELDS: [&str; 1] = [property_names::STATE_TRANSITION_PROTOCOL_VERSION];

pub const DEFAULT_SECURITY_LEVEL: SecurityLevel = SecurityLevel::HIGH;
