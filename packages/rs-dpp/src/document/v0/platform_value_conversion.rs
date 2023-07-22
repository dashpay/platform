use crate::document::serialization_traits::DocumentPlatformValueMethodsV0;
use crate::document::{property_names, DocumentV0};
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::Value;
use std::collections::BTreeMap;
use crate::version::PlatformVersion;

impl DocumentPlatformValueMethodsV0 for DocumentV0 {
    fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        map.insert(property_names::ID.to_string(), self.id.into());
        map.insert(property_names::OWNER_ID.to_string(), self.owner_id.into());

        if let Some(created_at) = self.created_at {
            map.insert(
                property_names::CREATED_AT.to_string(),
                Value::U64(created_at),
            );
        }
        if let Some(updated_at) = self.updated_at {
            map.insert(
                property_names::UPDATED_AT.to_string(),
                Value::U64(updated_at),
            );
        }
        if let Some(revision) = self.revision {
            map.insert(property_names::REVISION.to_string(), Value::U64(revision));
        }

        map.extend(self.properties.clone());

        Ok(map)
    }

    fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut map: BTreeMap<String, Value> = BTreeMap::new();
        map.insert(property_names::ID.to_string(), self.id.into());
        map.insert(property_names::OWNER_ID.to_string(), self.owner_id.into());

        if let Some(created_at) = self.created_at {
            map.insert(
                property_names::CREATED_AT.to_string(),
                Value::U64(created_at),
            );
        }
        if let Some(updated_at) = self.updated_at {
            map.insert(
                property_names::UPDATED_AT.to_string(),
                Value::U64(updated_at),
            );
        }
        if let Some(revision) = self.revision {
            map.insert(property_names::REVISION.to_string(), Value::U64(revision));
        }

        map.extend(self.properties);

        Ok(map)
    }

    fn into_value(self) -> Result<Value, ProtocolError> {
        Ok(self.into_map_value()?.into())
    }

    fn to_object(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_map_value()?.into())
    }

    fn from_platform_value(document_value: Value, _platform_version: &PlatformVersion) -> Result<Self, ProtocolError> {
        let mut properties = document_value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;

        let mut document = DocumentV0 {
            id: properties.remove_identifier(property_names::ID)?,
            owner_id: properties.remove_identifier(property_names::OWNER_ID)?,
            properties: BTreeMap::new(),
            revision: properties.remove_optional_integer(property_names::REVISION)?,
            created_at: properties.remove_optional_integer(property_names::CREATED_AT)?,
            updated_at: properties.remove_optional_integer(property_names::UPDATED_AT)?,
        };

        document.properties = properties;
        Ok(document)
    }
}
