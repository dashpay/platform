use std::collections::BTreeMap;
use platform_value::Value;
use crate::document::{DocumentV0, v0};
use crate::ProtocolError;

pub trait DocumentPlatformValueMethodsV0 {
    fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError>;
    fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError>;
    fn into_value(self) -> Result<Value, ProtocolError>;
    fn to_object(&self) -> Result<Value, ProtocolError>;
    fn from_platform_value(document_value: Value) -> Result<Self, ProtocolError> where Self: Sized;
}