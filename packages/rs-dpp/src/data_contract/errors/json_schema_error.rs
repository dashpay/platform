use thiserror::Error;

// @append_only
#[derive(Error, Debug)]
pub enum JSONSchemaError {
    #[error("can't create json schema: {0}")]
    CreateSchemaError(&'static str),
}
