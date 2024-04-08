use crate::document::fields::property_names;
use crate::document::serialization_traits::{
    DocumentJsonMethodsV0, DocumentPlatformValueMethodsV0,
};
use crate::document::DocumentV0;
use crate::util::json_value::JsonValueExt;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use std::convert::TryInto;

impl<'a> DocumentJsonMethodsV0<'a> for DocumentV0 {
    fn to_json_with_identifiers_using_bytes(
        &self,
        _platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError> {
        let mut value = json!({
            property_names::ID: self.id,
            property_names::OWNER_ID: self.owner_id,
        });
        let value_mut = value.as_object_mut().unwrap();
        if let Some(created_at) = self.created_at {
            value_mut.insert(
                property_names::CREATED_AT.to_string(),
                JsonValue::Number(created_at.into()),
            );
        }
        if let Some(updated_at) = self.updated_at {
            value_mut.insert(
                property_names::UPDATED_AT.to_string(),
                JsonValue::Number(updated_at.into()),
            );
        }
        if let Some(created_at_block_height) = self.created_at_block_height {
            value_mut.insert(
                property_names::CREATED_AT_BLOCK_HEIGHT.to_string(),
                JsonValue::Number(created_at_block_height.into()),
            );
        }

        if let Some(updated_at_block_height) = self.updated_at_block_height {
            value_mut.insert(
                property_names::UPDATED_AT_BLOCK_HEIGHT.to_string(),
                JsonValue::Number(updated_at_block_height.into()),
            );
        }

        if let Some(created_at_core_block_height) = self.created_at_core_block_height {
            value_mut.insert(
                property_names::CREATED_AT_CORE_BLOCK_HEIGHT.to_string(),
                JsonValue::Number(created_at_core_block_height.into()),
            );
        }

        if let Some(updated_at_core_block_height) = self.updated_at_core_block_height {
            value_mut.insert(
                property_names::UPDATED_AT_CORE_BLOCK_HEIGHT.to_string(),
                JsonValue::Number(updated_at_core_block_height.into()),
            );
        }
        if let Some(revision) = self.revision {
            value_mut.insert(
                property_names::REVISION.to_string(),
                JsonValue::Number(revision.into()),
            );
        }

        self.properties
            .iter()
            .try_for_each(|(key, property_value)| {
                let serde_value: JsonValue = property_value.try_to_validating_json()?;
                value_mut.insert(key.to_string(), serde_value);
                Ok::<(), ProtocolError>(())
            })?;

        Ok(value)
    }

    fn to_json(&self, _platform_version: &PlatformVersion) -> Result<JsonValue, ProtocolError> {
        self.to_object()
            .map(|v| v.try_into().map_err(ProtocolError::ValueError))?
    }

    fn from_json_value<S>(
        mut document_value: JsonValue,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = ProtocolError>,
    {
        let mut document = Self {
            ..Default::default()
        };

        if let Ok(value) = document_value.remove(property_names::ID) {
            let data: S = serde_json::from_value(value)?;
            document.id = data.try_into()?;
        }
        if let Ok(value) = document_value.remove(property_names::OWNER_ID) {
            let data: S = serde_json::from_value(value)?;
            document.owner_id = data.try_into()?;
        }
        if let Ok(value) = document_value.remove(property_names::REVISION) {
            document.revision = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(property_names::CREATED_AT) {
            document.created_at = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(property_names::UPDATED_AT) {
            document.updated_at = serde_json::from_value(value)?
        }
        if let Ok(value) = document_value.remove(property_names::CREATED_AT_BLOCK_HEIGHT) {
            document.created_at_block_height = serde_json::from_value(value)?;
        }
        if let Ok(value) = document_value.remove(property_names::UPDATED_AT_BLOCK_HEIGHT) {
            document.updated_at_block_height = serde_json::from_value(value)?;
        }
        if let Ok(value) = document_value.remove(property_names::CREATED_AT_CORE_BLOCK_HEIGHT) {
            document.created_at_core_block_height = serde_json::from_value(value)?;
        }
        if let Ok(value) = document_value.remove(property_names::UPDATED_AT_CORE_BLOCK_HEIGHT) {
            document.updated_at_core_block_height = serde_json::from_value(value)?;
        }

        let platform_value: Value = document_value.into();

        document.properties = platform_value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        Ok(document)
    }
}
