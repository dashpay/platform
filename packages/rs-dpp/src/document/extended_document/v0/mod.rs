mod cbor_conversion;
mod json_conversion;
mod platform_value_conversion;
mod serialize;

use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::DataContract;
use crate::document::extended_document::property_names;
use crate::document::{Document, DocumentV0Getters};
use crate::identity::TimestampMillis;
use crate::metadata::Metadata;
use crate::prelude::Revision;
use crate::serialization::{PlatformDeserializable, ValueConvertible};
#[cfg(feature = "cbor")]
use crate::util::cbor_value::CborCanonicalMap;
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::util::hash::hash_to_vec;
use crate::version::{FeatureVersion, LATEST_PLATFORM_VERSION};
use crate::ProtocolError;
#[cfg(feature = "cbor")]
use ciborium::Value as CborValue;

use platform_value::btreemap_extensions::{
    BTreeValueMapPathHelper, BTreeValueMapReplacementPathHelper, BTreeValueRemoveFromMapHelper,
};
use platform_value::{Bytes32, Identifier, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, HashSet};
use std::convert::TryInto;

use crate::data_contract::base::DataContractBaseMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::identifiers_and_binary_paths::DataContractIdentifiersAndBinaryPathsMethodsV0;
#[cfg(feature = "cbor")]
use crate::document::serialization_traits::DocumentCborMethodsV0;

use crate::document::serialization_traits::{
    DocumentJsonMethodsV0, DocumentPlatformValueMethodsV0,
};
use platform_value::converter::serde_json::BTreeValueJsonConverter;
#[cfg(feature = "json-object")]
use serde_json::Value as JsonValue;

/// The `ExtendedDocumentV0` struct represents the data provided by the platform in response to a query.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExtendedDocumentV0 {
    /// The document type name, stored as a string.
    #[serde(rename = "$type")]
    pub document_type_name: String,

    /// The identifier of the associated data contract.
    #[serde(rename = "$dataContractId")]
    pub data_contract_id: Identifier,

    /// The actual document object containing the data.
    #[serde(flatten)]
    pub document: Document,

    /// The data contract associated with the document.
    #[serde(rename = "$dataContract")]
    pub data_contract: DataContract,

    /// An optional field for metadata associated with the document.
    #[serde(rename = "$metadata", default)]
    pub metadata: Option<Metadata>,

    /// A field representing the entropy, stored as `Bytes32`.
    #[serde(rename = "$entropy")]
    pub entropy: Bytes32,
}

impl ExtendedDocumentV0 {
    #[cfg(feature = "json-object")]
    pub(super) fn properties_as_json_data(&self) -> Result<JsonValue, ProtocolError> {
        self.document
            .properties()
            .to_json_value()
            .map_err(ProtocolError::ValueError)
    }

    pub fn get_optional_value(&self, key: &str) -> Option<&Value> {
        self.document.properties().get(key)
    }

    pub fn properties(&self) -> &BTreeMap<String, Value> {
        &self.document.properties()
    }

    pub fn properties_as_mut(&mut self) -> &mut BTreeMap<String, Value> {
        &mut self.document.properties()
    }

    pub fn id(&self) -> Identifier {
        self.document.id()
    }

    pub fn owner_id(&self) -> Identifier {
        self.document.owner_id()
    }

    pub fn document_type(&self) -> Result<DocumentTypeRef, ProtocolError> {
        // We can unwrap because the Document can not be created without a valid Document Type
        self.data_contract
            .document_type_for_name(self.document_type_name.as_str())
    }

    pub fn can_be_modified(&self) -> Result<bool, ProtocolError> {
        self.document_type()
            .map(|document_type| document_type.documents_mutable())
    }

    pub fn needs_revision(&self) -> Result<bool, ProtocolError> {
        self.document_type()
            .map(|document_type| document_type.documents_mutable())
    }

    pub fn revision(&self) -> Option<&Revision> {
        self.document.revision().as_ref()
    }

    pub fn created_at(&self) -> Option<&TimestampMillis> {
        self.document.created_at().as_ref()
    }

    pub fn updated_at(&self) -> Option<&TimestampMillis> {
        self.document.updated_at().as_ref()
    }

    /// Create an extended document with additional information.
    ///
    /// # Arguments
    ///
    /// * `document` - A `Document` instance.
    /// * `data_contract` - A `DataContract` instance.
    /// * `document_type_name` - A `String` representing the document type name.
    pub fn from_document_with_additional_info(
        document: Document,
        data_contract: DataContract,
        document_type_name: String,
    ) -> Self {
        Self {
            document_type_name,
            data_contract_id: data_contract.id(),
            document,
            data_contract,
            metadata: None,
            entropy: Default::default(),
        }
    }

    #[cfg(feature = "json-object")]
    /// Create an extended document from a JSON string.
    ///
    /// # Arguments
    ///
    /// * `string` - A JSON string representing the extended document.
    /// * `contract` - A `DataContract` instance.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an error decoding the JSON string.
    pub fn from_json_string(string: &str, contract: DataContract) -> Result<Self, ProtocolError> {
        let json_value: JsonValue = serde_json::from_str(string).map_err(|_| {
            ProtocolError::StringDecodeError("error decoding from json string".to_string())
        })?;
        Self::from_untrusted_platform_value(json_value.into(), contract)
    }

    #[cfg(feature = "json-object")]
    /// Create an extended document from a raw JSON document.
    ///
    /// # Arguments
    ///
    /// * `raw_document` - A `JsonValue` representing the raw document.
    /// * `data_contract` - A `DataContract` instance.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an error processing the raw JSON document.
    pub fn from_raw_json_document(
        raw_document: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        Self::from_untrusted_platform_value(raw_document.into(), data_contract)
    }

    /// Create an extended document from a trusted platform value object where fields are already in
    /// the proper format for the contract.
    ///
    /// # Arguments
    ///
    /// * `document_value` - A `Value` representing the document value.
    /// * `data_contract` - A `DataContract` instance.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an error processing the trusted platform value.
    pub fn from_trusted_platform_value(
        document_value: Value,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let mut properties = document_value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        let document_type_name = properties
            .remove_string(property_names::DOCUMENT_TYPE)
            .map_err(ProtocolError::ValueError)?;

        let mut extended_document = Self {
            data_contract,
            document_type_name,
            ..Default::default()
        };

        // if the protocol version is not set, use the current protocol version
        extended_document.feature_version = properties
            .remove_optional_integer(property_names::FEATURE_VERSION)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or(
                LATEST_PLATFORM_VERSION
                    .extended_document
                    .default_current_version,
            );
        extended_document.data_contract_id = Identifier::new(
            properties
                .remove_optional_hash256_bytes(property_names::DATA_CONTRACT_ID)?
                .unwrap_or(extended_document.data_contract.id().to_buffer()),
        );
        extended_document.document = Document::from_map(properties, None, None)?;
        Ok(extended_document)
    }

    /// Create an extended document from an untrusted platform value object where fields might not
    /// be in the proper format for the contract.
    ///
    /// # Arguments
    ///
    /// * `document_value` - A `Value` representing the document value.
    /// * `data_contract` - A `DataContract` instance.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an error processing the untrusted platform value.
    pub fn from_untrusted_platform_value(
        document_value: Value,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        let mut properties = document_value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        let document_type_name = properties
            .remove_string(property_names::DOCUMENT_TYPE)
            .map_err(ProtocolError::ValueError)?;

        //Because we don't know how the json came in we need to sanitize it
        let (identifiers, binary_paths): (HashSet<_>, HashSet<_>) =
            data_contract.get_identifiers_and_binary_paths_owned(document_type_name.as_str())?;

        let mut extended_document = Self {
            data_contract,
            document_type_name,
            ..Default::default()
        };

        // if the protocol version is not set, use the current protocol version
        extended_document.feature_version = properties
            .remove_optional_integer(property_names::FEATURE_VERSION)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or(
                LATEST_PLATFORM_VERSION
                    .extended_document
                    .default_current_version,
            );
        extended_document.data_contract_id = properties
            .remove_optional_identifier(property_names::DATA_CONTRACT_ID)?
            .unwrap_or(extended_document.data_contract.id);
        extended_document.document = Document::from_map(properties, None, None)?;

        extended_document
            .document
            .properties()
            .replace_at_paths(identifiers, ReplacementType::Identifier)?;
        extended_document
            .document
            .properties()
            .replace_at_paths(binary_paths, ReplacementType::BinaryBytes)?;
        Ok(extended_document)
    }

    #[cfg(feature = "json-object")]
    /// Convert the extended document to a JSON object.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an error converting the document to JSON.
    pub fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let mut value = self.document.to_json()?;
        let value_mut = value.as_object_mut().unwrap();
        value_mut.insert(
            property_names::FEATURE_VERSION.to_string(),
            JsonValue::Number(self.feature_version.into()),
        );
        value_mut.insert(
            property_names::DOCUMENT_TYPE.to_string(),
            JsonValue::String(self.document_type_name.clone()),
        );
        value_mut.insert(
            property_names::DATA_CONTRACT_ID.to_string(),
            json!(self.data_contract.id),
        );
        Ok(value)
    }

    #[cfg(feature = "json-object")]
    /// Convert the extended document to a pretty JSON object.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an error converting the document to pretty JSON.
    pub fn to_pretty_json(&self) -> Result<JsonValue, ProtocolError> {
        let mut value = self.document.to_json()?;
        let value_mut = value.as_object_mut().unwrap();
        value_mut.insert(
            property_names::FEATURE_VERSION.to_string(),
            JsonValue::Number(self.feature_version.into()),
        );
        value_mut.insert(
            property_names::DOCUMENT_TYPE.to_string(),
            JsonValue::String(self.document_type_name.clone()),
        );
        value_mut.insert(
            property_names::DATA_CONTRACT_ID.to_string(),
            JsonValue::String(bs58::encode(self.data_contract_id.to_buffer()).into_string()),
        );
        Ok(value)
    }

    #[cfg(feature = "cbor")]
    pub fn from_cbor_buffer(cbor_bytes: impl AsRef<[u8]>) -> Result<Self, ProtocolError> {
        let SplitProtocolVersionOutcome {
            protocol_version,
            main_message_bytes: document_cbor_bytes,
            ..
        } = deserializer::split_cbor_protocol_version(cbor_bytes.as_ref())?;

        let document_cbor_map: BTreeMap<String, CborValue> =
            ciborium::de::from_reader(document_cbor_bytes)
                .map_err(|e| ProtocolError::EncodingError(format!("{}", e)))?;

        let mut document_map: BTreeMap<String, Value> =
            Value::convert_from_cbor_map(document_cbor_map)?;

        let data_contract_id = Identifier::new(
            document_map
                .remove_hash256_bytes(property_names::DATA_CONTRACT_ID)
                .map_err(ProtocolError::ValueError)?,
        );

        let document_type_name = document_map.remove_string(property_names::DOCUMENT_TYPE)?;

        let document = Document::from_map(document_map, None, None)?;
        Ok(ExtendedDocumentV0 {
            document_type_name,
            data_contract_id,
            document,
            ..Default::default()
        })
    }

    pub fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut object = self.document.to_map_value()?;
        object.insert(
            property_names::FEATURE_VERSION.to_string(),
            self.feature_version.into(),
        );
        object.insert(
            property_names::DOCUMENT_TYPE.to_string(),
            Value::Text(self.document_type_name.clone()),
        );
        object.insert(
            property_names::DATA_CONTRACT_ID.to_string(),
            Value::Identifier(self.data_contract_id.to_buffer()),
        );
        Ok(object)
    }

    pub fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let ExtendedDocumentV0 {
            document_type_name,
            data_contract_id,
            document,
            ..
        } = self;

        let mut object = document.into_map_value()?;
        object.insert(property_names::FEATURE_VERSION.to_string(), Value::U16(0));
        object.insert(
            property_names::DOCUMENT_TYPE.to_string(),
            Value::Text(document_type_name),
        );
        object.insert(
            property_names::DATA_CONTRACT_ID.to_string(),
            Value::Identifier(data_contract_id.to_buffer()),
        );
        Ok(object)
    }

    pub fn into_value(self) -> Result<Value, ProtocolError> {
        Ok(self.into_map_value()?.into())
    }

    pub fn to_value(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_map_value()?.into())
    }

    #[cfg(feature = "json-object")]
    pub fn to_json_object_for_validation(&self) -> Result<JsonValue, ProtocolError> {
        self.to_value()?
            .try_into_validating_json()
            .map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "cbor")]
    pub fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut result_buf = self.feature_version.encode_var_vec();

        let mut cbor_value = self.document.to_cbor_value()?;
        let value_mut = cbor_value.as_map_mut().unwrap();

        value_mut.push((
            CborValue::Text(property_names::DOCUMENT_TYPE.to_string()),
            CborValue::Text(self.document_type_name.clone()),
        ));

        value_mut.push((
            CborValue::Text(property_names::DATA_CONTRACT_ID.to_string()),
            CborValue::Bytes(self.data_contract_id.to_vec()),
        ));

        let canonical_map: CborCanonicalMap = cbor_value.try_into()?;

        let mut document_buffer = canonical_map
            .to_bytes()
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        result_buf.append(&mut document_buffer);

        Ok(result_buf)
    }

    #[cfg(feature = "cbor")]
    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash_to_vec(self.to_cbor_buffer()?))
    }

    /// Set the value under given path.
    /// The path supports syntax from `lodash` JS lib. Example: "root.people[0].name".
    /// If parents are not present they will be automatically created
    pub fn set(&mut self, path: &str, value: Value) -> Result<(), ProtocolError> {
        Ok(self.document.properties.insert_at_path(path, value)?)
    }

    /// Retrieves field specified by path
    pub fn get(&self, path: &str) -> Option<&Value> {
        self.properties().get_optional_at_path(path).ok().flatten()
    }

    pub fn get_identifiers_and_binary_paths(
        &self,
    ) -> Result<(HashSet<&str>, HashSet<&str>), ProtocolError> {
        let (mut identifiers_paths, binary_paths) = self
            .data_contract
            .get_identifiers_and_binary_paths(&self.document_type_name)?;

        identifiers_paths.extend(super::IDENTIFIER_FIELDS);

        Ok((identifiers_paths, binary_paths))
    }

    pub fn get_identifiers_and_binary_paths_owned<
        I: IntoIterator<Item = String> + Extend<String> + Default,
    >(
        &self,
    ) -> Result<(I, I), ProtocolError> {
        let (mut identifiers_paths, binary_paths): (I, I) = self
            .data_contract
            .get_identifiers_and_binary_paths_owned(&self.document_type_name)?;

        identifiers_paths.extend(super::IDENTIFIER_FIELDS.map(|str| str.to_string()));

        Ok((identifiers_paths, binary_paths))
    }
}

impl PlatformDeserializable for ExtendedDocumentV0 {
    fn deserialize(data: &[u8]) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        todo!()
    }
}

impl<'a> ValueConvertible<'a> for ExtendedDocumentV0 {
    fn to_object(&self) -> Result<Value, ProtocolError> {
        todo!()
    }

    fn into_object(self) -> Result<Value, ProtocolError> {
        todo!()
    }

    fn from_object(value: Value) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        todo!()
    }

    fn from_object_ref(value: &Value) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        todo!()
    }
}
