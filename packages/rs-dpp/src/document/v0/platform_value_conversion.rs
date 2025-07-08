use crate::document::serialization_traits::DocumentPlatformValueMethodsV0;
use crate::document::DocumentV0;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use std::collections::BTreeMap;

impl DocumentPlatformValueMethodsV0<'_> for DocumentV0 {
    fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        Ok(platform_value::to_value(self)?.into_btree_string_map()?)
    }

    fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        Ok(platform_value::to_value(self)?.into_btree_string_map()?)
    }

    fn into_value(self) -> Result<Value, ProtocolError> {
        Ok(platform_value::to_value(self)?)
    }

    fn to_object(&self) -> Result<Value, ProtocolError> {
        Ok(platform_value::to_value(self)?)
    }

    fn from_platform_value(
        document_value: Value,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(platform_value::from_value(document_value)?)
    }
}
