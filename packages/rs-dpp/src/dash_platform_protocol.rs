use std::borrow::Cow;
use crate::identity::IdentityFacade;
use crate::schema::identity::IdentitySchemaJsons;
use jsonschema::{JSONSchema, ValidationError};
use serde_json;
use thiserror::Error;
use crate::schema::SchemaJsons;

#[derive(Error, Debug)]
pub enum DashPlatformProtocolInitError {
    #[error("Schema is invalid")]
    SchemaValidationError(SchemaCompilationError),
    #[error("Couldn't parse JSON")]
    SchemaDeserializationError(serde_json::Error),
    #[error("Please fill me")]
    ValidationError(ValidationError<'static>)
}

fn into_owned(err: ValidationError) -> ValidationError<'static> {
    ValidationError {
        instance_path: err.instance_path.clone(),
        instance: Cow::Owned(err.instance.into_owned()),
        kind: err.kind,
        schema_path: err.schema_path,
    }
}

impl From<serde_json::Error> for DashPlatformProtocolInitError {
    fn from(error: serde_json::Error) -> Self {
        Self::SchemaDeserializationError(error)
    }
}

impl<'a> From<ValidationError<'a>> for DashPlatformProtocolInitError {
    fn from(err: ValidationError<'a>) -> Self {
        Self::ValidationError(into_owned(err))
    }
}

#[derive(Debug)]
pub struct IdentitySchemas {
    public_key: JSONSchema,
}

impl<'a> IdentitySchemas {
    pub fn new(
        identity_json: &'a serde_json::Value,
        public_key_json: &'a serde_json::Value,
    ) -> Result<Self, ValidationError<'a>> {
        Ok(Self {
            public_key: JSONSchema::compile(public_key_json)?,
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
}

impl JsonSchemas {
    pub fn new(schema_jsons: SchemaJsons) -> Self {
        Self {
            schema_jsons,
            identity: None,
        }
    }

    pub fn compile(&mut self) -> Result<(), ValidationError> {
        match IdentitySchemas::new(
            &self.schema_jsons.identity.identity_json,
            &self.schema_jsons.identity.public_key_json,
        ) {
            Ok(identity_schemas) => {
                self.identity = Some(identity_schemas);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    pub fn compilation_error(&self) -> Option<ValidationError> {
        match IdentitySchemas::new(
            &self.schema_jsons.identity.identity_json,
            &self.schema_jsons.identity.public_key_json,
        ) {
            Ok(_) => None,
            Err(err) => Some(err),
        }
    }
}

pub struct DashPlatformProtocol {
    json_schemas: JsonSchemas,
    identities: IdentityFacade,
}

impl DashPlatformProtocol {
    pub fn new() -> Result<Self, DashPlatformProtocolInitError> {
        let schema_jsons = SchemaJsons::new()?;
        let mut json_schemas = JsonSchemas::new(schema_jsons);
        json_schemas.compile()?;

        Ok(Self {
            identities: IdentityFacade::new()?,
            json_schemas,
        })
    }

    pub fn identities(&self) -> &IdentityFacade {
        &self.identities
    }
}
