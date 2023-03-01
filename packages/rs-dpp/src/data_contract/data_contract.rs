use std::collections::BTreeMap;
use std::convert::TryFrom;

use anyhow::anyhow;
use ciborium::value::Value as CborValue;
use integer_encoding::VarInt;
use itertools::{Either, Itertools};
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::consensus::basic::document::InvalidDocumentTypeError;
use crate::data_contract::contract_config;
use crate::data_contract::contract_config::{
    ContractConfig, DEFAULT_CONTRACT_CAN_BE_DELETED, DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY,
    DEFAULT_CONTRACT_DOCUMENT_MUTABILITY, DEFAULT_CONTRACT_KEEPS_HISTORY,
    DEFAULT_CONTRACT_MUTABILITY,
};

use crate::data_contract::get_binary_properties_from_schema::get_binary_properties;
use crate::util::cbor_value::CborCanonicalMap;
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::util::json_value::{JsonValueExt, ReplaceWith};
use crate::util::string_encoding::Encoding;
use crate::{
    errors::ProtocolError,
    identifier::Identifier,
    metadata::Metadata,
    util::{hash::hash, serializer},
};
use crate::{identifier, Convertible};

use super::document_type::DocumentType;
use super::errors::*;

use super::property_names;

pub type JsonSchema = JsonValue;
type DefinitionName = String;
type DocumentName = String;
type PropertyPath = String;

pub const SCHEMA_URI: &str = "https://schema.dash.org/dpp-0-4-0/meta/data-contract";

pub const IDENTIFIER_FIELDS: [&str; 2] = [property_names::ID, property_names::OWNER_ID];
pub const BINARY_FIELDS: [&str; 1] = [property_names::ENTROPY];

impl Convertible for DataContract {
    fn to_object(&self) -> Result<JsonValue, ProtocolError> {
        let mut json_object = serde_json::to_value(self)?;
        if !json_object.is_object() {
            return Err(anyhow!("the Data Contract isn't a JSON Value Object").into());
        }

        json_object.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Bytes)?;
        Ok(json_object)
    }

    /// Returns Data Contract as a JSON Value
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        Ok(serde_json::to_value(self)?)
    }

    /// Returns Data Contract as a Buffer
    fn to_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let protocol_version = self.protocol_version;
        // what means skip_identifiers_conversion
        let mut json_object = self.to_object(true)?;

        if let JsonValue::Object(ref mut o) = json_object {
            o.remove("protocolVersion");
        };

        serializer::value_to_cbor(json_object, Some(protocol_version))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DataContract {
    pub protocol_version: u32,
    #[serde(rename = "$id")]
    pub id: Identifier,
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: u32,
    pub owner_id: Identifier,

    #[serde(rename = "documents")]
    pub documents: BTreeMap<DocumentName, JsonSchema>,

    // TODO we may ensure in compile time that defs are not empty if we define a type for it
    #[serde(skip_serializing_if = "Option::is_none", rename = "$defs", default)]
    pub defs: Option<BTreeMap<DefinitionName, JsonSchema>>,

    #[serde(skip)]
    pub entropy: [u8; 32],

    #[serde(skip)]
    pub metadata: Option<Metadata>,
    #[serde(skip)]
    pub binary_properties: BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>,

    #[serde(skip)]
    pub(crate) config: ContractConfig,

    #[serde(skip)]
    pub document_types: BTreeMap<DocumentName, DocumentType>,
}

impl DataContract {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_raw_object(mut raw_object: JsonValue) -> Result<DataContract, ProtocolError> {
        // TODO identifier_default_deserializer: default deserializer should be changed to bytes
        // Identifiers fields should be replaced with the string format to deserialize Data Contract
        raw_object.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Base58)?;
        let mut data_contract: DataContract = serde_json::from_value(raw_object)?;
        data_contract.generate_binary_properties();

        Ok(data_contract)
    }

    pub fn from_json_object(mut json_value: JsonValue) -> Result<DataContract, ProtocolError> {
        json_value.replace_binary_paths(BINARY_FIELDS, ReplaceWith::Bytes)?;

        let mut data_contract: DataContract = serde_json::from_value(json_value)?;
        data_contract.generate_binary_properties();

        Ok(data_contract)
    }

    pub fn from_buffer(b: impl AsRef<[u8]>) -> Result<DataContract, ProtocolError> {
        Self::from_cbor(b)
    }

    pub fn from_cbor(cbor_bytes: impl AsRef<[u8]>) -> Result<Self, ProtocolError> {
        let SplitProtocolVersionOutcome {
            protocol_version,
            protocol_version_size,
            main_message_bytes: contract_cbor_bytes,
        } = deserializer::split_protocol_version(cbor_bytes.as_ref())?;

        let data_contract_cbor_map: BTreeMap<String, CborValue> =
            ciborium::de::from_reader(contract_cbor_bytes).map_err(|_| {
                ProtocolError::DecodingError(format!(
                    "unable to decode contract with protocol version {} offset {}",
                    protocol_version, protocol_version_size
                ))
            })?;

        let data_contract_map: BTreeMap<String, Value> =
            Value::convert_from_cbor_map(data_contract_cbor_map);

        let contract_id: [u8; 32] = data_contract_map.get_identifier(property_names::ID)?;
        let owner_id: [u8; 32] = data_contract_map.get_identifier(property_names::OWNER_ID)?;
        let schema = data_contract_map.get_string(property_names::SCHEMA)?;
        let version = data_contract_map.get_integer(property_names::VERSION)?;

        // Defs
        let defs =
            data_contract_map.get_optional_inner_str_json_value_map::<BTreeMap<_, _>>("$defs")?;

        // Documents
        let documents: BTreeMap<String, JsonValue> = data_contract_map
            .get_inner_str_json_value_map("documents")
            .map_err(ProtocolError::ValueError)?;

        let mutability = get_contract_configuration_properties(&data_contract_map)
            .map_err(|e| ProtocolError::ParsingError(e.to_string()))?;
        let definition_references = get_definitions(&data_contract_map)?;
        let document_types = get_document_types(
            &data_contract_map,
            definition_references,
            mutability.documents_keep_history_contract_default,
            mutability.documents_mutable_contract_default,
        )
        .map_err(|e| ProtocolError::ParsingError(e.to_string()))?;

        let mut data_contract = Self {
            protocol_version,
            id: Identifier::new(contract_id),
            schema,
            version,
            owner_id: Identifier::new(owner_id),
            documents,
            defs,
            metadata: None,
            entropy: [0; 32],
            binary_properties: Default::default(),
            document_types,
            config: mutability,
        };

        data_contract.generate_binary_properties();

        Ok(data_contract)
    }

    pub fn to_object(&self, skip_identifiers_conversion: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_object = serde_json::to_value(self)?;
        if !json_object.is_object() {
            return Err(anyhow!("the Data Contract isn't a JSON Value Object").into());
        }

        if !skip_identifiers_conversion {
            json_object.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Bytes)?;
        }
        Ok(json_object)
    }

    /// Returns Data Contract as a JSON Value
    pub fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        Ok(serde_json::to_value(self)?)
    }

    /// Returns Data Contract as a Buffer
    pub fn to_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        self.to_cbor()
    }

    pub fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = self.protocol_version().encode_var_vec();

        let contract_cbor_map = self.to_cbor_canonical_map()?;
        let mut contract_buf = contract_cbor_map
            .to_bytes()
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        buf.append(&mut contract_buf);
        Ok(buf)
    }

    pub(crate) fn to_cbor_canonical_map(&self) -> Result<CborCanonicalMap, ProtocolError> {
        let mut contract_cbor_map = CborCanonicalMap::new();

        contract_cbor_map.insert(property_names::ID, self.id().to_buffer().to_vec());
        contract_cbor_map.insert(property_names::SCHEMA, self.schema());
        contract_cbor_map.insert(property_names::VERSION, self.version());
        contract_cbor_map.insert(
            property_names::OWNER_ID,
            self.owner_id().to_buffer().to_vec(),
        );

        let docs = CborValue::serialized(&self.documents)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        contract_cbor_map.insert(property_names::DOCUMENTS, docs);

        if let Some(defs) = &self.defs {
            contract_cbor_map.insert(
                property_names::DEFINITIONS,
                CborValue::serialized(defs)
                    .map_err(|e| ProtocolError::EncodingError(e.to_string()))?,
            );
        }

        Ok(contract_cbor_map)
    }

    pub fn documents(&self) -> &BTreeMap<DocumentName, JsonSchema> {
        &self.documents
    }

    pub fn entropy(&self) -> [u8; 32] {
        self.entropy
    }

    pub fn owner_id(&self) -> &Identifier {
        &self.owner_id
    }

    pub fn protocol_version(&self) -> u32 {
        self.protocol_version
    }

    pub fn id(&self) -> &Identifier {
        &self.id
    }

    pub fn schema(&self) -> &str {
        &self.schema
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn definitions(&self) -> Option<&BTreeMap<String, JsonValue>> {
        self.defs.as_ref()
    }

    // Returns hash from Data Contract
    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash(self.to_buffer()?))
    }

    /// Increments version of Data Contract
    pub fn increment_version(&mut self) {
        self.version += 1;
    }

    /// Returns true if document type is defined
    pub fn is_document_defined(&self, doc_type: &str) -> bool {
        self.documents.contains_key(doc_type)
    }

    pub fn set_document_schema(&mut self, doc_type: String, schema: JsonSchema) {
        let binary_properties = get_binary_properties(&schema);
        self.documents.insert(doc_type.clone(), schema);
        self.binary_properties.insert(doc_type, binary_properties);
    }

    pub fn get_document_schema(&self, doc_type: &str) -> Result<&JsonSchema, ProtocolError> {
        let document = self
            .documents
            .get(doc_type)
            .ok_or(ProtocolError::DataContractError(
                DataContractError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(
                    doc_type.to_owned(),
                    self.id.clone(),
                )),
            ))?;
        Ok(document)
    }

    pub fn get_document_schema_ref(&self, doc_type: &str) -> Result<String, ProtocolError> {
        if !self.is_document_defined(doc_type) {
            return Err(ProtocolError::DataContractError(
                DataContractError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(
                    doc_type.to_owned(),
                    self.id.clone(),
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
    /// if they're not present. It is up to the developer to use proper methods like ['DataContract::set_document_schema'] which
    /// automatically generates binary properties when setting the Json Schema
    pub fn get_binary_properties(
        &self,
        doc_type: &str,
    ) -> Result<&BTreeMap<String, JsonValue>, ProtocolError> {
        self.get_optional_binary_properties(doc_type)?
            .ok_or(ProtocolError::DataContractError(
                DataContractError::InvalidDocumentTypeError(InvalidDocumentTypeError::new(
                    doc_type.to_owned(),
                    self.id.clone(),
                )),
            ))
    }

    /// Returns the binary properties for the given document type
    /// Comparing to JS version of DPP, the binary_properties are not generated automatically
    /// if they're not present. It is up to the developer to use proper methods like ['DataContract::set_document_schema'] which
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

    fn generate_binary_properties(&mut self) {
        self.binary_properties = self
            .documents
            .iter()
            .map(|(doc_type, schema)| (String::from(doc_type), get_binary_properties(schema)))
            .collect();
    }

    pub fn get_identifiers_and_binary_paths(
        &self,
        document_type: &str,
    ) -> Result<(Vec<&str>, Vec<&str>), ProtocolError> {
        let binary_properties = self.get_optional_binary_properties(document_type)?;

        // At this point we don't bother about returned error from `get_binary_properties`.
        // If document of given type isn't found, then empty vectors will be returned.
        let (binary_paths, identifiers_paths) = match binary_properties {
            None => (vec![], vec![]),
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
}

impl TryFrom<JsonValue> for DataContract {
    type Error = ProtocolError;
    fn try_from(v: JsonValue) -> Result<Self, Self::Error> {
        let mut v = v;

        v.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Base58)?;

        let mut data_contract: Self = serde_json::from_value(v)?;
        data_contract.generate_binary_properties();

        Ok(data_contract)
    }
}

impl TryFrom<&str> for DataContract {
    type Error = ProtocolError;
    fn try_from(v: &str) -> Result<Self, Self::Error> {
        let mut data_contract: DataContract = serde_json::from_str(v)?;
        data_contract.generate_binary_properties();

        Ok(data_contract)
    }
}

impl<'a> TryFrom<&'a [u8]> for DataContract {
    type Error = ProtocolError;

    fn try_from(_v: &[u8]) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<Vec<u8>> for DataContract {
    type Error = ProtocolError;

    fn try_from(_v: Vec<u8>) -> Result<Self, Self::Error> {
        todo!()
    }
}

pub fn get_contract_configuration_properties(
    contract: &BTreeMap<String, Value>,
) -> Result<ContractConfig, ProtocolError> {
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

    Ok(ContractConfig {
        can_be_deleted,
        readonly,
        keeps_history,
        documents_keep_history_contract_default,
        documents_mutable_contract_default,
    })
}

pub fn get_document_types(
    contract: &BTreeMap<String, Value>,
    definition_references: BTreeMap<String, &Value>,
    documents_keep_history_contract_default: bool,
    documents_mutable_contract_default: bool,
) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    let documents_cbor_value =
        contract
            .get("documents")
            .ok_or(ProtocolError::DataContractError(
                DataContractError::MissingRequiredKey("unable to get documents"),
            ))?;
    let contract_document_types_raw =
        documents_cbor_value
            .as_map()
            .ok_or(ProtocolError::DataContractError(
                DataContractError::InvalidContractStructure("documents must be a map"),
            ))?;
    let mut contract_document_types: BTreeMap<String, DocumentType> = BTreeMap::new();
    for (type_key_value, document_type_value) in contract_document_types_raw {
        let Some(type_key_str) = type_key_value.as_text() else {
            return Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                "document type name is not a string as expected",
            )));
        };

        // Make sure the document_type_value is a map
        let Some(document_type_value_map) = document_type_value.as_map() else {
            return Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
                "document type data is not a map as expected",
            )));
        };

        let document_type = DocumentType::from_platform_value(
            type_key_str,
            document_type_value_map,
            &definition_references,
            documents_keep_history_contract_default,
            documents_mutable_contract_default,
        )?;
        contract_document_types.insert(type_key_str.to_string(), document_type);
    }
    Ok(contract_document_types)
}

pub fn get_definitions(
    contract: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, &Value>, ProtocolError> {
    Ok(contract
        .get("$defs")
        .map(|definition_value| {
            definition_value
                .as_map()
                .map(Value::map_ref_into_btree_map)
                .transpose()
        })
        .transpose()?
        .flatten()
        .unwrap_or_default())
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use crate::{
        assert_error_contains,
        tests::{fixtures::get_data_contract_fixture, utils::*},
    };

    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    #[test]
    fn conversion_to_buffer_from_buffer() {
        init();
        let data_contract = get_data_contract_fixture(None);

        let data_contract_bytes = data_contract
            .to_buffer()
            .expect("data contract should be converted into the bytes");
        let data_contract_restored = DataContract::from_buffer(data_contract_bytes)
            .expect("data contract should be created from bytes");

        assert_eq!(
            data_contract.protocol_version,
            data_contract_restored.protocol_version
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
    }

    #[test]
    fn conversion_to_buffer_from_buffer_high_version() {
        init();
        let mut data_contract = get_data_contract_fixture(None);
        data_contract.protocol_version = 10000;

        let data_contract_bytes = data_contract
            .to_buffer()
            .expect("data contract should be converted into the bytes");

        let data_contract_restored = DataContract::from_buffer(data_contract_bytes)
            .expect("data contract should be created from bytes");

        assert_eq!(
            data_contract.protocol_version,
            data_contract_restored.protocol_version
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
    }

    #[test]
    fn conversion_to_buffer_from_buffer_too_high_version() {
        init();
        let data_contract = get_data_contract_fixture(None);

        let data_contract_bytes = data_contract
            .to_buffer()
            .expect("data contract should be converted into the bytes");

        let mut high_protocol_version_bytes = u64::MAX.encode_var_vec();

        let (_, offset) = u32::decode_var(&data_contract_bytes)
            .ok_or(ProtocolError::DecodingError(
                "contract cbor could not decode protocol version".to_string(),
            ))
            .expect("expected to decode protocol version");
        let (_, contract_cbor_bytes) = data_contract_bytes.split_at(offset);

        high_protocol_version_bytes.extend_from_slice(contract_cbor_bytes);

        let data_contract_restored = DataContract::from_buffer(&high_protocol_version_bytes)
            .expect("data contract should be created from bytes");

        assert_eq!(u32::MAX, data_contract_restored.protocol_version);
        assert_eq!(data_contract.schema, data_contract_restored.schema);
        assert_eq!(data_contract.version, data_contract_restored.version);
        assert_eq!(data_contract.id, data_contract_restored.id);
        assert_eq!(data_contract.owner_id, data_contract_restored.owner_id);
        assert_eq!(
            data_contract.binary_properties,
            data_contract_restored.binary_properties
        );
        assert_eq!(data_contract.documents, data_contract_restored.documents);
    }

    #[test]
    fn conversion_from_json() -> Result<()> {
        init();

        let string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;
        let contract = DataContract::try_from(string_contract.as_str())?;

        assert_eq!(contract.protocol_version, 0);
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

        let contract = DataContract::try_from(string_contract.as_str())?;
        let serialized_contract = serde_json::to_string(&contract.to_json()?)?;

        assert_eq!(serialized_contract, string_contract);
        Ok(())
    }

    #[test]
    fn conversion_to_object() -> Result<()> {
        let string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;
        let data_contract: DataContract = serde_json::from_str(&string_contract)?;

        let raw_data_contract = data_contract.to_object(false)?;
        for path in IDENTIFIER_FIELDS {
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
        let mut raw_contract: JsonValue = serde_json::from_str(&string_contract)?;
        raw_contract.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Bytes)?;

        for path in IDENTIFIER_FIELDS {
            assert!(raw_contract
                .get(path)
                .expect("the path should exist")
                .is_array())
        }

        let data_contract_from_raw = DataContract::try_from(raw_contract)?;
        assert_eq!(data_contract_from_raw.protocol_version, 0);
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

    #[test]
    fn conversion_from_invalid_object() -> Result<()> {
        init();

        let string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;

        let invalid_raw_contract: JsonValue = serde_json::from_str(&string_contract)?;
        // The identifiers are strings but they should be arrays of bytes
        for path in IDENTIFIER_FIELDS {
            assert!(invalid_raw_contract
                .get(path)
                .expect("the path should exist")
                .is_string())
        }

        let result = DataContract::try_from(invalid_raw_contract);
        assert_error_contains!(result, "expected a sequence");
        Ok(())
    }

    fn get_data_contract_cbor_bytes() -> Vec<u8> {
        let data_contract_cbor_hex = "01a56324696458208efef7338c0d34b2e408411b9473d724cbf9b675ca72b3126f7f8e7deb42ae516724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e657249645820962088aa3812bb3386d0c9130edbde51e4be17bb2d10031d4147c8597facee256776657273696f6e0169646f63756d656e7473a76b756e697175654461746573a56474797065666f626a65637467696e646963657382a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578326a70726f7065727469657381a16a2475706461746564417463617363687265717569726564836966697273744e616d656a246372656174656441746a247570646174656441746a70726f70657274696573a2686c6173744e616d65a1647479706566737472696e676966697273744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46c6e696365446f63756d656e74a46474797065666f626a656374687265717569726564816a246372656174656441746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e6e6f54696d65446f63756d656e74a36474797065666f626a6563746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e707265747479446f63756d656e74a46474797065666f626a65637468726571756972656482686c6173744e616d656a247570646174656441746a70726f70657274696573a1686c6173744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e7769746842797465417272617973a56474797065666f626a65637467696e646963657381a2646e616d6566696e646578316a70726f7065727469657381a16e6279746541727261794669656c6463617363687265717569726564816e6279746541727261794669656c646a70726f70657274696573a26e6279746541727261794669656c64a36474797065656172726179686d61784974656d731069627974654172726179f56f6964656e7469666965724669656c64a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572746164646974696f6e616c50726f70657274696573f46f696e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657386a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a1686c6173744e616d656464657363a2646e616d6566696e646578336a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6566696e646578346a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578356a70726f7065727469657381a16a2475706461746564417463617363a2646e616d6566696e646578366a70726f7065727469657381a16a2463726561746564417463617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e677468183f6966697273744e616d65a2647479706566737472696e67696d61784c656e677468183f746164646974696f6e616c50726f70657274696573f4781d6f7074696f6e616c556e69717565496e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657383a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657381a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657383a168246f776e6572496463617363a16966697273744e616d6563617363a1686c6173744e616d6563617363a3646e616d6566696e6465783366756e69717565f56a70726f7065727469657382a167636f756e74727963617363a1646369747963617363687265717569726564826966697273744e616d65686c6173744e616d656a70726f70657274696573a46463697479a2647479706566737472696e67696d61784c656e677468183f67636f756e747279a2647479706566737472696e67696d61784c656e677468183f686c6173744e616d65a2647479706566737472696e67696d61784c656e677468183f6966697273744e616d65a2647479706566737472696e67696d61784c656e677468183f746164646974696f6e616c50726f70657274696573f4";
        hex::decode(data_contract_cbor_hex).unwrap()
    }

    #[test]
    fn deserialize_dpp_cbor() {
        let data_contract_cbor = get_data_contract_cbor_bytes();

        let data_contract = DataContract::from_buffer(data_contract_cbor).unwrap();

        assert_eq!(data_contract.version(), 1);
        assert_eq!(data_contract.protocol_version(), 1);
        assert_eq!(
            data_contract.schema(),
            "https://schema.dash.org/dpp-0-4-0/meta/data-contract"
        );
        assert_eq!(
            data_contract.owner_id(),
            &Identifier::new([
                150, 32, 136, 170, 56, 18, 187, 51, 134, 208, 201, 19, 14, 219, 222, 81, 228, 190,
                23, 187, 45, 16, 3, 29, 65, 71, 200, 89, 127, 172, 238, 37
            ])
        );
        assert_eq!(
            data_contract.id(),
            &Identifier::new([
                142, 254, 247, 51, 140, 13, 52, 178, 228, 8, 65, 27, 148, 115, 215, 36, 203, 249,
                182, 117, 202, 114, 179, 18, 111, 127, 142, 125, 235, 66, 174, 81
            ])
        );
    }

    #[test]
    fn serialize_deterministically_serialize_to_cbor() {
        let data_contract_cbor = get_data_contract_cbor_bytes();

        let data_contract = DataContract::from_buffer(&data_contract_cbor).unwrap();

        let serialized = data_contract.to_buffer().unwrap();

        assert_eq!(data_contract_cbor, serialized);
    }
}
