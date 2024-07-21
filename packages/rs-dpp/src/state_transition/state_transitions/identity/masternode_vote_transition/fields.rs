use crate::state_transition::state_transitions;

use crate::state_transition::masternode_vote_transition::fields::property_names::PRO_TX_HASH;
pub use state_transitions::common_fields::property_names::{
    SIGNATURE, SIGNATURE_PUBLIC_KEY_ID, STATE_TRANSITION_PROTOCOL_VERSION, TRANSITION_TYPE,
};
pub use state_transitions::identity::common_fields::property_names::IDENTITY_ID;

pub(crate) mod property_names {
    pub const PRO_TX_HASH: &str = "proTxHash";
}

pub const IDENTIFIER_FIELDS: [&str; 1] = [PRO_TX_HASH];
pub const BINARY_FIELDS: [&str; 1] = [SIGNATURE];
pub const U32_FIELDS: [&str; 1] = [STATE_TRANSITION_PROTOCOL_VERSION];
