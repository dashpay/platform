pub(in crate::state_transition::state_transitions::document::documents_batch_transition) mod property_names {
    pub const ID: &str = "$id";
    pub const DATA_CONTRACT_ID: &str = "$dataContractId";
    pub const TOKEN_ID: &str = "$tokenId";
    pub const ACTION: &str = "$action";
    pub const IDENTITY_CONTRACT_NONCE: &str = "$identityContractNonce";
}

pub const IDENTIFIER_FIELDS: [&str; 2] = [property_names::ID, property_names::DATA_CONTRACT_ID];