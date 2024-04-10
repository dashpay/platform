use crate::error::{Error, JsonPointerPathNotFoundError, UnexpectedJsonValueTypeError};
use serde_json::Value;

pub trait ValueTryMethods {
    fn try_pointer(&self, path: &str) -> Result<&Self, Error>;

    fn try_to_f64(&self) -> Result<f64, Error>;

    fn try_to_u64(&self) -> Result<u64, Error>;

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
