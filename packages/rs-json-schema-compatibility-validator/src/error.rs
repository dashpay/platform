use crate::CompatibilityRules;
use json_patch::PatchOperation;
use serde_json::Value;

/// Compatibility validation errors.
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    /// An unexpected patch operation was encountered.
    #[error(transparent)]
    UnexpectedJsonPatchOperation(UnexpectedPatchOperationError),
    /// The JSON Pointer path doesn't exist in the JSON value.
    #[error(transparent)]
    JsonPointerPathNotFound(JsonPointerPathNotFoundError),
    /// There is no compatibility rules are defined for the encountered schema keyword
    #[error(transparent)]
    UnsupportedSchemaKeyword(UnsupportedSchemaKeywordError),
    /// The JSON patch operation path is too small and doesn't contain keywords.
    #[error(transparent)]
    InvalidJsonPatchOperationPath(InvalidJsonPatchOperationPathError),
    /// The [IsReplacementAllowedCallback] is not defined for the encountered keyword and [ReplaceOperation].
    #[error(transparent)]
    UndefinedReplaceCallback(UndefinedReplacementAllowedCallbackError),
    /// The JSON value type is not as expected.
    #[error(transparent)]
    UnexpectedJsonValueType(UnexpectedJsonValueTypeError),
}

/// An unexpected patch operation was encountered.
#[derive(thiserror::Error, Debug, Clone)]
#[error("unexpected patch operation: {0}")]
pub struct UnexpectedPatchOperationError(pub PatchOperation);

impl From<UnexpectedPatchOperationError> for Error {
    fn from(e: UnexpectedPatchOperationError) -> Self {
        Error::UnexpectedJsonPatchOperation(e)
    }
}

/// The JSON Pointer path doesn't exist in the JSON value.
#[derive(thiserror::Error, Debug, Clone)]
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

/// There is no compatibility rules are defined for the encountered schema keyword
#[derive(thiserror::Error, Debug, Clone)]
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

/// The JSON patch operation path is too small and doesn't contain keywords.
#[derive(thiserror::Error, Debug, Clone)]
#[error("json patch operation path '{path}' is too small and doesn't contain keywords")]
pub struct InvalidJsonPatchOperationPathError {
    pub path: String,
}

impl From<InvalidJsonPatchOperationPathError> for Error {
    fn from(e: InvalidJsonPatchOperationPathError) -> Self {
        Error::InvalidJsonPatchOperationPath(e)
    }
}

/// The [IsReplacementAllowedCallback] is not defined for the encountered keyword and [ReplaceOperation].
#[derive(thiserror::Error, Debug, Clone)]
#[error("undefined allow replacement callback for path '{path}'")]
pub struct UndefinedReplacementAllowedCallbackError {
    pub path: String,
    pub rules: CompatibilityRules,
}

impl From<UndefinedReplacementAllowedCallbackError> for Error {
    fn from(e: UndefinedReplacementAllowedCallbackError) -> Self {
        Error::UndefinedReplaceCallback(e)
    }
}

/// The JSON value type is not as expected.
#[derive(thiserror::Error, Debug, Clone)]
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
