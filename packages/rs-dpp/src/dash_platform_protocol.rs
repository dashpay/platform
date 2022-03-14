use crate::identity::IdentityFacade;
use crate::schema::identity::identity_json;
use jsonschema::{JSONSchema, ValidationError};
use serde_json::{Error, Value};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DashPlatformProtocolInitError {
    #[error("Schema is invalid")]
    SchemaValidationError(SchemaCompilationError),
    #[error("Couldn't parse JSON")]
    SchemaDeserializationError(serde_json::Error),
}

// impl<'a> From<ValidationError<'a>> for DashPlatformProtocolInitError<'a> {
//     fn from(validation_error: ValidationError<'a>) -> Self {
//         Self::SchemaValidationError(validation_error)
//     }
// }

impl<'a> From<serde_json::Error> for DashPlatformProtocolInitError {
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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct IdentitySchemas {
    identity: JSONSchema,
    public_key: JSONSchema,
}

impl<'a> IdentitySchemas {
    pub fn new(identity_json: &'a Value, public_key_json: &'a Value) -> Result<Self, ValidationError<'a>>  {
        Ok(Self {
            identity: JSONSchema::compile(identity_json)?,
            public_key: JSONSchema::compile(public_key_json)?,
        })
    }
}

#[derive(Debug)]
pub struct SchemaJsons {
    identity: IdentitySchemaJsons,
}

impl SchemaJsons {
    pub fn new() -> Result<Self, serde_json::Error> {
        Ok(Self {
            identity: IdentitySchemaJsons::new()?,
        })
    }
}

#[derive(Debug)]
pub struct SchemaCompilationError {
    json_schemas: JsonSchemas,
}

impl SchemaCompilationError {
    pub fn new(json_schemas: JsonSchemas) -> Self {
        Self { json_schemas }
    }

    pub fn error(&self) -> Option<ValidationError> {
        self.json_schemas.compilation_error()
    }
}

#[derive(Debug)]
pub struct JsonSchemas {
    schema_jsons: SchemaJsons,
    identity: Option<IdentitySchemas>,
    // compilation_error: Option<ValidationError<'a>>
}

impl JsonSchemas {
    pub fn new(schema_jsons: SchemaJsons) -> Self {
        Self { schema_jsons, identity: None }
    }

    pub fn compile(&mut self) -> Result<(), ValidationError> {
        // It is safe to unwrap here, as it's impossible to cons
        match IdentitySchemas::new(
            &self.schema_jsons.identity.identity_json,
            &self.schema_jsons.identity.public_key_json
        ) {
            Ok(identity_schemas) => {
                self.identity = Some(identity_schemas);
                Ok(())
            }
            Err(err) => {
                Err(err)
            }
        }
    }

    pub fn compilation_error(&self) -> Option<ValidationError> {
        match IdentitySchemas::new(
            &self.schema_jsons.identity.identity_json,
            &self.schema_jsons.identity.public_key_json
        ) {
            Ok(_) => { None }
            Err(err) => { Some(err) }
        }
    }
}

pub struct JsonSchemasBuilder {
    schema_jsons: Option<SchemaJsons>
}

// impl JsonSchemasBuilder {
//     pub fn new() -> Self {
//         Self {
//             schema_jsons: None
//         }
//     }
//
//     pub fn schema_jsons(mut self, schema_jsons: SchemaJsons) -> Self {
//         self.schema_jsons = Some(schema_jsons);
//         // Unwrap is safe here, as we're guaranteed to receive them in the method signature
//         self.json_schemas = JsonSchemas::new(&self.schema_jsons.unwrap());
//         self
//     }
//
//     pub fn build(mut self) -> JsonSchemas {
//         JsonSchemas {
//             identity: IdentitySchemas::new(&self.)
//         }
//     }
// }

pub struct DashPlatformProtocol {
    json_schemas: JsonSchemas,
    identities: IdentityFacade,
}

impl DashPlatformProtocol {
    pub fn new<'a>() -> Result<Self, DashPlatformProtocolInitError> {
        let schema_jsons = SchemaJsons::new()?;
        let mut json_schemas = JsonSchemas::new(schema_jsons);
        let res = json_schemas.compile();

        match res {
            Ok(_) => {
                Ok(Self {
                    identities: IdentityFacade::new(),
                    json_schemas,
                })
            }
            Err(_) => {
                Err(DashPlatformProtocolInitError::SchemaValidationError(SchemaCompilationError::new(json_schemas)))
            }
        }
    }

    pub fn identities(&self) -> &IdentityFacade {
        &self.identities
    }
}
