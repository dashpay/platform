use anyhow::anyhow;
use serde_json::{json, Map, Number, Value as JsonValue};
use std::sync::Arc;

use data_contract::state_transition::property_names as st_prop;

use crate::data_contract::errors::InvalidDataContractError;
use crate::data_contract::property_names;
use crate::util::serializer::value_to_cbor;
use crate::{
    data_contract::{self, generate_data_contract_id},
    decode_protocol_entity_factory::DecodeProtocolEntity,
    errors::{consensus::ConsensusError, ProtocolError},
    prelude::Identifier,
    util::entropy_generator,
};

use super::state_transition::data_contract_create_transition::DataContractCreateTransition;
use super::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use super::{validation::data_contract_validator::DataContractValidator, DataContract};

/// A way to provide external entropy generator.
pub trait EntropyGenerator {
    fn generate(&self) -> [u8; 32];
}

struct DefaultEntropyGenerator;

impl EntropyGenerator for DefaultEntropyGenerator {
    fn generate(&self) -> [u8; 32] {
        entropy_generator::generate().expect("entropy generation failed")
    }
}

pub struct DataContractFactory {
    protocol_version: u32,
    validate_data_contract: Arc<DataContractValidator>,
    entropy_generator: Box<dyn EntropyGenerator>,
}

impl DataContractFactory {
    pub fn new(protocol_version: u32, validate_data_contract: Arc<DataContractValidator>) -> Self {
        Self {
            protocol_version,
            validate_data_contract,
            entropy_generator: Box::new(DefaultEntropyGenerator),
        }
    }

    pub fn new_with_entropy_generator(
        protocol_version: u32,
        validate_data_contract: Arc<DataContractValidator>,
        entropy_generator: Box<dyn EntropyGenerator>,
    ) -> Self {
        Self {
            protocol_version,
            validate_data_contract,
            entropy_generator,
        }
    }

    /// Create Data Contract
    pub fn create(
        &self,
        owner_id: Identifier,
        documents: JsonValue,
        definitions: Option<JsonValue>,
    ) -> Result<DataContract, ProtocolError> {
        let entropy = self.entropy_generator.generate();

        let data_contract_id =
            Identifier::from_bytes(&generate_data_contract_id(owner_id.to_buffer(), entropy))?;

        // todo: workaround

        let mut root_map = Map::new();

        root_map.insert(
            property_names::ID.to_string(),
            JsonValue::String(bs58::encode(data_contract_id.to_buffer().as_slice()).into_string()),
        );
        root_map.insert(
            property_names::OWNER_ID.to_string(),
            JsonValue::String(bs58::encode(owner_id.to_buffer().as_slice()).into_string()),
        );
        root_map.insert(
            property_names::SCHEMA.to_string(),
            JsonValue::String(data_contract::SCHEMA_URI.to_string()),
        );
        root_map.insert(
            property_names::VERSION.to_string(),
            JsonValue::Number(1.into()),
        );

        if let Some(defs) = definitions {
            root_map.insert(property_names::DEFINITIONS.to_string(), defs);
        }

        root_map.insert(property_names::DOCUMENTS.to_string(), documents);

        let cbor = value_to_cbor(JsonValue::Object(root_map), Some(1))?;

        DataContract::from_cbor(cbor)
    }

    /// Create Data Contract from plain object
    pub async fn create_from_object(
        &self,
        raw_data_contract: JsonValue,
        skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        if !skip_validation {
            let result = self.validate_data_contract.validate(&raw_data_contract)?;

            if !result.is_valid() {
                return Err(ProtocolError::InvalidDataContractError(
                    InvalidDataContractError::new(result.errors, raw_data_contract),
                ));
            }
        }
        DataContract::from_raw_object(raw_data_contract)
    }

    /// Create Data Contract from buffer
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        let (protocol_version, mut raw_data_contract) =
            DecodeProtocolEntity::decode_protocol_entity(buffer)?;

        match raw_data_contract {
            JsonValue::Object(ref mut m) => m.insert(
                String::from("protocolVersion"),
                JsonValue::Number(Number::from(protocol_version)),
            ),
            _ => {
                return Err(ConsensusError::SerializedObjectParsingError {
                    parsing_error: anyhow!("the '{:?}' is not a map", raw_data_contract),
                }
                .into())
            }
        };

        self.create_from_object(raw_data_contract, skip_validation)
            .await
    }

    pub fn create_data_contract_create_transition(
        &self,
        data_contract: DataContract,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        DataContractCreateTransition::from_raw_object(json!({
            st_prop::PROTOCOL_VERSION: self.protocol_version,
            st_prop::DATA_CONTRACT: data_contract.to_object(false)?,
            st_prop::ENTROPY: data_contract.entropy,
        }))
    }

    pub fn create_data_contract_update_transition(
        &self,
        data_contract: DataContract,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        DataContractUpdateTransition::from_raw_object(json!({
            st_prop::PROTOCOL_VERSION: self.protocol_version,
            st_prop::DATA_CONTRACT: data_contract.to_object(false)?,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};
    use std::sync::Arc;

    pub struct TestData {
        data_contract: DataContract,
        raw_data_contract: JsonValue,
        factory: DataContractFactory,
    }

    fn get_test_data() -> TestData {
        let data_contract = get_data_contract_fixture(None);
        let raw_data_contract = data_contract.to_object(false).unwrap();
        let protocol_version_validator = ProtocolVersionValidator::new(
            LATEST_VERSION,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        );
        let data_contract_validator = Arc::new(DataContractValidator::new(Arc::new(
            protocol_version_validator,
        )));

        let factory = DataContractFactory::new(1, data_contract_validator);
        TestData {
            data_contract,
            raw_data_contract,
            factory,
        }
    }

    #[test]
    fn should_create_data_contract_with_specified_name_and_docs_definition() {
        let TestData {
            data_contract,
            raw_data_contract,
            factory,
        } = get_test_data();

        let raw_defs = raw_data_contract
            .get(property_names::DEFINITIONS)
            .expect("documents property should exist")
            .clone();

        let raw_documents = raw_data_contract
            .get(property_names::DOCUMENTS)
            .expect("documents property should exist")
            .clone();

        let result = factory
            .create(data_contract.owner_id, raw_documents, Some(raw_defs))
            .expect("Data Contract should be created");

        assert_eq!(data_contract.protocol_version, result.protocol_version);
        // id is generated based on entropy which is different every time the `create` call is used
        assert_eq!(data_contract.id.buffer.len(), result.id.buffer.len());
        assert_ne!(data_contract.id, result.id);
        assert_eq!(data_contract.schema, result.schema);
        assert_eq!(data_contract.owner_id, result.owner_id);
        assert_eq!(data_contract.documents, result.documents);
        assert_eq!(data_contract.metadata, result.metadata);
        assert_eq!(data_contract.binary_properties, result.binary_properties);
    }

    #[tokio::test]
    async fn should_crate_data_contract_from_object() {
        let TestData {
            data_contract,
            raw_data_contract,
            factory,
        } = get_test_data();

        let result = factory
            .create_from_object(raw_data_contract, true)
            .await
            .expect("Data Contract should be created");

        assert_eq!(data_contract.protocol_version, result.protocol_version);
        assert_eq!(data_contract.id, result.id);
        assert_eq!(data_contract.schema, result.schema);
        assert_eq!(data_contract.owner_id, result.owner_id);
        assert_eq!(data_contract.documents, result.documents);
        assert_eq!(data_contract.metadata, result.metadata);
        assert_eq!(data_contract.binary_properties, result.binary_properties);
        assert_eq!(data_contract.defs, result.defs);
    }

    #[tokio::test]
    async fn should_create_data_contract_from_buffer() {
        let TestData {
            data_contract,
            factory,
            ..
        } = get_test_data();
        let serialized_data_contract = data_contract
            .to_buffer()
            .expect("should be serialized to buffer");
        let result = factory
            .create_from_buffer(serialized_data_contract, false)
            .await
            .expect("Data Contract should be created from the buffer");

        assert_eq!(data_contract.protocol_version, result.protocol_version);
        assert_eq!(data_contract.id, result.id);
        assert_eq!(data_contract.schema, result.schema);
        assert_eq!(data_contract.owner_id, result.owner_id);
        assert_eq!(data_contract.documents, result.documents);
        assert_eq!(data_contract.metadata, result.metadata);
        assert_eq!(data_contract.binary_properties, result.binary_properties);
        assert_eq!(data_contract.defs, result.defs);
    }

    #[test]
    fn should_create_data_contract_create_transition_from_data_contract() {
        let TestData {
            data_contract,
            factory,
            raw_data_contract,
        } = get_test_data();

        let result = factory
            .create_data_contract_create_transition(data_contract.clone())
            .expect("Data Contract Transition should be created");

        assert_eq!(1, result.get_protocol_version());
        assert_eq!(&data_contract.entropy, result.get_entropy());
        assert_eq!(
            raw_data_contract,
            result.data_contract.to_object(false).unwrap()
        );
    }
}
