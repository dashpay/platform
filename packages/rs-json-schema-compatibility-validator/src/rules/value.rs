use crate::error::{Error, JsonPointerPathNotFoundError, UnexpectedJsonValueTypeError};
use serde_json::Value;

/// Trait that provides methods for trying to convert a `Value` into different types.
pub trait ValueTryMethods {
    /// Tries to get a reference to a `Value` at a given JSON Pointer path.
    ///
    /// # Arguments
    ///
    /// * `path` - A JSON Pointer path as a string.
    ///
    /// # Returns
    ///
    /// * `Result<&Self, Error>` - A result that contains a reference to the `Value` at the given path, or an error if the path does not exist.
    fn try_pointer(&self, path: &str) -> Result<&Self, Error>;

    /// Tries to convert the `Value` into a `f64`.
    ///
    /// # Returns
    ///
    /// * `Result<f64, Error>` - A result that contains the `Value` as a `f64`, or an error if the `Value` is not a `f64`.
    fn try_to_f64(&self) -> Result<f64, Error>;

    /// Tries to convert the `Value` into a `u64`.
    ///
    /// # Returns
    ///
    /// * `Result<u64, Error>` - A result that contains the `Value` as a `u64`, or an error if the `Value` is not a `u64`.
    fn try_to_u64(&self) -> Result<u64, Error>;

    /// Tries to convert the `Value` into a `bool`.
    ///
    /// # Returns
    ///
    /// * `Result<bool, Error>` - A result that contains the `Value` as a `bool`, or an error if the `Value` is not a `bool`.
    fn try_to_bool(&self) -> Result<bool, Error>;
}

impl ValueTryMethods for Value {
    fn try_pointer(&self, path: &str) -> Result<&Self, Error> {
        self.pointer(path).ok_or_else(|| {
            Error::JsonPointerPathNotFound(JsonPointerPathNotFoundError {
                path: path.to_string(),
                value: self.clone(),
            })
        })
    }

    fn try_to_f64(&self) -> Result<f64, Error> {
        self.as_f64().ok_or_else(|| {
            Error::UnexpectedJsonValueType(UnexpectedJsonValueTypeError {
                expected_type: "f64".to_string(),
                value: self.clone(),
            })
        })
    }

    fn try_to_u64(&self) -> Result<u64, Error> {
        self.as_u64().ok_or_else(|| {
            Error::UnexpectedJsonValueType(UnexpectedJsonValueTypeError {
                expected_type: "u64".to_string(),
                value: self.clone(),
            })
        })
    }

    fn try_to_bool(&self) -> Result<bool, Error> {
        self.as_bool().ok_or_else(|| {
            Error::UnexpectedJsonValueType(UnexpectedJsonValueTypeError {
                expected_type: "bool".to_string(),
                value: self.clone(),
            })
        })
    }
}
