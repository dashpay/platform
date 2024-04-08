use json_patch::PatchOperation;
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    UnexpectedPatchOperation(UnexpectedPatchOperationError),
    #[error(transparent)]
    InvalidJsonPointerPath(InvalidJsonPointerPathError),
    #[error(transparent)]
    JsonPointerPathNotFound(JsonPointerPathNotFoundError),
}

#[derive(thiserror::Error, Debug)]
#[error("Unexpected patch operation: {0}")]
pub struct UnexpectedPatchOperationError(pub PatchOperation);

impl From<UnexpectedPatchOperationError> for Error {
    fn from(e: UnexpectedPatchOperationError) -> Self {
        Error::UnexpectedPatchOperation(e)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Invalid JSON pointer path '{path}': {error}")]
pub struct InvalidJsonPointerPathError {
    pub path: String,
    pub error: jsonptr::MalformedPointerError,
}

impl From<InvalidJsonPointerPathError> for Error {
    fn from(e: InvalidJsonPointerPathError) -> Self {
        Error::InvalidJsonPointerPath(e)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("JSON Pointer path '{path}' doesn't exist in schema: {error}")]
pub struct JsonPointerPathNotFoundError {
    pub path: String,
    pub error: jsonptr::Error,
}

impl From<JsonPointerPathNotFoundError> for Error {
    fn from(e: JsonPointerPathNotFoundError) -> Self {
        Error::JsonPointerPathNotFound(e)
    }
}
