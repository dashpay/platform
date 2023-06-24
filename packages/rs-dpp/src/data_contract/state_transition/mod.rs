pub mod errors;

pub(crate) mod property_names {
    pub const STATE_TRANSITION_PROTOCOL_VERSION: &str = "version";
    pub const SIGNATURE_PUBLIC_KEY_ID: &str = "signaturePublicKeyId";
    pub const DATA_CONTRACT: &str = "dataContract";
    pub const SIGNATURE: &str = "signature";
    pub const ENTROPY: &str = "entropy";
    pub const TRANSITION_TYPE: &str = "type";
}
