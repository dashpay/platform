use crate::document::{v0, DocumentV0};
use crate::ProtocolError;
use platform_value::Value;
use std::collections::BTreeMap;

pub trait DocumentPlatformValueMethodsV0 {
    fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError>;
    fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError>;
    fn into_value(self) -> Result<Value, ProtocolError>;
    fn to_object(&self) -> Result<Value, ProtocolError>;
    fn from_platform_value(document_value: Value) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
