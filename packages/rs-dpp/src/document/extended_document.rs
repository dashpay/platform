use crate::data_contract::DataContract;
use crate::metadata::Metadata;
use crate::prelude::Identifier;
use crate::prelude::{Revision, TimestampMillis};
#[cfg(feature = "cbor")]
use crate::util::cbor_value::CborCanonicalMap;
#[cfg(feature = "cbor")]
use ciborium::Value as CborValue;
use crate::util::deserializer;
use crate::util::deserializer::{ProtocolVersion, SplitProtocolVersionOutcome};
use crate::util::hash::hash_to_vec;
use crate::ProtocolError;
use integer_encoding::VarInt;

use crate::data_contract::document_type::document_type::PROTOCOL_VERSION;
use crate::data_contract::document_type::DocumentType;
use crate::document::Document;
use platform_value::btreemap_extensions::BTreeValueMapInsertionPathHelper;
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::btreemap_extensions::BTreeValueMapReplacementPathHelper;
use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use platform_value::converter::serde_json::BTreeValueJsonConverter;
use platform_value::{Bytes32, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use std::collections::{BTreeMap, HashSet};
use std::convert::TryInto;

pub mod property_names {
    pub const PROTOCOL_VERSION: &str = "$protocolVersion";
    pub const ID: &str = "$id";
    pub const DOCUMENT_TYPE: &str = "$type";
    pub const REVISION: &str = "$revision";
    pub const DATA_CONTRACT_ID: &str = "$dataContractId";
    pub const OWNER_ID: &str = "$ownerId";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
}

pub const IDENTIFIER_FIELDS: [&str; 3] = [
    property_names::ID,
    property_names::DATA_CONTRACT_ID,
    property_names::OWNER_ID,
];

/// The document object represents the data provided by the platform in response to a query.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ExtendedDocument {
    #[serde(rename = "$protocolVersion")]
    pub protocol_version: u32,
    #[serde(rename = "$type")]
    pub document_type_name: String,
    #[serde(rename = "$dataContractId")]
    pub data_contract_id: Identifier,
    #[serde(flatten)]
    pub document: Document,
    #[serde(skip)]
    pub data_contract: DataContract,
    #[serde(skip)]
    pub metadata: Option<Metadata>,
    #[serde(skip)]
    //todo: make entropy optional
    pub entropy: Bytes32,
}

impl ExtendedDocument {
    fn properties_as_json_data(&self) -> Result<JsonValue, ProtocolError> {
        self.document
            .properties
            .to_json_value()
            .map_err(ProtocolError::ValueError)
    }

    pub fn get_optional_value(&self, key: &str) -> Option<&Value> {
        self.document.properties.get(key)
    }

    pub fn properties(&self) -> &BTreeMap<String, Value> {
        &self.document.properties
    }

    pub fn properties_as_mut(&mut self) -> &mut BTreeMap<String, Value> {
        &mut self.document.properties
    }

    pub fn id(&self) -> Identifier {
        self.document.id
    }

    pub fn owner_id(&self) -> Identifier {
        self.document.owner_id
    }

    pub fn document_type(&self) -> Result<&DocumentType, ProtocolError> {
        // We can unwrap because the Document can not be created without a valid Document Type
        self.data_contract
            .document_type_for_name(self.document_type_name.as_str())
    }

    pub fn can_be_modified(&self) -> Result<bool, ProtocolError> {
        self.document_type()
            .map(|document_type| document_type.documents_mutable)
    }

    pub fn needs_revision(&self) -> Result<bool, ProtocolError> {
        self.document_type()
            .map(|document_type| document_type.documents_mutable)
    }

    pub fn revision(&self) -> Option<&Revision> {
        self.document.revision.as_ref()
    }

    pub fn created_at(&self) -> Option<&TimestampMillis> {
        self.document.created_at.as_ref()
    }

    pub fn updated_at(&self) -> Option<&TimestampMillis> {
        self.document.updated_at.as_ref()
    }

    pub fn from_document_with_additional_info(
        document: Document,
        data_contract: DataContract,
        document_type_name: String,
        protocol_version: ProtocolVersion,
    ) -> Self {
        Self {
            protocol_version,
            document_type_name,
            data_contract_id: data_contract.id,
            document,
            data_contract,
            metadata: None,
            entropy: Default::default(),
        }
    }

    pub fn from_json_string(string: &str, contract: DataContract) -> Result<Self, ProtocolError> {
        let json_value: JsonValue = serde_json::from_str(string).map_err(|_| {
            ProtocolError::StringDecodeError("error decoding from json string".to_string())
        })?;
        Self::from_untrusted_platform_value(json_value.into(), contract)
    }

    pub fn from_raw_json_document(
        raw_document: JsonValue,
        data_contract: DataContract,
    ) -> Result<Self, ProtocolError> {
        Self::from_untrusted_platform_value(raw_document.into(), data_contract)
    }

    /// Create an extended document from a platform value object where fields are already in the
    /// proper format for the contract
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
        extended_document.protocol_version = properties
            .remove_optional_integer(property_names::PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or(PROTOCOL_VERSION);
        extended_document.data_contract_id = Identifier::new(
            properties
                .remove_optional_hash256_bytes(property_names::DATA_CONTRACT_ID)?
                .unwrap_or(extended_document.data_contract.id.to_buffer()),
        );
        extended_document.document = Document::from_map(properties, None, None)?;
        Ok(extended_document)
    }

    /// Create an extended document from a platform value object where fields might not be in the
    /// proper format for the contract
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
        extended_document.protocol_version = properties
            .remove_optional_integer(property_names::PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?
            .unwrap_or(PROTOCOL_VERSION);
        extended_document.data_contract_id = properties
            .remove_optional_identifier(property_names::DATA_CONTRACT_ID)?
            .unwrap_or(extended_document.data_contract.id);
        extended_document.document = Document::from_map(properties, None, None)?;

        extended_document
            .document
            .properties
            .replace_at_paths(identifiers, ReplacementType::Identifier)?;
        extended_document
            .document
            .properties
            .replace_at_paths(binary_paths, ReplacementType::BinaryBytes)?;
        Ok(extended_document)
    }

    pub fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let mut value = self.document.to_json()?;
        let value_mut = value.as_object_mut().unwrap();
        value_mut.insert(
            property_names::PROTOCOL_VERSION.to_string(),
            JsonValue::Number(self.protocol_version.into()),
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

    pub fn to_pretty_json(&self) -> Result<JsonValue, ProtocolError> {
        let mut value = self.document.to_json()?;
        let value_mut = value.as_object_mut().unwrap();
        value_mut.insert(
            property_names::PROTOCOL_VERSION.to_string(),
            JsonValue::Number(self.protocol_version.into()),
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
        } = deserializer::split_protocol_version(cbor_bytes.as_ref())?;

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
        Ok(ExtendedDocument {
            protocol_version,
            document_type_name,
            data_contract_id,
            document,
            ..Default::default()
        })
    }

    pub fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        let mut object = self.document.to_map_value()?;
        object.insert(
            property_names::PROTOCOL_VERSION.to_string(),
            Value::U32(self.protocol_version),
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
        let ExtendedDocument {
            protocol_version,
            document_type_name,
            data_contract_id,
            document,
            ..
        } = self;

        let mut object = document.into_map_value()?;
        object.insert(
            property_names::PROTOCOL_VERSION.to_string(),
            Value::U32(protocol_version),
        );
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

    pub fn to_json_object_for_validation(&self) -> Result<JsonValue, ProtocolError> {
        self.to_value()?
            .try_into_validating_json()
            .map_err(ProtocolError::ValueError)
    }

    pub fn to_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut result_buf = self.protocol_version.encode_var_vec();

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

    pub fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash_to_vec(self.to_buffer()?))
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

        identifiers_paths.extend(IDENTIFIER_FIELDS);

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

        identifiers_paths.extend(IDENTIFIER_FIELDS.map(|str| str.to_string()));

        Ok((identifiers_paths, binary_paths))
    }
}

impl TryInto<Value> for ExtendedDocument {
    type Error = ProtocolError;

    fn try_into(self) -> Result<Value, Self::Error> {
        self.into_value()
    }
}

impl TryInto<Value> for &ExtendedDocument {
    type Error = ProtocolError;

    fn try_into(self) -> Result<Value, Self::Error> {
        self.to_value()
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serde_json::{json, Value as JsonValue};
    use std::convert::TryInto;

    use crate::document::extended_document::{ExtendedDocument, IDENTIFIER_FIELDS};

    use crate::data_contract::DataContract;
    use crate::document::Document;
    use crate::prelude::Identifier;
    use crate::system_data_contracts::load_system_data_contract;
    use crate::tests::utils::*;
    use data_contracts::SystemDataContract;
    use platform_value::btreemap_extensions::BTreeValueMapHelper;
    use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
    use platform_value::string_encoding::Encoding;
    use platform_value::Value;
    use pretty_assertions::assert_eq;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }
    pub(crate) fn data_contract_with_dynamic_properties() -> DataContract {
        // The following is equivalent to the data contract
        // {
        //     "protocolVersion" :0,
        //     "$id" : vec![0_u8;32],
        //     "$schema" : "schema",
        //     "version" : 0,
        //     "ownerId" : vec![0_u8;32],
        //     "documents" : {
        //         "test" : {
        //             "properties" : {
        //                 "alphaIdentifier" :  {
        //                     "type": "array",
        //                     "byteArray": true,
        //                     "contentMediaType": "application/x.dash.dpp.identifier",
        //                 },
        //                 "alphaBinary" :  {
        //                     "type": "array",
        //                     "byteArray": true,
        //                 }
        //             }
        //         }
        //     }
        // }
        let test_document_properties_alpha_identifier = Value::from([
            ("type", Value::Text("array".to_string())),
            ("byteArray", Value::Bool(true)),
            (
                "contentMediaType",
                Value::Text("application/x.dash.dpp.identifier".to_string()),
            ),
        ]);
        let test_document_properties_alpha_binary = Value::from([
            ("type", Value::Text("array".to_string())),
            ("byteArray", Value::Bool(true)),
        ]);
        let test_document_properties = Value::from([
            ("alphaIdentifier", test_document_properties_alpha_identifier),
            ("alphaBinary", test_document_properties_alpha_binary),
        ]);
        let test_document = Value::from([("properties", test_document_properties)]);
        let documents = Value::from([("test", test_document)]);
        Value::from([
            ("protocolVersion", Value::U32(1)),
            ("$id", Value::Identifier([0_u8; 32])),
            ("$schema", Value::Text("schema".to_string())),
            ("version", Value::U32(0)),
            ("ownerId", Value::Identifier([0_u8; 32])),
            ("documents", documents),
        ])
        .try_into()
        .unwrap()
    }

    #[test]
    fn test_document_deserialize() -> Result<()> {
        init();
        let dpns_contract = load_system_data_contract(SystemDataContract::DPNS)?;
        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json")?;
        let doc = ExtendedDocument::from_json_string(&document_json, dpns_contract)?;
        assert_eq!(doc.document_type_name, "domain");
        assert_eq!(doc.protocol_version, 0);
        assert_eq!(
            doc.id().to_buffer(),
            Identifier::from_string(
                "4veLBZPHDkaCPF9LfZ8fX3JZiS5q5iUVGhdBbaa9ga5E",
                Encoding::Base58
            )
            .unwrap()
            .to_buffer()
        );
        assert_eq!(
            doc.data_contract_id.to_buffer(),
            Identifier::from_string(
                "566vcJkmebVCAb2Dkj2yVMSgGFcsshupnQqtsz1RFbcy",
                Encoding::Base58
            )
            .unwrap()
            .to_buffer()
        );

        assert_eq!(
            doc.properties()
                .get("label")
                .expect("expected to get label"),
            &Value::Text("user-9999".to_string())
        );
        assert_eq!(
            doc.properties()
                .get_at_path("records.dashUniqueIdentityId")
                .expect("expected to get value"),
            &Value::Identifier(
                bs58::decode("HBNMY5QWuBVKNFLhgBTC1VmpEnscrmqKPMXpnYSHwhfn")
                    .into_vec()
                    .unwrap()
                    .try_into()
                    .unwrap()
            )
        );
        assert_eq!(
            doc.properties()
                .get_at_path("subdomainRules.allowSubdomains")
                .expect("expected to get value"),
            &Value::Bool(false)
        );
        Ok(())
    }

    #[test]
    fn test_buffer_serialize_deserialize() {
        init();
        let init_doc = new_example_document();
        let buffer_document = init_doc.to_buffer().expect("no errors");

        let doc = ExtendedDocument::from_cbor_buffer(buffer_document)
            .expect("document should be created from buffer");

        assert_eq!(init_doc.created_at(), doc.created_at());
        assert_eq!(init_doc.updated_at(), doc.updated_at());
        assert_eq!(init_doc.id(), doc.id());
        assert_eq!(init_doc.data_contract_id, doc.data_contract_id);
        assert_eq!(init_doc.owner_id(), doc.owner_id());
    }

    #[test]
    fn test_to_object() {
        init();
        let dpns_contract = load_system_data_contract(SystemDataContract::DPNS).unwrap();
        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json").unwrap();
        let document = ExtendedDocument::from_json_string(&document_json, dpns_contract).unwrap();
        let document_object = document.to_json_object_for_validation().unwrap();

        for property in IDENTIFIER_FIELDS {
            let id = document_object
                .get(property)
                .unwrap()
                .as_array()
                .expect("the property must be an array");
            assert_eq!(32, id.len())
        }
    }

    #[test]
    fn test_json_serialize() -> Result<()> {
        init();

        let dpns_contract = load_system_data_contract(SystemDataContract::DPNS)?;
        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json")?;
        let document = ExtendedDocument::from_json_string(&document_json, dpns_contract)?;
        let string = serde_json::to_string(&document)?;

        assert_eq!("{\"$protocolVersion\":0,\"$type\":\"domain\",\"$dataContractId\":\"566vcJkmebVCAb2Dkj2yVMSgGFcsshupnQqtsz1RFbcy\",\"$id\":\"4veLBZPHDkaCPF9LfZ8fX3JZiS5q5iUVGhdBbaa9ga5E\",\"$ownerId\":\"HBNMY5QWuBVKNFLhgBTC1VmpEnscrmqKPMXpnYSHwhfn\",\"label\":\"user-9999\",\"normalizedLabel\":\"user-9999\",\"normalizedParentDomainName\":\"dash\",\"preorderSalt\":\"BzQi567XVqc8wYiVHS887sJtL6MDbxLHNnp+UpTFSB0=\",\"records\":{\"dashUniqueIdentityId\":\"HBNMY5QWuBVKNFLhgBTC1VmpEnscrmqKPMXpnYSHwhfn\"},\"subdomainRules\":{\"allowSubdomains\":false},\"$revision\":1,\"$createdAt\":null,\"$updatedAt\":null}", string);

        Ok(())
    }

    #[test]
    fn test_document_to_buffer() -> Result<()> {
        init();

        let document_json = get_data_from_file("src/tests/payloads/document_dpns.json")?;
        let dpns_contract = load_system_data_contract(SystemDataContract::DPNS).unwrap();
        ExtendedDocument::from_json_string(&document_json, dpns_contract)
            .expect("expected extended document");
        Ok(())
    }

    #[test]
    fn deserialize_js_cpp_cbor() -> Result<()> {
        let document_cbor = document_cbor_bytes();

        let document = ExtendedDocument::from_cbor_buffer(document_cbor)?;

        assert_eq!(document.protocol_version, 1);
        assert_eq!(
            document.id().to_buffer().to_vec(),
            vec![
                113, 93, 61, 101, 117, 96, 36, 162, 222, 10, 177, 178, 187, 30, 131, 181, 239, 41,
                123, 240, 198, 250, 97, 106, 173, 92, 136, 126, 79, 16, 222, 249
            ]
        );
        assert_eq!(&document.document_type_name, "niceDocument");
        assert_eq!(
            document.data_contract_id.to_buffer().to_vec(),
            vec![
                122, 188, 95, 154, 180, 188, 208, 97, 46, 214, 202, 206, 194, 4, 221, 109, 116, 17,
                165, 97, 39, 212, 36, 138, 241, 234, 218, 203, 147, 82, 93, 162
            ]
        );
        assert_eq!(
            document.owner_id().to_buffer().to_vec(),
            vec![
                182, 191, 55, 77, 48, 47, 190, 43, 81, 27, 67, 226, 61, 3, 63, 150, 94, 46, 51,
                160, 36, 199, 65, 157, 176, 117, 51, 212, 186, 125, 112, 142
            ]
        );
        assert_eq!(document.revision(), Some(&1));
        assert_eq!(document.created_at().unwrap(), &1656583332347);
        assert_eq!(document.properties().get_string("name").unwrap(), "Cutie");

        Ok(())
    }

    #[test]
    fn to_buffer_serialize_to_the_same_format_as_js_dpp() -> Result<()> {
        let document_cbor = document_cbor_bytes();
        let document = ExtendedDocument::from_cbor_buffer(&document_cbor)?;

        let buffer = document.to_buffer()?;

        assert_eq!(hex::encode(document_cbor), hex::encode(buffer));
        Ok(())
    }

    #[test]
    fn json_should_generate_human_readable_binaries() {
        let data_contract = data_contract_with_dynamic_properties();
        let alpha_value = vec![10_u8; 32];
        let id = vec![11_u8; 32];
        let owner_id = vec![12_u8; 32];
        let data_contract_id = vec![13_u8; 32];

        let raw_document = json!({
            "$protocolVersion"  : 0,
            "$id" : id,
            "$ownerId" : owner_id,
            "$type" : "test",
            "$dataContractId" : data_contract_id,
            "$revision" : 1,
            "alphaBinary" : alpha_value,
            "alphaIdentifier" : alpha_value,
        });

        let document =
            ExtendedDocument::from_raw_json_document(raw_document, data_contract).unwrap();
        let json_document = document.to_pretty_json().expect("no errors");

        assert_eq!(
            json_document["$id"],
            JsonValue::String(bs58::encode(&id).into_string())
        );
        assert_eq!(
            json_document["$ownerId"],
            JsonValue::String(bs58::encode(&owner_id).into_string())
        );
        assert_eq!(
            json_document["$dataContractId"],
            JsonValue::String(bs58::encode(&data_contract_id).into_string())
        );
        assert_eq!(
            json_document["alphaBinary"],
            JsonValue::String(base64::encode(&alpha_value))
        );
        assert_eq!(
            json_document["alphaIdentifier"],
            JsonValue::String(bs58::encode(&alpha_value).into_string())
        );
    }

    fn document_cbor_bytes() -> Vec<u8> {
        hex::decode("01a7632469645820715d3d65756024a2de0ab1b2bb1e83b5ef297bf0c6fa616aad5c887e4f10def9646e616d656543757469656524747970656c6e696365446f63756d656e7468246f776e657249645820b6bf374d302fbe2b511b43e23d033f965e2e33a024c7419db07533d4ba7d708e69247265766973696f6e016a246372656174656441741b00000181b40fa1fb6f2464617461436f6e7472616374496458207abc5f9ab4bcd0612ed6cacec204dd6d7411a56127d4248af1eadacb93525da2").unwrap()
    }

    fn new_example_document() -> ExtendedDocument {
        ExtendedDocument {
            document: Document {
                id: generate_random_identifier_struct(),
                owner_id: generate_random_identifier_struct(),
                created_at: Some(1648013404492),
                updated_at: Some(1648013404492),
                ..Default::default()
            },
            data_contract_id: generate_random_identifier_struct(),
            ..Default::default()
        }
    }
}
