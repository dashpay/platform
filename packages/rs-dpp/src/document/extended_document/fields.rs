pub mod property_names {
    pub const FEATURE_VERSION: &str = "$version";
    pub const ID: &str = "$id";
    pub const DOCUMENT_TYPE_NAME: &str = "$type";
    pub const REVISION: &str = "$revision";
    pub const DATA_CONTRACT: &str = "$dataContract";
    pub const DATA_CONTRACT_ID: &str = "$dataContractId";
    pub const OWNER_ID: &str = "$ownerId";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
}

pub const IDENTIFIER_FIELDS: [&str; 3] = [
    property_names::ID,
    property_names::DATA_CONTRACT_ID,
    property_names::OWNER_ID,
];
