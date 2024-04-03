/// The property names of a document
pub mod property_names {
    pub const FEATURE_VERSION: &str = "$version";
    pub const ID: &str = "$id";
    pub const DATA_CONTRACT_ID: &str = "$dataContractId";
    pub const REVISION: &str = "$revision";
    pub const OWNER_ID: &str = "$ownerId";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
    pub const CREATED_AT_BLOCK_HEIGHT: &str = "$createdAtBlockHeight";
    pub const UPDATED_AT_BLOCK_HEIGHT: &str = "$updatedAtBlockHeight";
    pub const CREATED_AT_CORE_BLOCK_HEIGHT: &str = "$createdAtCoreBlockHeight";
    pub const UPDATED_AT_CORE_BLOCK_HEIGHT: &str = "$updatedAtCoreBlockHeight";
}

pub const IDENTIFIER_FIELDS: [&str; 3] = [
    property_names::ID,
    property_names::OWNER_ID,
    property_names::DATA_CONTRACT_ID,
];
