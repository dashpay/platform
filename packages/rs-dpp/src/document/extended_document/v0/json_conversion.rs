use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use crate::document::extended_document::fields::property_names;
use crate::document::extended_document::v0::ExtendedDocumentV0;
use crate::document::serialization_traits::{
    DocumentJsonMethodsV0, DocumentPlatformValueMethodsV0,
};
use crate::document::DocumentV0;
use crate::serialization::ValueConvertible;
use crate::util::json_value::JsonValueExt;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use std::convert::TryInto;

impl DocumentJsonMethodsV0 for ExtendedDocumentV0 {
    fn to_json_with_identifiers_using_bytes(&self) -> Result<JsonValue, ProtocolError> {
        let mut json = self.document.to_json_with_identifiers_using_bytes()?;
        let mut value_mut = json.as_object_mut().unwrap();
        let contract = self.data_contract.to_json_object()?;
        value_mut.insert(property_names::DATA_CONTRACT.to_owned(), contract);
        value_mut.insert(
            property_names::DOCUMENT_TYPE_NAME.to_owned(),
            self.document_type_name.into(),
        );
        Ok(value)
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_object()
            .map(|v| v.try_into().map_err(ProtocolError::ValueError))?
    }

    fn from_json_value<S>(
        mut document_value: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = ProtocolError>,
    {
        Self::from_platform_value(document_value.into(), platform_version)
    }
}
