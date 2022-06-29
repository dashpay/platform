use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

use anyhow::anyhow;
use ciborium::value::Value as CborValue;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{
    errors::ProtocolError,
    identifier::Identifier,
    metadata::Metadata,
    util::{hash::hash, serializer},
};
use crate::common::bytes_for_system_value_from_tree_map;
use crate::data_contract::get_binary_properties_from_schema::get_binary_properties;
use crate::util::cbor_value::{cbor_value_to_json_value, CborCanonicalMap};
use crate::util::deserializer;
use crate::util::json_value::{JsonValueExt, ReplaceWith};
use crate::util::string_encoding::Encoding;

use super::errors::*;

pub type JsonSchema = JsonValue;
type DocumentType = String;
type PropertyPath = String;

pub const SCHEMA: &str = "https://schema.dash.org/dpp-0-4-0/meta/data-contract";

pub const IDENTIFIER_FIELDS: [&str; 2] = ["$id", "ownerId"];

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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
    pub documents: BTreeMap<DocumentType, JsonSchema>,
    #[serde(rename = "$defs", default)]
    pub defs: BTreeMap<DocumentType, JsonSchema>,
    #[serde(skip)]
    pub metadata: Option<Metadata>,
    #[serde(skip)]
    pub entropy: [u8; 32],
    #[serde(skip)]
    pub binary_properties: BTreeMap<DocumentType, BTreeMap<PropertyPath, JsonValue>>,
    #[serde(skip)]
    pub documents_cbor_internal: CborCanonicalMap,
    #[serde(skip)]
    pub defs_cbor_internal: CborCanonicalMap,
}

impl DataContract {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_buffer(b: impl AsRef<[u8]>) -> Result<DataContract, ProtocolError> {
        let (protocol_bytes, document_bytes) = b.as_ref().split_at(4);

        let mut json_value: JsonValue = ciborium::de::from_reader(document_bytes)
            .map_err(|e| ProtocolError::EncodingError(format!("{}", e)))?;

        json_value.parse_and_add_protocol_version(protocol_bytes)?;

        // Identifiers fields should be replaced with the string format to deserialize Data Contract
        json_value.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Base58)?;

        let mut data_contract: DataContract = serde_json::from_value(json_value)?;
        data_contract.generate_binary_properties();
        Ok(data_contract)
    }

    pub fn from_cbor(cbor_bytes: impl AsRef<[u8]>) -> Result<Self, ProtocolError> {
        let (protocol_version_bytes, contract_cbor_bytes) = cbor_bytes.as_ref().split_at(4);
        let protocol_version = deserializer::get_protocol_version(protocol_version_bytes)?;

        let data_contract_map: BTreeMap<String, CborValue> =
            ciborium::de::from_reader(contract_cbor_bytes).map_err(|_| {
                ProtocolError::DecodingError(String::from("unable to decode contract"))
            })?;

        let contract_id: [u8; 32] =
            bytes_for_system_value_from_tree_map(&data_contract_map, "$id")?
                .ok_or_else(|| {ProtocolError::DecodingError(String::from("unable to get contract id")) })?
                .try_into()
                .map_err(|_| {
                    ProtocolError::DecodingError(String::from("contract_id must be 32 bytes"))
                })?;

        let owner_id: [u8; 32] =
            bytes_for_system_value_from_tree_map(&data_contract_map, "ownerId")?
                .ok_or_else(|| {
                    ProtocolError::DecodingError(String::from("unable to get contract owner id"))
                })?
                .try_into()
                .map_err(|_| {
                    ProtocolError::DecodingError(String::from("ownerId must be 32 bytes"))
                })?;

        let schema = data_contract_map
            .get("$schema")
            .ok_or_else(|| {ProtocolError::DecodingError(String::from(
                "unable to get contract owner id",
            ))})?
            .as_text()
            .ok_or_else(|| {ProtocolError::DecodingError(String::from(
                "Decoding contract: expect $schema to be a string",
            ))})?;

        let version = i128::from(
            data_contract_map
                .get("$schema")
                .ok_or_else(|| {ProtocolError::DecodingError(String::from(
                    "unable to get contract owner id",
                ))})?
                .as_integer()
                .ok_or_else(|| {ProtocolError::DecodingError(String::from(
                    "Decoding contract: expect $schema to be a string",
                ))})?,
        ) as u32;

        // Defs
        let (defs, defs_cbor) = match data_contract_map.get("$defs") {
            None => (BTreeMap::new(), CborCanonicalMap::new()),
            Some(definition_value) => {
                let definition_map = definition_value.as_map();
                match definition_map {
                    None => (BTreeMap::new(), CborCanonicalMap::new()),
                    Some(cbor_map) => {
                        let mut res = BTreeMap::<String, JsonValue>::new();
                        for (key, value) in cbor_map {
                            let key_string =
                                key.as_text()
                                    .ok_or_else(|| {ProtocolError::DecodingError(String::from(
                                        "Expect $defs keys to be strings",
                                    ))})?;
                            let json_value = cbor_value_to_json_value(value)?;
                            res.insert(String::from(key_string), json_value);
                        }
                        (res, cbor_map.into())
                    }
                }
            }
        };

        // Documents
        let documents_cbor_value = data_contract_map
            .get("documents")
            .ok_or_else(|| {ProtocolError::DecodingError(String::from("unable to get documents"))} )?;
        let contract_documents_cbor_map = documents_cbor_value
            .as_map()
            .ok_or_else(||{ ProtocolError::DecodingError(String::from("documents must be a map")) })?;
        let documents_vec = contract_documents_cbor_map
            .iter()
            .map(|(key, value)| {
                Ok((
                    key.as_text()
                        .ok_or_else(|| {ProtocolError::DecodingError(String::from(
                            "expect document type to be a string",
                        ))})?
                        .to_string(),
                    cbor_value_to_json_value(value)?,
                ))
            })
            .collect::<Result<Vec<(String, JsonValue)>, ProtocolError>>()?;

        let mut documents: BTreeMap<String, JsonValue> = BTreeMap::new();

        for (key, value) in documents_vec {
            documents.insert(key, value);
        }

        // let mut contract_documents: BTreeMap<String, DocumentType> = BTreeMap::new();
        //
        // // Build the document type hashmap
        // for (type_key_value, document_type_value) in contract_documents_cbor_map {
        //     if !type_key_value.is_text() {
        //         return Err(ProtocolError::EncodingError(String::from(
        //             "document type name is not a string as expected",
        //         )));
        //     }
        //
        //     // Make sure the document_type_value is a map
        //     if !document_type_value.is_map() {
        //         return Err(ProtocolError::EncodingError(String::from(
        //             "document type data is not a map as expected",
        //         )));
        //     }
        //
        //     let document_type = DocumentType::from_cbor_value(
        //         type_key_value.as_text().expect("confirmed as text"),
        //         document_type_value.as_map().expect("confirmed as map"),
        //         &definition_references,
        //         documents_keep_history_contract_default,
        //         documents_mutable_contract_default,
        //     )?;
        //     contract_documents.insert(
        //         String::from(type_key_value.as_text().expect("confirmed as text")),
        //         document_type,
        //     );
        // }

        Ok(Self {
            protocol_version,
            id: Identifier::new(contract_id),
            schema: schema.to_string(),
            version,
            owner_id: Identifier::new(owner_id),
            documents,
            defs,
            metadata: None,
            entropy: [0; 32],
            binary_properties: Default::default(),
            documents_cbor_internal: contract_documents_cbor_map.into(),
            defs_cbor_internal: defs_cbor,
        })
    }

    pub fn to_object(&self, skip_identifiers_conversion: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_object = serde_json::to_value(&self)?;
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
        Ok(serde_json::to_value(&self)?)
    }

    /// Returns Data Contract as a Buffer
    pub fn to_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let protocol_version = self.protocol_version;
        let mut json_object = self.to_object(true)?;

        if let JsonValue::Object(ref mut o) = json_object {
            o.remove("protocolVersion");
        };

        serializer::value_to_cbor(json_object, Some(protocol_version))
    }

    pub fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut contract_cbor_map = CborCanonicalMap::new();

        // pub protocol_version: u32,
        // #[serde(rename = "$id")]
        // pub id: Identifier,
        // #[serde(rename = "$schema")]
        // pub schema: String,
        // pub version: u32,
        // pub owner_id: Identifier,
        // #[serde(rename = "documents")]
        // pub documents: BTreeMap<DocumentType, JsonSchema>,
        // #[serde(rename = "$defs", default)]
        // pub defs: BTreeMap<DocumentType, JsonSchema>,
        // #[serde(skip)]
        // pub metadata: Option<Metadata>,
        // #[serde(skip)]
        // pub entropy: [u8; 32],
        // #[serde(skip)]
        // pub binary_properties: BTreeMap<DocumentType, BTreeMap<PropertyPath, JsonValue>>,

        contract_cbor_map.insert("protocolVersion", self.protocol_version());
        contract_cbor_map.insert("$id", self.id().to_buffer().to_vec());
        contract_cbor_map.insert("$schema", self.schema());
        contract_cbor_map.insert("version", self.version());
        contract_cbor_map.insert("ownerId", self.owner_id().to_buffer().to_vec());
        //contract_cbor_map.insert("documents", self.owner_id());
        //contract_cbor_map.insert("$defs", self.defs());

        contract_cbor_map
            .to_bytes()
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))
    }

    pub fn documents(&self) -> &BTreeMap<DocumentType, JsonSchema> {
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
        let d =
            self.documents
                .get(doc_type)
                .ok_or(DataContractError::InvalidDocumentTypeError {
                    doc_type: doc_type.to_owned(),
                    data_contract: self.clone(),
                })?;
        Ok(d)
    }

    pub fn get_document_schema_ref(&self, doc_type: &str) -> Result<String, ProtocolError> {
        if !self.is_document_defined(doc_type) {
            return Err(DataContractError::InvalidDocumentTypeError {
                doc_type: doc_type.to_owned(),
                data_contract: self.clone(),
            }
            .into());
        };

        return Ok(format!(
            "{}/#documents/{}",
            self.id.to_string(Encoding::Base58),
            doc_type
        ));
    }

    /// Returns the binary properties for the given document type
    /// Comparing to JS version of DPP, the binary_properties are not generated automatically
    /// if they're not present. It is up to the developer to use proper methods like ['DataContract::set_document_schema'] which
    /// automatically generates binary properties when setting the Json Schema
    pub fn get_binary_properties(
        &self,
        doc_type: &str,
    ) -> Result<&BTreeMap<String, JsonValue>, ProtocolError> {
        if !self.is_document_defined(doc_type) {
            return Err(DataContractError::InvalidDocumentTypeError {
                doc_type: doc_type.to_owned(),
                data_contract: self.clone(),
            }
            .into());
        }

        // The rust implementation doesn't set the value if it is not present in `binary_properties`. The difference is caused by
        // required `mut` annotation. As `get_binary_properties` is reused in many other read-only methods, the mutation would require
        // propagating the `mut` to other getters which by the definition shouldn't be mutable.
        self.binary_properties.get(doc_type).ok_or_else(|| {
            {
                anyhow::anyhow!(
                    "document '{}' has not generated binary_properties",
                    doc_type
                )
            }
            .into()
        })
    }

    fn generate_binary_properties(&mut self) {
        self.binary_properties = self
            .documents
            .iter()
            .map(|(doc_type, schema)| (String::from(doc_type), get_binary_properties(schema)))
            .collect();
    }
}

impl TryFrom<JsonValue> for DataContract {
    type Error = ProtocolError;
    fn try_from(v: JsonValue) -> Result<Self, Self::Error> {
        let mut v = v;
        // TODO add binary_properties regeneration
        v.replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Base58)?;
        Ok(serde_json::from_value(v)?)
    }
}

impl TryFrom<&str> for DataContract {
    type Error = ProtocolError;
    fn try_from(v: &str) -> Result<Self, Self::Error> {
        Ok(serde_json::from_str(v)?)
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use crate::{assert_error_contains, tests::utils::*};

    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Trace)
            .try_init();
    }

    #[test]
    fn test_deserialize_contract_from_string() -> Result<()> {
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
    fn test_serialize_contract_json() -> Result<()> {
        init();

        let mut string_contract = get_data_from_file("src/tests/payloads/contract_example.json")?;
        string_contract.retain(|c| !c.is_whitespace());

        let contract = DataContract::try_from(string_contract.as_str())?;
        let serialized_contract = serde_json::to_string(&contract.to_json()?)?;

        assert_eq!(serialized_contract, string_contract);
        Ok(())
    }

    #[test]
    fn test_serialize_to_object() -> Result<()> {
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
    fn test_deserialize_contract_from_raw() -> Result<()> {
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
    fn deserialize_contract_from_invalid_raw_object() -> Result<()> {
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
        let data_contract_cbor_hex = "01000000a5632469645820a5ff56541066c322ef47b1250f2d2e8bb20efec9183c884318e5008a20aa8f6a6724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458202b903793a2118e7a00c982dc9c7bba225bb60b27fec306af4e818d72aa6ff7d66776657273696f6e0169646f63756d656e7473a76b756e697175654461746573a56474797065666f626a65637467696e646963657382a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578326a70726f7065727469657381a16a2475706461746564417463617363687265717569726564836966697273744e616d656a246372656174656441746a247570646174656441746a70726f70657274696573a2686c6173744e616d65a1647479706566737472696e676966697273744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46c6e696365446f63756d656e74a46474797065666f626a656374687265717569726564816a246372656174656441746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e6e6f54696d65446f63756d656e74a36474797065666f626a6563746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e707265747479446f63756d656e74a46474797065666f626a65637468726571756972656482686c6173744e616d656a247570646174656441746a70726f70657274696573a1686c6173744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e7769746842797465417272617973a56474797065666f626a65637467696e646963657381a2646e616d6566696e646578316a70726f7065727469657381a16e6279746541727261794669656c6463617363687265717569726564816e6279746541727261794669656c646a70726f70657274696573a26e6279746541727261794669656c64a36474797065656172726179686d61784974656d731069627974654172726179f56f6964656e7469666965724669656c64a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572746164646974696f6e616c50726f70657274696573f46f696e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657386a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a1686c6173744e616d656464657363a2646e616d6566696e646578336a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6566696e646578346a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578356a70726f7065727469657381a16a2475706461746564417463617363a2646e616d6566696e646578366a70726f7065727469657381a16a2463726561746564417463617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e677468183f6966697273744e616d65a2647479706566737472696e67696d61784c656e677468183f746164646974696f6e616c50726f70657274696573f4781d6f7074696f6e616c556e69717565496e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657383a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657381a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657383a168246f776e6572496463617363a16966697273744e616d6563617363a1686c6173744e616d6563617363a3646e616d6566696e6465783366756e69717565f56a70726f7065727469657382a167636f756e74727963617363a1646369747963617363687265717569726564826966697273744e616d65686c6173744e616d656a70726f70657274696573a46463697479a2647479706566737472696e67696d61784c656e677468183f67636f756e747279a2647479706566737472696e67696d61784c656e677468183f686c6173744e616d65a2647479706566737472696e67696d61784c656e677468183f6966697273744e616d65a2647479706566737472696e67696d61784c656e677468183f746164646974696f6e616c50726f70657274696573f4";
        hex::decode(data_contract_cbor_hex).unwrap()
    }

    #[test]
    fn deserialize_dpp_cbor() {
        let data_contract_cbor = get_data_contract_cbor_bytes();

        let data_contract = DataContract::from_buffer(&data_contract_cbor).unwrap();

        assert_eq!(
            data_contract.id(),
            &Identifier::new([
                195, 171, 133, 147, 50, 165, 14, 164, 10, 2, 128, 8, 137, 107, 124, 111, 169, 72,
                123, 92, 205, 178, 244, 141, 135, 239, 122, 157, 156, 130, 43, 174
            ])
        );
        assert_eq!(
            data_contract.owner_id(),
            &Identifier::new([
                5, 69, 119, 146, 84, 158, 71, 13, 38, 144, 200, 68, 41, 13, 108, 119, 138, 80, 84,
                132, 107, 58, 183, 159, 166, 34, 220, 70, 170, 108, 121, 35
            ])
        );
        assert_eq!(data_contract.version(), 1);
        assert_eq!(
            data_contract.schema(),
            "https://schema.dash.org/dpp-0-4-0/meta/data-contract"
        );
        //assert_eq!(data_contract.documents())
        // assert_eq!(data_contract.entropy(), [
        //     58, 14, 223, 103, 195, 135, 118, 133,
        //     248, 34, 203,  30, 182, 138, 186, 119,
        //     26, 79, 182,  90,  79, 129, 103, 143,
        //     22, 21, 122,  55, 128,  73, 195,  59
        // ])
    }

    #[test]
    fn serialize_deterministically() {}
}
