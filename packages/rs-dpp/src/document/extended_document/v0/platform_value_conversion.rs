use crate::document::extended_document::v0::ExtendedDocumentV0;
use crate::document::serialization_traits::DocumentPlatformValueMethodsV0;
use crate::document::{property_names, Document, DocumentV0};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::Value;
use std::collections::BTreeMap;

impl DocumentPlatformValueMethodsV0<'_> for ExtendedDocumentV0 {
    fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        map.insert(property_names::ID.to_string(), self.id().into());
        map.insert(property_names::OWNER_ID.to_string(), self.owner_id().into());

        if let Some(created_at) = self.created_at() {
            map.insert(
                property_names::CREATED_AT.to_string(),
                Value::U64(*created_at),
            );
        }
        if let Some(updated_at) = self.updated_at() {
            map.insert(
                property_names::UPDATED_AT.to_string(),
                Value::U64(*updated_at),
            );
        }
        if let Some(revision) = self.revision() {
            map.insert(property_names::REVISION.to_string(), Value::U64(*revision));
        }

        map.extend(self.properties().clone());

        Ok(map)
    }

    fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        map.insert(property_names::ID.to_string(), self.id().into());
        map.insert(property_names::OWNER_ID.to_string(), self.owner_id().into());

        if let Some(created_at) = self.created_at() {
            map.insert(
                property_names::CREATED_AT.to_string(),
                Value::U64(*created_at),
            );
        }
        if let Some(updated_at) = self.updated_at() {
            map.insert(
                property_names::UPDATED_AT.to_string(),
                Value::U64(*updated_at),
            );
        }
        if let Some(revision) = self.revision() {
            map.insert(property_names::REVISION.to_string(), Value::U64(*revision));
        }

        map.extend(self.properties().to_owned());

        Ok(map)
    }

    fn into_value(self) -> Result<Value, ProtocolError> {
        Ok(self.into_map_value()?.into())
    }

    fn to_object(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_map_value()?.into())
    }

    fn from_platform_value(
        document_value: Value,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(platform_value::from_value(document_value)?)
    }
}
