use thiserror::Error;

// @append_only
#[derive(Error, Debug)]
#[ferment_macro::export]
pub enum JsonSchemaError {
    #[error("can't create json schema: {0}")]
    CreateSchemaError(&'static str),
}
