mod identity;
mod public_key;

pub use identity::identity_json;
pub use public_key::public_key_json;

#[derive(Debug)]
pub struct IdentitySchemaJsons {
    pub(crate) identity_json: serde_json::Value,
    pub(crate) public_key_json: serde_json::Value,
    //state_transition: IdentityStateTransitionSchemas,
}

impl IdentitySchemaJsons {
    pub fn new() -> Result<Self, serde_json::Error> {
        Ok(Self {
            identity_json: identity::identity_json()?,
            public_key_json: public_key::public_key_json()?,
        })
    }
}