pub mod asset_lock_proof;
pub mod identity_create_transition;
pub mod identity_credit_withdrawal_transition;
pub mod identity_topup_transition;
pub mod identity_update_transition;
pub mod validate_public_key_signatures;

pub(crate) mod properties {
    pub const PROPERTY_SIGNATURE_PUBLIC_KEY_ID: &str = "signaturePublicKeyId";
    pub const PROPERTY_SIGNATURE: &str = "signature";
    pub const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";
    pub const PROPERTY_TRANSITION_TYPE: &str = "type";
    pub const PROPERTY_OUTPUT_SCRIPT: &str = "outputScript";
    pub const PROPERTY_IDENTITY_ID: &str = "identityId";
    pub const PROPERTY_OWNER_ID: &str = "ownerId";
}
