use anyhow::anyhow;
use serde_json::{json, Number, Value as JsonValue};
use std::collections::BTreeMap;

use crate::{
    data_contract::{self, generate_data_contract_id},
    decode_protocol_entity_factory::DecodeProtocolEntity,
    errors::{consensus::ConsensusError, ProtocolError},
    prelude::Identifier,
    util::entropy_generator,
    Convertible,
};

use super::{
    state_transition::{DataContractCreateTransition, DataContractUpdateTransition},
    validation::data_contract_validator::DataContractValidator,
    DataContract,
};
use data_contract::state_transition::properties as st_prop;

pub struct DataContractFactory {
    protocol_version: u32,
    validate_data_contract: DataContractValidator,
}

impl DataContractFactory {
    pub fn new(protocol_version: u32, validate_data_contract: DataContractValidator) -> Self {
        Self {
            protocol_version,
            validate_data_contract,
        }
    }

    /// Create Data Contract
    pub fn create(
        &self,
        owner_id: Identifier,
        documents: JsonValue,
    ) -> Result<DataContract, ProtocolError> {
        let entropy = entropy_generator::generate();
        let data_contract_id =
            Identifier::from_bytes(&generate_data_contract_id(owner_id.to_buffer(), entropy))?;

        let mut data_contract = DataContract {
            protocol_version: self.protocol_version,
            schema: String::from(data_contract::SCHEMA),
            id: data_contract_id,
            version: 1,
            owner_id,
            defs: BTreeMap::new(),
            entropy,

            ..Default::default()
        };

        if let JsonValue::Object(documents) = documents {
            for (document_name, value) in documents {
                data_contract.set_document_schema(document_name, value);
            }
        } else {
            return Err(ProtocolError::Generic(String::from(
                "attached documents are not in form a map",
            )));
        }

        Ok(data_contract)
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
                return Err(ProtocolError::InvalidDataContractError {
                    errors: result.errors,
                    raw_data_contract,
                });
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
            st_prop::PROPERTY_PROTOCOL_VERSION: self.protocol_version,
            st_prop::PROPERTY_DATA_CONTRACT: data_contract.to_object()?,
            st_prop::PROPERTY_ENTROPY: data_contract.entropy,
        }))
    }

    pub fn create_data_contract_update_transition(
        &self,
        data_contract: DataContract,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        DataContractUpdateTransition::from_raw_object(json!({
            st_prop::PROPERTY_PROTOCOL_VERSION: self.protocol_version,
            st_prop::PROPERTY_DATA_CONTRACT: data_contract.to_object()?,
        }))
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::DataContractFactory;
    use crate::{
        data_contract::{validation::data_contract_validator::DataContractValidator, DataContract},
        state_transition::StateTransitionLike,
        tests::fixtures::get_data_contract_fixture,
        version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
        Convertible,
    };
    use serde_json::Value as JsonValue;

    pub struct TestData {
        data_contract: DataContract,
        raw_data_contract: JsonValue,
        factory: DataContractFactory,
    }

    fn get_test_data() -> TestData {
        let data_contract = get_data_contract_fixture(None);
        let raw_data_contract = data_contract.to_object().unwrap();
        let protocol_version_validator = ProtocolVersionValidator::new(
            LATEST_VERSION,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        );
        let data_contract_validator =
            DataContractValidator::new(Arc::new(protocol_version_validator));

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
        let raw_documents = raw_data_contract
            .get("documents")
            .expect("documents property should exist")
            .clone();

        let result = factory
            .create(data_contract.owner_id.clone(), raw_documents)
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
    async fn should_crete_data_contract_from_buffer() {
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
        assert_eq!(raw_data_contract, result.data_contract.to_object().unwrap());
    }
}
