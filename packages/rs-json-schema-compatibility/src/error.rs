use json_patch::PatchOperation;
use serde_json::Value;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    UnexpectedJsonPatchOperation(UnexpectedPatchOperationError),
    #[error(transparent)]
    JsonPointerPathNotFound(JsonPointerPathNotFoundError),
    #[error(transparent)]
    UnsupportedSchemaKeyword(UnsupportedSchemaKeywordError),
    #[error(transparent)]
    InvalidJsonPatchOperationPath(InvalidJsonPatchOperationPathError),
    #[error(transparent)]
    UndefinedReplaceCallback(UndefinedReplaceCallbackError),
    #[error(transparent)]
    UnexpectedJsonValueType(UnexpectedJsonValueTypeError),
}

#[derive(thiserror::Error, Debug)]
#[error("unexpected patch operation: {0}")]
pub struct UnexpectedPatchOperationError(pub PatchOperation);

impl From<UnexpectedPatchOperationError> for Error {
    fn from(e: UnexpectedPatchOperationError) -> Self {
        Error::UnexpectedJsonPatchOperation(e)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("JSON Pointer path '{path}' doesn't exist in JSON value")]
pub struct JsonPointerPathNotFoundError {
    pub path: String,
    pub value: Value,
}

impl From<JsonPointerPathNotFoundError> for Error {
    fn from(e: JsonPointerPathNotFoundError) -> Self {
        Error::JsonPointerPathNotFound(e)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("schema keyword '{keyword}' at path '{path}' is not supported")]
pub struct UnsupportedSchemaKeywordError {
    pub keyword: String,
    pub path: String,
}

impl From<UnsupportedSchemaKeywordError> for Error {
    fn from(e: UnsupportedSchemaKeywordError) -> Self {
        Error::UnsupportedSchemaKeyword(e)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("json patch operation path '{path}' is too small and doesn't contain keywords")]
pub struct InvalidJsonPatchOperationPathError {
    pub path: String,
}

impl From<InvalidJsonPatchOperationPathError> for Error {
    fn from(e: InvalidJsonPatchOperationPathError) -> Self {
        Error::InvalidJsonPatchOperationPath(e)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("undefined allow replace callback for keyword '{keyword}'")]
pub struct UndefinedReplaceCallbackError {
    pub keyword: String,
}

impl From<UndefinedReplaceCallbackError> for Error {
    fn from(e: UndefinedReplaceCallbackError) -> Self {
        Error::UndefinedReplaceCallback(e)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("expected JSON value type '{expected_type}', but got '{value}'")]
pub struct UnexpectedJsonValueTypeError {
    pub expected_type: String,
    pub value: Value,
}

impl From<UnexpectedJsonValueTypeError> for Error {
    fn from(e: UnexpectedJsonValueTypeError) -> Self {
        Error::UnexpectedJsonValueType(e)
    }
}
