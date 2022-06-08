mod data_contract_update_transition;
pub use data_contract_update_transition::*;

mod data_contract_create_transition;
pub use data_contract_create_transition::*;

pub(crate) mod properties {
    pub const PROPERTY_SIGNATURE_PUBLIC_KEY_ID: &str = "signaturePublicKeyId";
    pub const PROPERTY_DATA_CONTRACT: &str = "dataContract";
    pub const PROPERTY_SIGNATURE: &str = "signature";
    pub const PROPERTY_ENTROPY: &str = "entropy";
    pub const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";
    pub const PROPERTY_TRANSITION_TYPE: &str = "type";
}
