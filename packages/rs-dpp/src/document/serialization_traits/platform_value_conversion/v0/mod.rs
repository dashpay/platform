use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub trait DocumentPlatformValueMethodsV0<'a>: Serialize + Deserialize<'a> {
    fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError>;
    fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError>;
    fn into_value(self) -> Result<Value, ProtocolError>;
    fn to_object(&self) -> Result<Value, ProtocolError>;
    fn from_platform_value(
        document_value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized;
}
