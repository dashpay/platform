use crate::data_contract::conversion::cbor_conversion::DataContractCborConversionMethodsV0;
use crate::data_contract::conversion::platform_value_conversion::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::data_contract::DataContractV0;
use crate::data_contract::data_contract_config::v0::DataContractConfigGettersV0;
use crate::data_contract::data_contract_config::DataContractConfig;
use crate::data_contract::identifiers_and_binary_paths::DataContractIdentifiersAndBinaryPathsMethodsV0;
use crate::data_contract::serialized_version::v0::DataContractSerializationFormatV0;
use crate::data_contract::{data_contract_config, property_names, DataContract};
use crate::util::cbor_value::CborCanonicalMap;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::util::{cbor_serializer, deserializer};
use crate::version::PlatformVersion;
use crate::{Convertible, ProtocolError};
use ciborium::Value as CborValue;
use integer_encoding::VarInt;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::{Identifier, Value, ValueMapHelper};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::convert::TryFrom;

impl DataContractCborConversionMethodsV0 for DataContractV0 {
    fn from_cbor_with_id(
        cbor_bytes: impl AsRef<[u8]>,
        contract_id: Option<Identifier>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut data_contract = Self::from_cbor(cbor_bytes, platform_version)?;
        if let Some(id) = contract_id {
            data_contract.id = id;
        }
        Ok(data_contract)
    }

    fn from_cbor(
        cbor_bytes: impl AsRef<[u8]>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let SplitProtocolVersionOutcome {
            protocol_version,
            protocol_version_size,
            main_message_bytes: contract_cbor_bytes,
        } = deserializer::split_cbor_protocol_version(cbor_bytes.as_ref())?;

        let data_contract_cbor_value: CborValue = ciborium::de::from_reader(contract_cbor_bytes)
            .map_err(|_| {
                ProtocolError::DecodingError(format!(
                    "unable to decode contract with protocol version {} offset {}",
                    protocol_version, protocol_version_size
                ))
            })?;

        let data_contract_value: Value =
            Value::try_from(data_contract_cbor_value).map_err(ProtocolError::ValueError)?;

        // TODO: use from object
        // TODO: We should used versioned format?
        let value: DataContractSerializationFormatV0 =
            platform_value::from_value(data_contract_value).map_err(ProtocolError::ValueError)?;

        DataContractV0::try_from(value, platform_version)
        //
        // let contract_id: Identifier = data_contract_map.get_identifier(property_names::ID)?;
        // let owner_id: Identifier = data_contract_map.get_identifier(property_names::OWNER_ID)?;
        // let schema = data_contract_map.get_string(property_names::SCHEMA)?;
        // let version = data_contract_map.get_integer(property_names::VERSION)?;
        //
        // // Defs
        // let defs =
        //     data_contract_map.get_optional_inner_str_json_value_map::<BTreeMap<_, _>>("$defs")?;
        //
        // // Documents
        // let documents: BTreeMap<String, JsonValue> = data_contract_map
        //     .get_inner_str_json_value_map("documents")
        //     .map_err(ProtocolError::ValueError)?;
        //
        // let mutability = DataContractConfig::get_contract_configuration_properties(
        //     &data_contract_map,
        //     platform_version,
        // )
        // .map_err(|e| ProtocolError::ParsingError(e.to_string()))?;
        // let definition_references =
        //     DataContract::get_definitions(&data_contract_map, platform_version)?;
        // let document_types = DataContract::get_document_types_from_contract(
        //     contract_id,
        //     &data_contract_map,
        //     &definition_references,
        //     mutability.documents_keep_history_contract_default(),
        //     mutability.documents_mutable_contract_default(),
        //     platform_version,
        // )
        // .map_err(|e| ProtocolError::ParsingError(e.to_string()))?;
        //
        // let mut data_contract = Self {
        //     id: contract_id,
        //     schema,
        //     version,
        //     owner_id,
        //     documents,
        //     defs,
        //     metadata: None,
        //     binary_properties: Default::default(),
        //     document_types,
        //     config: mutability,
        // };
        //
        // data_contract.generate_binary_properties(platform_version)?;
        //
        // Ok(data_contract)
    }

    fn to_cbor(&self, platform_version: &PlatformVersion) -> Result<Vec<u8>, ProtocolError> {
        // TODO: Use to_object and then cbor it
        // TODO: We should used versioned format?
        let format = DataContractSerializationFormatV0::from(self);
        self.to_serialization_format_based_on_default_current_version(platform_version)?;

        let mut buf: Vec<u8> = Vec::new();

        ciborium::ser::into_writer(format, &mut buf)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        Ok(buf)
    }

    // /// Returns Data Contract as a Buffer
    // fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
    //     let mut object = self.to_object()?;
    //     if self.defs.is_none() {
    //         object.remove(property_names::DEFINITIONS)?;
    //     }
    //     object
    //         .to_map_mut()
    //         .unwrap()
    //         .sort_by_lexicographical_byte_ordering_keys_and_inner_maps();
    //
    //     // we are on version 0 here
    //     cbor_serializer::serializable_value_to_cbor(&object, Some(0))
    // }

    fn to_cbor_canonical_map(&self) -> Result<CborCanonicalMap, ProtocolError> {
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
