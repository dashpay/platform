use bincode::de::{BorrowDecoder, Decoder};
use bincode::enc::Encoder;
use bincode::error::{DecodeError, EncodeError};
use bincode::{BorrowDecode, Decode, Encode};
use std::collections::{BTreeMap, HashSet};
use std::convert::{TryFrom, TryInto};

use crate::serialization_traits::PlatformSerializable;
use itertools::{Either, Itertools};
use platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueRemoveFromMapHelper};
use platform_value::Identifier;
use platform_value::{ReplacementType, Value, ValueMapHelper};
use serde::de::Error;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::consensus::basic::document::InvalidDocumentTypeError;
use crate::data_contract::contract_config;
use crate::data_contract::contract_config::{
    ContractConfigV0, DEFAULT_CONTRACT_CAN_BE_DELETED, DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
    DEFAULT_CONTRACT_DOCUMENT_MUTABILITY, DEFAULT_CONTRACT_KEEPS_HISTORY,
    DEFAULT_CONTRACT_MUTABILITY,
};

use crate::data_contract::get_binary_properties_from_schema::get_binary_properties;

use crate::data_contract::document_type::v0::DocumentTypeV0;
#[cfg(feature = "cbor")]
use crate::util::cbor_serializer;
use crate::{errors::ProtocolError, metadata::Metadata, util::hash::hash_to_vec};
use crate::{identifier, Convertible};
use platform_value::string_encoding::Encoding;

use super::document_type::DocumentType;
use crate::data_contract::errors::DataContractError;
use crate::version::LATEST_VERSION;

use super::super::property_names;

pub type JsonSchema = JsonValue;
type DefinitionName = String;
pub type DocumentName = String;
type PropertyPath = String;

pub const DATA_CONTRACT_SCHEMA_URI_V0: &str =
    "https://schema.dash.org/dpp-0-4-0/meta/data-contract";

pub const DATA_CONTRACT_IDENTIFIER_FIELDS_V0: [&str; 2] =
    [property_names::ID, property_names::OWNER_ID];
pub const DATA_CONTRACT_BINARY_FIELDS_V0: [&str; 1] = [property_names::ENTROPY];

impl Convertible for DataContractV0 {
    #[cfg(feature = "platform-value")]
    fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "platform-value")]
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        let mut value = platform_value::to_value(self).map_err(ProtocolError::ValueError)?;
        if self.defs.is_none() {
            value.remove(property_names::DEFINITIONS)?;
        }
        Ok(value)
    }

    #[cfg(feature = "platform-value")]
    fn into_object(self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    #[cfg(feature = "json-object")]
    fn to_json_object(&self) -> Result<JsonValue, ProtocolError> {
        self.to_object()?
            .try_into_validating_json()
            .map_err(ProtocolError::ValueError)
    }

    /// Returns Data Contract as a JSON Value
    #[cfg(feature = "json-object")]
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        self.to_object()?
            .try_into()
            .map_err(ProtocolError::ValueError)
    }

    /// Returns Data Contract as a Buffer
    #[cfg(feature = "cbor")]
    fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let protocol_version = self.data_contract_protocol_version;

        let mut object = self.to_object()?;
        object.remove(property_names::PROTOCOL_VERSION)?;
        if self.defs.is_none() {
            object.remove(property_names::DEFINITIONS)?;
        }
        object
            .to_map_mut()
            .unwrap()
            .sort_by_lexicographical_byte_ordering_keys_and_inner_maps();

        cbor_serializer::serializable_value_to_cbor(&object, Some(protocol_version))
    }
}

/// `DataContractV0` represents a data contract in a decentralized platform.
///
/// It contains information about the contract, such as its protocol version, unique identifier,
/// schema, version, and owner identifier. The struct also includes details about the document
/// types, metadata, configuration, and document schemas associated with the contract.
///
/// Additionally, `DataContractV0` holds definitions for JSON schemas, entropy, and binary properties
/// of the documents.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(try_from = "DataContractV0Inner")]
#[serde(rename_all = "camelCase")]
pub struct DataContractV0 {
    /// A unique identifier for the data contract.
    #[serde(rename = "$id")]
    pub id: Identifier,

    /// A reference to the JSON schema that defines the contract.
    #[serde(rename = "$schema")]
    pub schema: String,

    /// The version of this data contract.
    pub version: u32,

    /// The identifier of the contract owner.
    pub owner_id: Identifier,

    /// A mapping of document names to their corresponding document types.
    #[serde(skip)]
    pub document_types: BTreeMap<DocumentName, DocumentTypeV0>,

    /// Optional metadata associated with the contract.
    #[serde(skip)]
    pub metadata: Option<Metadata>,

    /// Internal configuration for the contract.
    #[serde(skip)]
    pub config: ContractConfigV0,

    /// A mapping of document names to their corresponding JSON schemas.
    pub documents: BTreeMap<DocumentName, JsonSchema>,

    /// Optional mapping of definition names to their corresponding JSON schemas.
    #[serde(rename = "$defs", default)]
    pub defs: Option<BTreeMap<DefinitionName, JsonSchema>>,

    /// A nested mapping of document names and property paths to their binary values.
    #[serde(skip)]
    pub binary_properties: BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>,
}

impl Encode for DataContractV0 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        let inner: DataContractV0Inner = self.clone().into();
        inner.encode(encoder)
    }
}

impl Decode for DataContractV0 {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let inner = DataContractV0Inner::decode(decoder)?;
        inner
            .try_into()
            .map_err(|e: ProtocolError| DecodeError::custom(e.to_string()))
    }
}

impl<'a> BorrowDecode<'a> for DataContractV0 {
    fn borrow_decode<D: BorrowDecoder<'a>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let inner = DataContractV0Inner::decode(decoder)?;
        inner
            .try_into()
            .map_err(|e: ProtocolError| DecodeError::custom(e.to_string()))
    }
}

// Standalone default_protocol_version function
fn default_protocol_version() -> u32 {
    1
}

#[derive(Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct DataContractV0Inner {
    /// A unique identifier for the data contract.
    #[serde(rename = "$id")]
    pub id: Identifier,

    /// Internal configuration for the contract.
    #[serde(default)]
    pub config: ContractConfigV0,

    /// A reference to the JSON schema that defines the contract.
    #[serde(rename = "$schema")]
    pub schema: String,

    /// The version of this data contract.
    pub version: u32,

    /// The identifier of the contract owner.
    pub owner_id: Identifier,

    /// A mapping of document names to their corresponding JSON values.
    pub documents: BTreeMap<DocumentName, Value>,

    /// Optional mapping of definition names to their corresponding JSON values.
    #[serde(rename = "$defs", default)]
    pub defs: Option<BTreeMap<DefinitionName, Value>>,
}

impl From<DataContractV0> for DataContractV0Inner {
    fn from(value: DataContractV0) -> Self {
        let DataContractV0 {
            id,
            config,
            schema,
            version,
            owner_id,
            documents,
            defs,
            ..
        } = value;
        DataContractV0Inner {
            id,
            config,
            schema,
            version,
            owner_id,
            documents: documents
                .into_iter()
                .map(|(key, value)| (key, value.into()))
                .collect(),
            defs: defs.map(|defs| {
                defs.into_iter()
                    .map(|(key, value)| (key, value.into()))
                    .collect()
            }),
        }
    }
}

impl TryFrom<DataContractV0Inner> for DataContractV0 {
    type Error = ProtocolError;

    fn try_from(value: DataContractV0Inner) -> Result<Self, Self::Error> {
        let DataContractV0Inner {
            id,
            config,
            schema,
            version,
            owner_id,
            documents,
            defs,
        } = value;

        let document_types = get_document_types_from_value_array(
            id,
            &documents
                .iter()
                .map(|(key, value)| (key.as_str(), value))
                .collect(),
            &defs
                .as_ref()
                .map(|defs| {
                    defs.iter()
                        .map(|(key, value)| Ok((key.clone(), value)))
                        .collect::<Result<BTreeMap<String, &Value>, ProtocolError>>()
                })
                .transpose()?
                .unwrap_or_default(),
            config.documents_keep_history_contract_default,
            config.documents_mutable_contract_default,
        )?;

        let binary_properties = documents
            .iter()
            .map(|(doc_type, schema)| Ok((String::from(doc_type), get_binary_properties(&schema.clone().try_into()?))))
            .collect::<Result<BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>, ProtocolError>>()?;

        let data_contract = DataContractV0 {
            id,
            schema,
            version,
            owner_id,
            document_types,
            metadata: None,
            config,
            documents: documents
                .into_iter()
                .map(|(key, value)| Ok((key, value.try_into()?)))
                .collect::<Result<BTreeMap<DocumentName, JsonSchema>, ProtocolError>>()?,
            defs: defs
                .map(|defs| {
                    defs.into_iter()
                        .map(|(key, value)| Ok((key, value.try_into()?)))
                        .collect::<Result<BTreeMap<DefinitionName, JsonSchema>, ProtocolError>>()
                })
                .transpose()?,
            binary_properties,
        };

        Ok(data_contract)
    }
}

impl DataContractV0 {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "platform-value")]
    pub fn from_raw_object(raw_object: Value) -> Result<DataContractV0, ProtocolError> {
        let mut data_contract_map = raw_object
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;

        let id = data_contract_map
            .remove_identifier(property_names::ID)
            .map_err(ProtocolError::ValueError)?;

        let mutability = get_contract_configuration_properties(&data_contract_map)?;
        let definition_references = get_definitions(&data_contract_map)?;
        let document_types = get_document_types_from_contract(
            id,
            &data_contract_map,
            &definition_references,
            mutability.documents_keep_history_contract_default,
            mutability.documents_mutable_contract_default,
        )?;

        let documents = data_contract_map
            .remove(property_names::DOCUMENTS)
            .map(|value| value.try_into_validating_btree_map_json())
            .transpose()?
            .unwrap_or_default();

        let mutability = get_contract_configuration_properties(&data_contract_map)?;

        // Defs
        let defs =
            data_contract_map.get_optional_inner_str_json_value_map::<BTreeMap<_, _>>("$defs")?;

        let binary_properties = documents
            .iter()
            .map(|(doc_type, schema)| (String::from(doc_type), get_binary_properties(schema)))
            .collect();

        let data_contract = DataContractV0 {
            id,
            schema: data_contract_map
                .remove_string(property_names::SCHEMA)
                .map_err(ProtocolError::ValueError)?,
            version: data_contract_map
                .remove_integer(property_names::VERSION)
                .map_err(ProtocolError::ValueError)?,
            owner_id: data_contract_map
                .remove_identifier(property_names::OWNER_ID)
                .map_err(ProtocolError::ValueError)?,
            document_types,
            metadata: None,
            config: mutability,
            documents,
            defs,
            binary_properties,
        };

        Ok(data_contract)
    }

    #[cfg(feature = "json-object")]
    pub fn from_json_object(json_value: JsonValue) -> Result<DataContractV0, ProtocolError> {
        let mut value: Value = json_value.into();
        value.replace_at_paths(DATA_CONTRACT_BINARY_FIELDS_V0, ReplacementType::BinaryBytes)?;
        value.replace_at_paths(
            DATA_CONTRACT_IDENTIFIER_FIELDS_V0,
            ReplacementType::Identifier,
        )?;
        Self::from_raw_object(value)
    }

    #[cfg(feature = "cbor")]
    pub fn from_cbor_buffer(b: impl AsRef<[u8]>) -> Result<DataContractV0, ProtocolError> {
        Self::from_cbor(b)
    }

    // Returns hash from Data Contract
    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash_to_vec(PlatformSerializable::serialize(self)?))
    }

    /// Increments version of Data Contract
    pub fn increment_version(&mut self) {
        self.version += 1;
    }

    /// Returns true if document type is defined
    pub fn is_document_defined(&self, document_type_name: &str) -> bool {
        self.document_types.get(document_type_name).is_some()
    }

    pub fn set_document_schema(
        &mut self,
        doc_type: String,
        schema: JsonSchema,
    ) -> Result<(), ProtocolError> {
        let binary_properties = get_binary_properties(&schema);
        self.documents.insert(doc_type.clone(), schema.clone());
        self.binary_properties
            .insert(doc_type.clone(), binary_properties);

        let document_type_value = platform_value::Value::from(schema);

        // Make sure the document_type_value is a map
        let Some(document_type_value_map) = document_type_value.as_map() else {
            return Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                "document type data is not a map as expected",
            )));
        };

        let document_type = DocumentType::from_platform_value(
            self.id,
            &doc_type,
            document_type_value_map,
            &BTreeMap::new(),
            self.config.documents_keep_history_contract_default,
            self.config.documents_mutable_contract_default,
        )?;

        self.document_types.insert(doc_type, document_type);

        Ok(())
    }

    pub fn get_document_schema(&self, doc_type: &str) -> Result<&JsonSchema, ProtocolError> {
        let document = self
            .documents
            .get(doc_type)
            .ok_or(ProtocolError::DataContractError(
                DataContractError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(
                    doc_type.to_owned(),
                    self.id,
                )),
            ))?;
        Ok(document)
    }

    pub fn get_document_schema_ref(&self, doc_type: &str) -> Result<String, ProtocolError> {
        if !self.is_document_defined(doc_type) {
            return Err(ProtocolError::DataContractError(
                DataContractError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(
                    doc_type.to_owned(),
                    self.id,
                )),
            ));
        };

        Ok(format!(
            "{}#/documents/{}",
            self.id.to_string(Encoding::Base58),
            doc_type
        ))
    }

    /// Returns the binary properties for the given document type
    /// Comparing to JS version of DPP, the binary_properties are not generated automatically
    /// if they're not present. It is up to the developer to use proper methods like ['DataContractV0::set_document_schema'] which
    /// automatically generates binary properties when setting the Json Schema
    pub fn get_binary_properties(
        &self,
        doc_type: &str,
    ) -> Result<&BTreeMap<String, JsonValue>, ProtocolError> {
        self.get_optional_binary_properties(doc_type)?
            .ok_or(ProtocolError::DataContractError(
                DataContractError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(
                    doc_type.to_owned(),
                    self.id,
                )),
            ))
    }

    /// Returns the binary properties for the given document type
    /// Comparing to JS version of DPP, the binary_properties are not generated automatically
    /// if they're not present. It is up to the developer to use proper methods like ['DataContractV0::set_document_schema'] which
    /// automatically generates binary properties when setting the Json Schema
    // TODO: Naming is confusing. It's not clear, it sounds like it will return optional document properties
    //   but not None if document type is not present. Rename this
    pub fn get_optional_binary_properties(
        &self,
        doc_type: &str,
    ) -> Result<Option<&BTreeMap<String, JsonValue>>, ProtocolError> {
        if !self.is_document_defined(doc_type) {
            return Ok(None);
        }

        // The rust implementation doesn't set the value if it is not present in `binary_properties`. The difference is caused by
        // required `mut` annotation. As `get_binary_properties` is reused in many other read-only methods, the mutation would require
        // propagating the `mut` to other getters which by the definition shouldn't be mutable.
        self.binary_properties
            .get(doc_type)
            .ok_or_else(|| {
                {
                    anyhow::anyhow!(
                        "document '{}' has not generated binary_properties",
                        doc_type
                    )
                }
                .into()
            })
            .map(Some)
    }

    pub(crate) fn generate_binary_properties(&mut self) {
        self.binary_properties = self
            .documents
            .iter()
            .map(|(doc_type, schema)| (String::from(doc_type), get_binary_properties(schema)))
            .collect();
    }

    pub fn get_identifiers_and_binary_paths(
        &self,
        document_type: &str,
    ) -> Result<(HashSet<&str>, HashSet<&str>), ProtocolError> {
        let binary_properties = self.get_optional_binary_properties(document_type)?;

        // At this point we don't bother about returned error from `get_binary_properties`.
        // If document of given type isn't found, then empty vectors will be returned.
        let (binary_paths, identifiers_paths) = match binary_properties {
            None => (HashSet::new(), HashSet::new()),
            Some(binary_properties) => binary_properties.iter().partition_map(|(path, v)| {
                if let Some(JsonValue::String(content_type)) = v.get("contentMediaType") {
                    if content_type == identifier::MEDIA_TYPE {
                        Either::Right(path.as_str())
                    } else {
                        Either::Left(path.as_str())
                    }
                } else {
                    Either::Left(path.as_str())
                }
            }),
        };
        Ok((identifiers_paths, binary_paths))
    }

    pub fn get_identifiers_and_binary_paths_owned<
        I: IntoIterator<Item = String> + Extend<String> + Default,
    >(
        &self,
        document_type: &str,
    ) -> Result<(I, I), ProtocolError> {
        let binary_properties = self.get_optional_binary_properties(document_type)?;

        // At this point we don't bother about returned error from `get_binary_properties`.
        // If document of given type isn't found, then empty vectors will be returned.
        Ok(binary_properties
            .map(|binary_properties| {
                binary_properties.iter().partition_map(|(path, v)| {
                    if let Some(JsonValue::String(content_type)) = v.get("contentMediaType") {
                        if content_type == platform_value::IDENTIFIER_MEDIA_TYPE {
                            Either::Left(path.clone())
                        } else {
                            Either::Right(path.clone())
                        }
                    } else {
                        Either::Right(path.clone())
                    }
                })
            })
            .unwrap_or_default())
    }

    pub fn optional_document_type_for_name(
        &self,
        document_type_name: &str,
    ) -> Option<&DocumentType> {
        self.document_types.get(document_type_name)
    }

    pub fn document_type_for_name(
        &self,
        document_type_name: &str,
    ) -> Result<&DocumentType, ProtocolError> {
        self.document_types.get(document_type_name).ok_or({
            ProtocolError::DataContractError(DataContractError::DocumentTypeNotFound(
                "can not get document type from contract",
            ))
        })
    }

    pub fn has_document_type_for_name(&self, document_type_name: &str) -> bool {
        self.document_types.get(document_type_name).is_some()
    }
}

#[cfg(feature = "json-object")]
impl TryFrom<JsonValue> for DataContractV0 {
    type Error = ProtocolError;
    fn try_from(v: JsonValue) -> Result<Self, Self::Error> {
        DataContractV0::from_json_object(v)
    }
}

#[cfg(feature = "platform-value")]
impl TryFrom<Value> for DataContractV0 {
    type Error = ProtocolError;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        DataContractV0::from_raw_object(value)
    }
}

impl TryFrom<DataContractV0> for Value {
    type Error = ProtocolError;

    fn try_from(value: DataContractV0) -> Result<Self, Self::Error> {
        value.into_object()
    }
}

impl TryFrom<&DataContractV0> for Value {
    type Error = ProtocolError;

    fn try_from(value: &DataContractV0) -> Result<Self, Self::Error> {
        value.to_object()
    }
}

#[cfg(feature = "platform-value")]
impl TryFrom<&str> for DataContractV0 {
    type Error = ProtocolError;
    fn try_from(v: &str) -> Result<Self, Self::Error> {
        let data_contract: DataContractV0 = serde_json::from_str(v)?;
        //todo: there's a better to do this, find it
        let value = data_contract.to_object()?;
        DataContractV0::from_raw_object(value)
    }
}

impl<'a> TryFrom<&'a [u8]> for DataContractV0 {
    type Error = ProtocolError;

    fn try_from(_v: &[u8]) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<Vec<u8>> for DataContractV0 {
    type Error = ProtocolError;

    fn try_from(_v: Vec<u8>) -> Result<Self, Self::Error> {
        todo!()
    }
}

pub fn get_contract_configuration_properties(
    contract: &BTreeMap<String, Value>,
) -> Result<ContractConfigV0, ProtocolError> {
    let keeps_history = contract
        .get_optional_bool(contract_config::property::KEEPS_HISTORY)?
        .unwrap_or(DEFAULT_CONTRACT_KEEPS_HISTORY);
    let can_be_deleted = contract
        .get_optional_bool(contract_config::property::CAN_BE_DELETED)?
        .unwrap_or(DEFAULT_CONTRACT_CAN_BE_DELETED);

    let readonly = contract
        .get_optional_bool(contract_config::property::READONLY)?
        .unwrap_or(!DEFAULT_CONTRACT_MUTABILITY);

    let documents_keep_history_contract_default = contract
        .get_optional_bool(contract_config::property::DOCUMENTS_KEEP_HISTORY_CONTRACT_DEFAULT)?
        .unwrap_or(DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY);

    let documents_mutable_contract_default = contract
        .get_optional_bool(contract_config::property::DOCUMENTS_MUTABLE_CONTRACT_DEFAULT)?
        .unwrap_or(DEFAULT_CONTRACT_DOCUMENT_MUTABILITY);

    Ok(ContractConfigV0 {
        can_be_deleted,
        readonly,
        keeps_history,
        documents_keep_history_contract_default,
        documents_mutable_contract_default,
    })
}

pub fn get_document_types_from_value(
    data_contract_id: Identifier,
    documents_value: &Value,
    definition_references: &BTreeMap<String, &Value>,
    documents_keep_history_contract_default: bool,
    documents_mutable_contract_default: bool,
) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    let contract_document_types_raw =
        documents_value
            .as_map()
            .ok_or(ProtocolError::DataContractError(
                DataContractError::InvalidContractStructure("documents must be a map"),
            ))?.iter().map(|(key, value)| {
            let Some(type_key_str) = key.as_text() else {
                return Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                    "document type name is not a string as expected",
                )));
            };
            Ok((type_key_str, value))
        }).collect::<Result<Vec<(&str, &Value)>, ProtocolError>>()?;
    get_document_types_from_value_array(
        data_contract_id,
        &contract_document_types_raw,
        definition_references,
        documents_keep_history_contract_default,
        documents_mutable_contract_default,
    )
}

pub fn get_document_types_from_value_array(
    data_contract_id: Identifier,
    contract_document_types_raw: &Vec<(&str, &Value)>,
    definition_references: &BTreeMap<String, &Value>,
    documents_keep_history_contract_default: bool,
    documents_mutable_contract_default: bool,
) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    let mut contract_document_types: BTreeMap<String, DocumentType> = BTreeMap::new();
    for (type_key_str, document_type_value) in contract_document_types_raw {
        // Make sure the document_type_value is a map
        let Some(document_type_value_map) = document_type_value.as_map() else {
            return Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                "document type data is not a map as expected",
            )));
        };

        let document_type = DocumentType::from_platform_value(
            data_contract_id,
            type_key_str,
            document_type_value_map,
            definition_references,
            documents_keep_history_contract_default,
            documents_mutable_contract_default,
        )?;
        contract_document_types.insert(type_key_str.to_string(), document_type);
    }
    Ok(contract_document_types)
}

pub fn get_document_types_from_contract(
    data_contract_id: Identifier,
    contract: &BTreeMap<String, Value>,
    definition_references: &BTreeMap<String, &Value>,
    documents_keep_history_contract_default: bool,
    documents_mutable_contract_default: bool,
) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    let Some(documents_value) =
        contract
            .get("documents") else {
        return Ok(BTreeMap::new());
    };
    get_document_types_from_value(
        data_contract_id,
        documents_value,
        definition_references,
        documents_keep_history_contract_default,
        documents_mutable_contract_default,
    )
}

pub fn get_definitions(
    contract: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, &Value>, ProtocolError> {
    Ok(contract
        .get_optional_str_value_map("$defs")?
        .unwrap_or_default())
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use integer_encoding::VarInt;

    use crate::tests::{fixtures::get_data_contract_fixture, utils::*};

    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    #[test]
    #[cfg(feature = "cbor")]
    fn conversion_to_cbor_buffer_from_cbor_buffer() {
        init();
        let data_contract = get_data_contract_fixture(None).data_contract;

        let data_contract_bytes = data_contract
            .to_cbor_buffer()
            .expect("data contract should be converted into the bytes");
        let data_contract_restored = DataContractV0::from_cbor_buffer(data_contract_bytes)
            .expect("data contract should be created from bytes");

        assert_eq!(
            data_contract.data_contract_protocol_version,
            data_contract_restored.data_contract_protocol_version
        );
        assert_eq!(data_contract.schema, data_contract_restored.schema);
        assert_eq!(data_contract.version, data_contract_restored.version);
        assert_eq!(data_contract.id, data_contract_restored.id);
        assert_eq!(data_contract.owner_id, data_contract_restored.owner_id);
        assert_eq!(
            data_contract.binary_properties,
            data_contract_restored.binary_properties
        );
        assert_eq!(data_contract.documents, data_contract_restored.documents);
        assert_eq!(
            data_contract.document_types,
            data_contract_restored.document_types
        );
    }

    #[test]
    #[cfg(feature = "cbor")]
    fn conversion_to_cbor_buffer_from_cbor_buffer_high_version() {
        init();
        let mut data_contract = get_data_contract_fixture(None).data_contract;
        data_contract.data_contract_protocol_version = 10000;

        let data_contract_bytes = data_contract
            .to_cbor_buffer()
            .expect("data contract should be converted into the bytes");

        let data_contract_restored = DataContractV0::from_cbor_buffer(data_contract_bytes)
            .expect("data contract should be created from bytes");

        assert_eq!(
            data_contract.data_contract_protocol_version,
            data_contract_restored.data_contract_protocol_version
        );
        assert_eq!(data_contract.schema, data_contract_restored.schema);
        assert_eq!(data_contract.version, data_contract_restored.version);
        assert_eq!(data_contract.id, data_contract_restored.id);
        assert_eq!(data_contract.owner_id, data_contract_restored.owner_id);
        assert_eq!(
            data_contract.binary_properties,
            data_contract_restored.binary_properties
        );
        assert_eq!(data_contract.documents, data_contract_restored.documents);
        assert_eq!(
            data_contract.document_types,
            data_contract_restored.document_types
        );
    }

    #[test]
    fn conversion_to_cbor_buffer_from_cbor_buffer_too_high_version() {
        init();
        let data_contract = get_data_contract_fixture(None).data_contract;

        let data_contract_bytes = data_contract
            .to_cbor_buffer()
            .expect("data contract should be converted into the bytes");

        let mut high_protocol_version_bytes = u64::MAX.encode_var_vec();

        let (_, offset) = u32::decode_var(&data_contract_bytes)
            .ok_or(ProtocolError::DecodingError(
                "contract cbor could not decode protocol version".to_string(),
            ))
            .expect("expected to decode protocol version");
        let (_, contract_cbor_bytes) = data_contract_bytes.split_at(offset);

        high_protocol_version_bytes.extend_from_slice(contract_cbor_bytes);

        let data_contract_restored = DataContractV0::from_cbor_buffer(&high_protocol_version_bytes)
            .expect("data contract should be created from bytes");

        assert_eq!(
            u32::MAX,
            data_contract_restored.data_contract_protocol_version
        );
        assert_eq!(data_contract.schema, data_contract_restored.schema);
        assert_eq!(data_contract.version, data_contract_restored.version);
        assert_eq!(data_contract.id, data_contract_restored.id);
        assert_eq!(data_contract.owner_id, data_contract_restored.owner_id);
        assert_eq!(
            data_contract.binary_properties,
            data_contract_restored.binary_properties
        );
        assert_eq!(data_contract.documents, data_contract_restored.documents);
        assert_eq!(
            data_contract.document_types,
            data_contract_restored.document_types
        );
    }

    #[test]
    fn conversion_from_json() -> Result<()> {
        init();

        let string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;
        let contract = DataContractV0::try_from(string_contract.as_str())?;
        assert_eq!(contract.data_contract_protocol_version, 0);
        assert_eq!(
            contract.schema,
            "https://schema.dash.org/dpp-0-4-0/meta/data-contract"
        );
        assert_eq!(contract.version, 5);
        assert_eq!(
            contract.id.to_string(Encoding::Base58),
            "AoDzJxWSb1gUi2dSmvFeUFpSsjZQRJaqCpn7vCLkwwJj"
        );
        assert_eq!(
            contract.documents["note"]["properties"]["message"]["type"],
            "string"
        );
        assert!(contract.is_document_defined("note"));

        Ok(())
    }

    #[test]
    fn conversion_to_json() -> Result<()> {
        init();

        let mut string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;
        string_contract.retain(|c| !c.is_whitespace());

        let contract = DataContractV0::try_from(string_contract.as_str())?;
        let serialized_contract = serde_json::to_string(&contract.to_json()?)?;

        // they will be out of order so won't be exactly the same
        assert_eq!(serialized_contract, string_contract);
        Ok(())
    }

    #[test]
    fn conversion_to_object() -> Result<()> {
        let string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;
        let data_contract: DataContractV0 = serde_json::from_str(&string_contract)?;

        let raw_data_contract = data_contract.to_json_object()?;
        for path in DATA_CONTRACT_IDENTIFIER_FIELDS_V0 {
            assert!(raw_data_contract
                .get(path)
                .expect("the path should exist")
                .is_array())
        }
        Ok(())
    }

    #[test]
    fn conversion_from_object() -> Result<()> {
        init();

        let string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;
        let raw_contract: JsonValue = serde_json::from_str(&string_contract)?;

        for path in DATA_CONTRACT_IDENTIFIER_FIELDS_V0 {
            raw_contract.get(path).expect("the path should exist");
        }

        let data_contract_from_raw = DataContractV0::try_from(raw_contract)?;
        assert_eq!(data_contract_from_raw.data_contract_protocol_version, 0);
        assert_eq!(
            data_contract_from_raw.schema,
            "https://schema.dash.org/dpp-0-4-0/meta/data-contract"
        );
        assert_eq!(data_contract_from_raw.version, 5);
        assert_eq!(
            data_contract_from_raw.id.to_string(Encoding::Base58),
            "AoDzJxWSb1gUi2dSmvFeUFpSsjZQRJaqCpn7vCLkwwJj"
        );
        assert_eq!(
            data_contract_from_raw.documents["note"]["properties"]["message"]["type"],
            "string"
        );

        Ok(())
    }

    fn get_data_contract_cbor_bytes() -> Vec<u8> {
        let data_contract_cbor_hex = "01a56324696458208efef7338c0d34b2e408411b9473d724cbf9b675ca72b3126f7f8e7deb42ae516724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e657249645820962088aa3812bb3386d0c9130edbde51e4be17bb2d10031d4147c8597facee256776657273696f6e0169646f63756d656e7473a76b756e697175654461746573a56474797065666f626a65637467696e646963657382a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578326a70726f7065727469657381a16a2475706461746564417463617363687265717569726564836966697273744e616d656a246372656174656441746a247570646174656441746a70726f70657274696573a2686c6173744e616d65a1647479706566737472696e676966697273744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46c6e696365446f63756d656e74a46474797065666f626a656374687265717569726564816a246372656174656441746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e6e6f54696d65446f63756d656e74a36474797065666f626a6563746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e707265747479446f63756d656e74a46474797065666f626a65637468726571756972656482686c6173744e616d656a247570646174656441746a70726f70657274696573a1686c6173744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e7769746842797465417272617973a56474797065666f626a65637467696e646963657381a2646e616d6566696e646578316a70726f7065727469657381a16e6279746541727261794669656c6463617363687265717569726564816e6279746541727261794669656c646a70726f70657274696573a26e6279746541727261794669656c64a36474797065656172726179686d61784974656d731069627974654172726179f56f6964656e7469666965724669656c64a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572746164646974696f6e616c50726f70657274696573f46f696e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657386a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a1686c6173744e616d656464657363a2646e616d6566696e646578336a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6566696e646578346a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578356a70726f7065727469657381a16a2475706461746564417463617363a2646e616d6566696e646578366a70726f7065727469657381a16a2463726561746564417463617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e677468183f6966697273744e616d65a2647479706566737472696e67696d61784c656e677468183f746164646974696f6e616c50726f70657274696573f4781d6f7074696f6e616c556e69717565496e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657383a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657381a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657383a168246f776e6572496463617363a16966697273744e616d6563617363a1686c6173744e616d6563617363a3646e616d6566696e6465783366756e69717565f56a70726f7065727469657382a167636f756e74727963617363a1646369747963617363687265717569726564826966697273744e616d65686c6173744e616d656a70726f70657274696573a46463697479a2647479706566737472696e67696d61784c656e677468183f67636f756e747279a2647479706566737472696e67696d61784c656e677468183f686c6173744e616d65a2647479706566737472696e67696d61784c656e677468183f6966697273744e616d65a2647479706566737472696e67696d61784c656e677468183f746164646974696f6e616c50726f70657274696573f4";
        hex::decode(data_contract_cbor_hex).unwrap()
    }

    #[test]
    fn deserialize_dpp_cbor() {
        let data_contract_cbor = get_data_contract_cbor_bytes();

        let data_contract = DataContractV0::from_cbor_buffer(data_contract_cbor).unwrap();

        assert_eq!(data_contract.version, 1);
        assert_eq!(data_contract.data_contract_protocol_version, 1);
        assert_eq!(
            data_contract.schema,
            "https://schema.dash.org/dpp-0-4-0/meta/data-contract"
        );
        assert_eq!(
            data_contract.owner_id,
            Identifier::new([
                150, 32, 136, 170, 56, 18, 187, 51, 134, 208, 201, 19, 14, 219, 222, 81, 228, 190,
                23, 187, 45, 16, 3, 29, 65, 71, 200, 89, 127, 172, 238, 37
            ])
        );
        assert_eq!(
            data_contract.id,
            Identifier::new([
                142, 254, 247, 51, 140, 13, 52, 178, 228, 8, 65, 27, 148, 115, 215, 36, 203, 249,
                182, 117, 202, 114, 179, 18, 111, 127, 142, 125, 235, 66, 174, 81
            ])
        );
    }

    #[test]
    fn serialize_deterministically_serialize_to_cbor() {
        let data_contract_cbor = get_data_contract_cbor_bytes();

        let data_contract = DataContractV0::from_cbor_buffer(&data_contract_cbor).unwrap();

        let serialized = data_contract.to_cbor_buffer().unwrap();

        assert_eq!(hex::encode(data_contract_cbor), hex::encode(serialized));
    }

    #[test]
    fn serialize_deterministically_serialize_to_bincode() {
        let data_contract_cbor = get_data_contract_cbor_bytes();

        let data_contract = DataContractV0::from_cbor_buffer(&data_contract_cbor).unwrap();

        let serialized = data_contract.to_cbor_buffer().unwrap();

        assert_eq!(hex::encode(data_contract_cbor), hex::encode(serialized));
    }
}
