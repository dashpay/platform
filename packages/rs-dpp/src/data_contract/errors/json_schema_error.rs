use thiserror::Error;

// @append_only
#[derive(Error, Debug)]
pub enum JsonSchemaError {
    #[error("can't create json schema: {0}")]
    CreateSchemaError(&'static str),
}
