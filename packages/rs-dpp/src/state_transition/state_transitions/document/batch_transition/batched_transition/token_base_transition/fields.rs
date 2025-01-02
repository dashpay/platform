pub(in crate::state_transition::state_transitions::document::batch_transition) mod property_names {
    pub const DATA_CONTRACT_ID: &str = "$dataContractId";
    pub const TOKEN_CONTRACT_POSITION: &str = "$tokenContractPosition";
    pub const TOKEN_ID: &str = "$tokenId";
    pub const GROUP_CONTRACT_POSITION: &str = "$groupContractPosition";
    pub const GROUP_ACTION_ID: &str = "$groupActionId";
    pub const GROUP_ACTION_IS_PROPOSER: &str = "$groupActionIsProposer";
    pub const ACTION: &str = "$action";
    pub const IDENTITY_CONTRACT_NONCE: &str = "$identityContractNonce";
}

pub const IDENTIFIER_FIELDS: [&str; 1] = [property_names::DATA_CONTRACT_ID];
