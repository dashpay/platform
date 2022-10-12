pub use data_contract_create_transition::*;
pub use data_contract_update_transition::*;

pub mod data_contract_create_transition;
pub mod data_contract_update_transition;

pub(crate) mod property_names {
    pub const SIGNATURE_PUBLIC_KEY_ID: &str = "signaturePublicKeyId";
    pub const DATA_CONTRACT: &str = "dataContract";
    pub const SIGNATURE: &str = "signature";
    pub const ENTROPY: &str = "entropy";
    pub const PROTOCOL_VERSION: &str = "protocolVersion";
    pub const TRANSITION_TYPE: &str = "type";
}
