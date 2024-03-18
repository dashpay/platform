#[cfg(feature = "document-json-conversion")]
mod json_conversion;
#[cfg(feature = "document-value-conversion")]
mod platform_value_conversion;
mod serialize;

use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::DataContract;
#[cfg(any(
    feature = "document-value-conversion",
    feature = "document-json-conversion"
))]
use crate::document::extended_document::fields::property_names;
use crate::document::{Document, DocumentV0Getters, ExtendedDocument};
use crate::identity::TimestampMillis;
use crate::metadata::Metadata;
use crate::prelude::{BlockHeight, CoreBlockHeight, Revision};

use crate::util::hash::hash_double_to_vec;
use crate::ProtocolError;

use platform_value::btreemap_extensions::{
    BTreeValueMapInsertionPathHelper, BTreeValueMapPathHelper,
};
#[cfg(feature = "document-value-conversion")]
use platform_value::btreemap_extensions::{
    BTreeValueMapReplacementPathHelper, BTreeValueRemoveFromMapHelper,
};
use platform_value::{Bytes32, Identifier, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
#[cfg(feature = "validation")]
use crate::data_contract::validation::DataContractValidationMethodsV0;
#[cfg(feature = "document-json-conversion")]
use crate::document::serialization_traits::DocumentJsonMethodsV0;
#[cfg(feature = "document-value-conversion")]
use crate::document::serialization_traits::DocumentPlatformValueMethodsV0;
use crate::document::serialization_traits::ExtendedDocumentPlatformConversionMethodsV0;
#[cfg(feature = "validation")]
use crate::validation::SimpleConsensusValidationResult;
#[cfg(feature = "document-json-conversion")]
use platform_value::converter::serde_json::BTreeValueJsonConverter;
use platform_version::version::PlatformVersion;
#[cfg(feature = "document-json-conversion")]
use serde_json::Value as JsonValue;

/// The `ExtendedDocumentV0` struct represents the data provided by the platform in response to a query.
#[derive(Debug, Clone)]
#[cfg_attr(
    all(
        feature = "document-serde-conversion",
        feature = "data-contract-serde-conversion"
    ),
    derive(Serialize, Deserialize)
)]
pub struct ExtendedDocumentV0 {
    /// The document type name, stored as a string.
    #[cfg_attr(
        all(
            feature = "document-serde-conversion",
            feature = "data-contract-serde-conversion"
        ),
        serde(rename = "$type")
    )]
    pub document_type_name: String,

    /// The identifier of the associated data contract.
    #[cfg_attr(
        all(
            feature = "document-serde-conversion",
            feature = "data-contract-serde-conversion"
        ),
        serde(rename = "$dataContractId")
    )]
    pub data_contract_id: Identifier,

    /// The actual document object containing the data.
    #[cfg_attr(
        all(
            feature = "document-serde-conversion",
            feature = "data-contract-serde-conversion"
        ),
        serde(flatten)
    )]
    pub document: Document,

    // TODO: We should remove it from here, or at least keep a ref
    //  also there is no point to keep both contract and its ID
    /// The data contract associated with the document.
    #[cfg_attr(
        all(
            feature = "document-serde-conversion",
            feature = "data-contract-serde-conversion"
        ),
        serde(rename = "$dataContract")
    )]
    pub data_contract: DataContract,

    /// An optional field for metadata associated with the document.
    #[cfg_attr(
        all(
            feature = "document-serde-conversion",
            feature = "data-contract-serde-conversion"
        ),
        serde(rename = "$metadata", default)
    )]
    pub metadata: Option<Metadata>,

    /// A field representing the entropy, stored as `Bytes32`.
    #[cfg_attr(
        all(
            feature = "document-serde-conversion",
            feature = "data-contract-serde-conversion"
        ),
        serde(rename = "$entropy")
    )]
    pub entropy: Bytes32,
}

impl ExtendedDocumentV0 {
    #[cfg(feature = "document-json-conversion")]
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
        self.document.properties()
    }

    pub fn properties_as_mut(&mut self) -> &mut BTreeMap<String, Value> {
        self.document.properties_mut()
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
            .map_err(ProtocolError::DataContractError)
    }

    pub fn can_be_modified(&self) -> Result<bool, ProtocolError> {
        self.document_type()
            .map(|document_type| document_type.documents_mutable())
    }

    pub fn needs_revision(&self) -> Result<bool, ProtocolError> {
        self.document_type()
            .map(|document_type| document_type.documents_mutable())
    }

    pub fn revision(&self) -> Option<Revision> {
        self.document.revision()
    }

    /// Returns an optional block timestamp at which the document was created.
    /// It will be None if it is not required by the schema.
    pub fn created_at(&self) -> Option<TimestampMillis> {
        self.document.created_at()
    }

    /// Returns an optional block timestamp at which the document was updated.
    /// It will be None if it is not required by the schema.
    pub fn updated_at(&self) -> Option<TimestampMillis> {
        self.document.updated_at()
    }

    /// Returns an optional block height at which the document was created.
    /// It will be None if it is not required by the schema.
    pub fn created_at_block_height(&self) -> Option<BlockHeight> {
        self.document.created_at_block_height()
    }

    /// Returns an optional block height at which the document was last updated.
    /// It will be None if it is not required by the schema.
    pub fn updated_at_block_height(&self) -> Option<BlockHeight> {
        self.document.updated_at_block_height()
    }

    /// Returns an optional core block height at which the document was created.
    /// It will be None if it is not required by the schema.
    pub fn created_at_core_block_height(&self) -> Option<CoreBlockHeight> {
        self.document.created_at_core_block_height()
    }

    /// Returns an optional core block height at which the document was last updated.
    /// It will be None if it is not required by the schema.
    pub fn updated_at_core_block_height(&self) -> Option<CoreBlockHeight> {
        self.document.updated_at_core_block_height()
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

    #[cfg(feature = "document-json-conversion")]
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
    pub fn from_json_string(
        string: &str,
        contract: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let json_value: JsonValue = serde_json::from_str(string).map_err(|_| {
            ProtocolError::StringDecodeError("error decoding from json string".to_string())
        })?;
        Self::from_untrusted_platform_value(json_value.into(), contract, platform_version)
    }

    #[cfg(feature = "document-json-conversion")]
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
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Self::from_untrusted_platform_value(raw_document.into(), data_contract, platform_version)
    }

    #[cfg(feature = "document-value-conversion")]
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
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut properties = document_value
            .clone()
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        let document_type_name = properties
            .remove_string(property_names::DOCUMENT_TYPE_NAME)
            .map_err(ProtocolError::ValueError)?;

        let document = Document::from_platform_value(document_value, platform_version)?;

        let data_contract_id = data_contract.id();
        let mut extended_document = Self {
            data_contract,
            document_type_name,
            document,
            data_contract_id,
            metadata: None,
            entropy: Default::default(),
        };

        extended_document.data_contract_id = Identifier::new(
            properties
                .remove_optional_hash256_bytes(property_names::DATA_CONTRACT_ID)?
                .unwrap_or(extended_document.data_contract.id().to_buffer()),
        );
        Ok(extended_document)
    }

    #[cfg(feature = "document-value-conversion")]
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
        mut document_value: Value,
        data_contract: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut properties = document_value
            .clone()
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        let document_type_name = properties
            .remove_string(property_names::DOCUMENT_TYPE_NAME)
            .map_err(ProtocolError::ValueError)?;

        let document_type = data_contract.document_type_for_name(document_type_name.as_str())?;

        let identifiers = document_type.identifier_paths().to_owned();
        let binary_paths = document_type.binary_paths();

        document_value.replace_at_paths(["$id", "$ownerId"], ReplacementType::Identifier)?;
        document_value.replace_at_paths(
            identifiers.iter().map(|s| s.as_str()),
            ReplacementType::Identifier,
        )?;
        document_value.replace_at_paths(
            binary_paths.iter().map(|s| s.as_str()),
            ReplacementType::BinaryBytes,
        )?;

        let document = Document::from_platform_value(document_value, platform_version)?;
        let data_contract_id = data_contract.id();
        let mut extended_document = Self {
            data_contract,
            document_type_name,
            document,
            data_contract_id,
            metadata: None,
            entropy: Default::default(),
        };

        extended_document.data_contract_id = properties
            .remove_optional_identifier(property_names::DATA_CONTRACT_ID)?
            .unwrap_or(extended_document.data_contract.id());
        extended_document
            .document
            .properties_mut()
            .replace_at_paths(&identifiers, ReplacementType::Identifier)?;
        Ok(extended_document)
    }

    #[cfg(feature = "document-json-conversion")]
    /// Convert the extended document to a pretty JSON object.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if there is an error converting the document to pretty JSON.
    pub fn to_pretty_json(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError> {
        let mut value = self.document.to_json(platform_version)?;
        let value_mut = value.as_object_mut().unwrap();
        value_mut.insert(
            property_names::DOCUMENT_TYPE_NAME.to_string(),
            JsonValue::String(self.document_type_name.clone()),
        );
        value_mut.insert(
            property_names::DATA_CONTRACT_ID.to_string(),
            JsonValue::String(bs58::encode(self.data_contract_id.to_buffer()).into_string()),
        );
        Ok(value)
    }

    #[cfg(feature = "document-value-conversion")]
    pub fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut object = self.document.to_map_value()?;
        object.insert(
            property_names::DOCUMENT_TYPE_NAME.to_string(),
            Value::Text(self.document_type_name.clone()),
        );
        object.insert(
            property_names::DATA_CONTRACT_ID.to_string(),
            Value::Identifier(self.data_contract_id.to_buffer()),
        );
        Ok(object)
    }

    #[cfg(feature = "document-value-conversion")]
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
            property_names::DOCUMENT_TYPE_NAME.to_string(),
            Value::Text(document_type_name),
        );
        object.insert(
            property_names::DATA_CONTRACT_ID.to_string(),
            Value::Identifier(data_contract_id.to_buffer()),
        );
        Ok(object)
    }

    #[cfg(feature = "document-value-conversion")]
    pub fn into_value(self) -> Result<Value, ProtocolError> {
        Ok(self.into_map_value()?.into())
    }

    #[cfg(feature = "document-value-conversion")]
    pub fn to_value(&self) -> Result<Value, ProtocolError> {
        Ok(self.to_map_value()?.into())
    }

    #[cfg(feature = "document-json-conversion")]
    pub fn to_json_object_for_validation(&self) -> Result<JsonValue, ProtocolError> {
        self.to_value()?
            .try_into_validating_json()
            .map_err(ProtocolError::ValueError)
    }

    pub fn hash(&self, platform_version: &PlatformVersion) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash_double_to_vec(
            ExtendedDocumentPlatformConversionMethodsV0::serialize_to_bytes(
                self,
                platform_version,
            )?,
        ))
    }

    /// Set the value under given path.
    /// The path supports syntax from `lodash` JS lib. Example: "root.people[0].name".
    /// If parents are not present they will be automatically created
    pub fn set(&mut self, path: &str, value: Value) -> Result<(), ProtocolError> {
        Ok(self.document.properties_mut().insert_at_path(path, value)?)
    }

    /// Set the value under given path.
    /// The path supports syntax from `lodash` JS lib. Example: "root.people[0].name".
    /// If parents are not present they will be automatically created
    pub fn set_untrusted(&mut self, path: &str, value: Value) -> Result<(), ProtocolError> {
        let document_type = self.document_type()?;

        let identifiers = document_type.identifier_paths();
        let binary_paths = document_type.binary_paths();

        if identifiers.contains(path) {
            let value =
                ReplacementType::Identifier.replace_for_bytes(value.to_identifier_bytes()?)?;
            self.set(path, value)
        } else if binary_paths.contains(path) {
            let value =
                ReplacementType::BinaryBytes.replace_for_bytes(value.to_identifier_bytes()?)?;
            self.set(path, value)
        } else {
            self.set(path, value)
        }
    }

    /// Retrieves field specified by path
    pub fn get(&self, path: &str) -> Option<&Value> {
        self.properties().get_optional_at_path(path).ok().flatten()
    }

    // TODO: We probably should validate extended document on creation and during modification
    //  instead of have a dedicated method. That would be more Rust way approach
    #[cfg(feature = "validation")]
    /// Validate external document
    pub fn validate(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        self.data_contract.validate_document(
            &self.document_type_name,
            &self.document,
            platform_version,
        )
    }
}

impl From<ExtendedDocumentV0> for ExtendedDocument {
    fn from(value: ExtendedDocumentV0) -> Self {
        ExtendedDocument::V0(value)
    }
}
