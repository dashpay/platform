pub mod data_contract;
pub mod identity;

use identity::IdentitySchemaJsons;

#[derive(Debug)]
pub struct SchemaJsons {
    pub(crate) identity: IdentitySchemaJsons,
}

impl SchemaJsons {
    pub fn new() -> Result<Self, serde_json::Error> {
        Ok(Self {
            identity: IdentitySchemaJsons::new()?,
        })
    }
}