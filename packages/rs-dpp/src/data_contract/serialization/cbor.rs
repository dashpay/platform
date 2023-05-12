use crate::data_contract::{contract_config, property_names, DataContract};
use crate::prelude::Identifier;
use crate::util::cbor_value::CborCanonicalMap;
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::{data_contract, ProtocolError};
use ciborium::Value as CborValue;

use integer_encoding::VarInt;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::{Bytes32, Value};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

impl DataContract {
    pub fn from_cbor_with_id(
        cbor_bytes: impl AsRef<[u8]>,
        contract_id: Option<Identifier>,
    ) -> Result<Self, ProtocolError> {
        let mut data_contract = Self::from_cbor(cbor_bytes)?;
        if let Some(id) = contract_id {
            data_contract.id = id;
        }
        Ok(data_contract)
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
            Value::convert_from_cbor_map(data_contract_cbor_map)?;

        let contract_id: Identifier = data_contract_map.get_identifier(property_names::ID)?;
        let owner_id: Identifier = data_contract_map.get_identifier(property_names::OWNER_ID)?;
        let schema = data_contract_map.get_string(property_names::SCHEMA)?;
        let version = data_contract_map.get_integer(property_names::VERSION)?;

        // Defs
        let defs =
            data_contract_map.get_optional_inner_str_json_value_map::<BTreeMap<_, _>>("$defs")?;

        // Documents
        let documents: BTreeMap<String, JsonValue> = data_contract_map
            .get_inner_str_json_value_map("documents")
            .map_err(ProtocolError::ValueError)?;

        let mutability = data_contract::get_contract_configuration_properties(&data_contract_map)
            .map_err(|e| ProtocolError::ParsingError(e.to_string()))?;
        let definition_references = data_contract::get_definitions(&data_contract_map)?;
        let document_types = data_contract::get_document_types_from_contract(
            contract_id,
            &data_contract_map,
            &definition_references,
            mutability.documents_keep_history_contract_default,
            mutability.documents_mutable_contract_default,
        )
        .map_err(|e| ProtocolError::ParsingError(e.to_string()))?;

        let mut data_contract = Self {
            protocol_version,
            id: contract_id,
            schema,
            version,
            owner_id,
            documents,
            defs,
            metadata: None,
            binary_properties: Default::default(),
            document_types,
            config: mutability,
        };

        data_contract.generate_binary_properties();

        Ok(data_contract)
    }

    pub fn to_cbor(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut buf = self.protocol_version.encode_var_vec();

        let mut contract_cbor_map = self.to_cbor_canonical_map()?;

        contract_cbor_map.insert(contract_config::property::READONLY, self.config.readonly);
        contract_cbor_map.insert(
            contract_config::property::KEEPS_HISTORY,
            self.config.keeps_history,
        );
        contract_cbor_map.insert(
            contract_config::property::DOCUMENTS_KEEP_HISTORY_CONTRACT_DEFAULT,
            self.config.documents_keep_history_contract_default,
        );
        contract_cbor_map.insert(
            contract_config::property::DOCUMENTS_MUTABLE_CONTRACT_DEFAULT,
            self.config.documents_mutable_contract_default,
        );

        let mut contract_buf = contract_cbor_map
            .to_bytes()
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        buf.append(&mut contract_buf);
        Ok(buf)
    }

    pub(crate) fn to_cbor_canonical_map(&self) -> Result<CborCanonicalMap, ProtocolError> {
        let mut contract_cbor_map = CborCanonicalMap::new();

        contract_cbor_map.insert(property_names::ID, self.id.to_buffer().to_vec());
        contract_cbor_map.insert(property_names::SCHEMA, self.schema.as_str());
        contract_cbor_map.insert(property_names::VERSION, self.version);
        contract_cbor_map.insert(property_names::OWNER_ID, self.owner_id.to_buffer().to_vec());

        let docs = CborValue::serialized(&self.documents)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        contract_cbor_map.insert(property_names::DOCUMENTS, docs);

        if let Some(_defs) = &self.defs {
            contract_cbor_map.insert(
                property_names::DEFINITIONS,
                CborValue::serialized(&self.defs)
                    .map_err(|e| ProtocolError::EncodingError(e.to_string()))?,
            );
        };

        Ok(contract_cbor_map)
    }
}
