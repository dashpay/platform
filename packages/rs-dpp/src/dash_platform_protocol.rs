use crate::identity::IdentityFacade;
use crate::schema::identity::identity_json;
use jsonschema::{JSONSchema, ValidationError};
use serde_json::{Error, Value};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DashPlatformProtocolInitError<'a> {
    #[error("Schema is invalid")]
    SchemaValidationError(ValidationError<'a>),
    #[error("Couldn't parse JSON")]
    SchemaDeserializationError(serde_json::Error),
}

impl<'a> From<ValidationError<'a>> for DashPlatformProtocolInitError<'a> {
    fn from(validation_error: ValidationError<'a>) -> Self {
        Self::SchemaValidationError(validation_error)
    }
}

impl<'a> From<serde_json::Error> for DashPlatformProtocolInitError<'a> {
    fn from(error: serde_json::Error) -> Self {
        Self::SchemaDeserializationError(error)
    }
}

struct AssetLockProofStateTransitionsSchema {
    chain_asset_lock_proof_json: Value,
    instant_asset_lock_proof_json: Value,
}

struct IdentityStateTransitionSchemas {
    asset_lock_proof: AssetLockProofStateTransitionsSchema,
    identity_create_json: Value,
    identity_top_up_json: Value,
}

struct IdentitySchemaJsons {
    identity_json: Value,
    public_key_json: Value,
    // identity: JSONSchema,
    // public_key: JSONSchema,
    //state_transition: IdentityStateTransitionSchemas,
}

impl IdentitySchemaJsons {
    pub fn new() -> Result<Self, serde_json::Error> {
        Ok(Self {
            identity_json: identity_json()?,
            public_key_json: Default::default(),
        })
    }
}

pub struct IdentitySchemas {
    identity: JSONSchema,
    public_key: JSONSchema,
}

impl IdentitySchemas {
    pub fn new<'a>(identity_json: &'a Value, public_key_json: &'a Value) -> Result<Self, ValidationError<'a>>  {
        Ok(Self {
            identity: JSONSchema::compile(identity_json)?,
            public_key: JSONSchema::compile(public_key_json)?,
        })
    }
}

struct SchemaJsons {
    identity: IdentitySchemaJsons,
}

impl SchemaJsons {
    pub fn new() -> Result<Self, serde_json::Error> {
        Ok(Self {
            identity: IdentitySchemaJsons::new()?,
        })
    }
}

struct JsonSchemas {
    identity: IdentitySchemas,
}

impl JsonSchemas {
    pub fn new(schema_jsons: &SchemaJsons) -> Result<Self, ValidationError> {
        Ok(Self {
            identity: IdentitySchemas::new(
                &schema_jsons.identity.identity_json,
                &schema_jsons.identity.public_key_json,
            )?,
        })
    }
}

pub struct DashPlatformProtocol {
    schema_jsons: SchemaJsons,
    json_schemas: JsonSchemas,
    identities: IdentityFacade,
}

impl DashPlatformProtocol {
    pub fn new<'a>() -> Result<Self, DashPlatformProtocolInitError<'a>> {
        let schema_jsons = SchemaJsons::new()?;
        let json_schemas = JsonSchemas::new(&schema_jsons)?;
        Ok(Self {
            identities: IdentityFacade::new(),
            schema_jsons,
            json_schemas,
        })
    }

    pub fn identities(&self) -> &IdentityFacade {
        &self.identities
    }
}
