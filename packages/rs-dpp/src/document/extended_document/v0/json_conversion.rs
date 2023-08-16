use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use crate::document::extended_document::fields::property_names;
use crate::document::extended_document::v0::ExtendedDocumentV0;
use crate::document::serialization_traits::{
    DocumentJsonMethodsV0, DocumentPlatformValueMethodsV0,
};

use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::convert::TryInto;

impl DocumentJsonMethodsV0<'_> for ExtendedDocumentV0 {
    fn to_json_with_identifiers_using_bytes(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError> {
        let mut json = self
            .document
            .to_json_with_identifiers_using_bytes(platform_version)?;
        let value_mut = json.as_object_mut().unwrap();
        let contract = self.data_contract.to_validating_json(platform_version)?;
        value_mut.insert(property_names::DATA_CONTRACT.to_owned(), contract);
        value_mut.insert(
            property_names::DOCUMENT_TYPE_NAME.to_owned(),
            self.document_type_name.clone().into(),
        );
        Ok(json)
    }

    fn to_json(&self, platform_version: &PlatformVersion) -> Result<JsonValue, ProtocolError> {
        let mut json = self.document.to_json(platform_version)?;
        let value_mut = json.as_object_mut().unwrap();
        let contract = self.data_contract.to_json(platform_version)?;
        value_mut.insert(property_names::DATA_CONTRACT.to_owned(), contract);
        value_mut.insert(
            property_names::DOCUMENT_TYPE_NAME.to_owned(),
            self.document_type_name.clone().into(),
        );
        Ok(json)
    }

    fn from_json_value<S>(
        document_value: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = ProtocolError>,
    {
        Self::from_platform_value(document_value.into(), platform_version)
    }
}
