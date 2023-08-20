use platform_value::btreemap_extensions::BTreeValueRemoveFromMapHelper;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::sync::Arc;

use data_contract::state_transition::property_names as st_prop;
use platform_value::{Bytes32, Error, Value};

use crate::data_contract::contract_config::ContractConfig;
use crate::data_contract::errors::InvalidDataContractError;

use crate::data_contract::property_names::PROTOCOL_VERSION;

use crate::consensus::basic::decode::SerializedObjectParsingError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::serialization_traits::PlatformDeserializable;
use crate::state_transition::StateTransitionType;

use crate::util::entropy_generator::{DefaultEntropyGenerator, EntropyGenerator};
use crate::{
    data_contract::{self, generate_data_contract_id},
    errors::ProtocolError,
    prelude::Identifier,
    Convertible,
};

use super::state_transition::data_contract_create_transition::DataContractCreateTransition;
use super::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use super::{validation::data_contract_validator::DataContractValidator, DataContract};

pub struct DataContractFactory {
    protocol_version: u32,
    validate_data_contract: Arc<DataContractValidator>,
    entropy_generator: Box<dyn EntropyGenerator>,
}

#[derive(Clone, Debug)]
pub struct CreatedDataContract {
    pub data_contract: DataContract,
    pub entropy_used: Bytes32,
}

impl CreatedDataContract {
    pub fn from_raw_object(raw_object: Value) -> Result<Self, ProtocolError> {
        let mut raw_map = raw_object
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;

        let raw_data_contract = raw_map.remove(st_prop::DATA_CONTRACT).ok_or_else(|| {
            Error::StructureError("unable to remove property dataContract".to_string())
        })?;

        let raw_entropy = raw_map
            .remove_bytes(st_prop::ENTROPY)
            .map_err(ProtocolError::ValueError)?;

        let data_contract = DataContract::from_raw_object(raw_data_contract)?;

        Ok(Self {
            data_contract,
            entropy_used: Bytes32::from_vec(raw_entropy)?,
        })
    }
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
        documents: Value,
        config: Option<ContractConfig>,
        definitions: Option<Value>,
    ) -> Result<CreatedDataContract, ProtocolError> {
        let entropy = Bytes32::new(self.entropy_generator.generate()?);

        let data_contract_id = generate_data_contract_id(owner_id.to_buffer(), entropy.to_buffer());

        let definition_references = definitions
            .as_ref()
            .map(|defs| defs.to_btree_ref_string_map())
            .transpose()
            .map_err(ProtocolError::ValueError)?
            .unwrap_or_default();

        let config = config.unwrap_or_default();
        let document_types = data_contract::get_document_types_from_value(
            data_contract_id,
            &documents,
            &definition_references,
            config.documents_keep_history_contract_default,
            config.documents_mutable_contract_default,
        )
        .map_err(|e| ProtocolError::ParsingError(e.to_string()))?;

        let document_values = documents
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)?;
        let documents = document_values
            .into_iter()
            .map(|(key, value)| Ok((key, value.try_into().map_err(ProtocolError::ValueError)?)))
            .collect::<Result<BTreeMap<String, JsonValue>, ProtocolError>>()?;

        let json_defs = if !definition_references.is_empty() {
            Some(
                definition_references
                    .into_iter()
                    .map(|(key, value)| {
                        Ok((
                            key,
                            value
                                .clone()
                                .try_into()
                                .map_err(ProtocolError::ValueError)?,
                        ))
                    })
                    .collect::<Result<BTreeMap<String, JsonValue>, ProtocolError>>()?,
            )
        } else {
            None
        };
        let mut data_contract = DataContract {
            protocol_version: self.protocol_version,
            id: data_contract_id,
            schema: data_contract::SCHEMA_URI.to_string(),
            version: 1,
            owner_id,
            document_types,
            metadata: None,
            config,
            documents,
            defs: json_defs,
            binary_properties: Default::default(),
        };

        data_contract.generate_binary_properties();
        Ok(CreatedDataContract {
            data_contract,
            entropy_used: entropy,
        })
    }

    /// Create Data Contract from plain object
    pub async fn create_from_object(
        &self,
        mut data_contract_object: Value,
        skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        if !skip_validation {
            self.validate_data_contract(&data_contract_object)?;
        }
        if !data_contract_object
            .has(PROTOCOL_VERSION)
            .map_err(ProtocolError::ValueError)?
        {
            data_contract_object
                .insert(
                    PROTOCOL_VERSION.to_string(),
                    Value::U32(self.protocol_version),
                )
                .map_err(ProtocolError::ValueError)?;
        }
        DataContract::from_raw_object(data_contract_object)
    }

    /// Create Data Contract from buffer
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        let data_contract: DataContract =
            DataContract::deserialize(buffer.as_slice()).map_err(|e| {
                ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
                    SerializedObjectParsingError::new(format!("Decode protocol entity: {:#?}", e)),
                ))
            })?;

        if !skip_validation {
            self.validate_data_contract(&data_contract.to_cleaned_object()?)?;
        }

        Ok(data_contract)
    }

    pub fn validate_data_contract(&self, raw_data_contract: &Value) -> Result<(), ProtocolError> {
        let result = self.validate_data_contract.validate(raw_data_contract)?;

        if !result.is_valid() {
            return Err(ProtocolError::InvalidDataContractError(
                InvalidDataContractError::new(result.errors, raw_data_contract.to_owned()),
            ));
        }

        Ok(())
    }

    pub fn create_data_contract_create_transition(
        &self,
        created_data_contract: CreatedDataContract,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        Ok(DataContractCreateTransition {
            protocol_version: self.protocol_version,
            transition_type: StateTransitionType::DataContractCreate,
            data_contract: created_data_contract.data_contract,
            entropy: created_data_contract.entropy_used,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }

    pub fn create_data_contract_update_transition(
        &self,
        data_contract: DataContract,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        Ok(DataContractUpdateTransition {
            protocol_version: self.protocol_version,
            transition_type: StateTransitionType::DataContractUpdate,
            signature_public_key_id: 0,
            signature: Default::default(),
            data_contract,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::property_names;
    use crate::serialization_traits::PlatformSerializable;
    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};
    use crate::Convertible;
    use std::sync::Arc;

    pub struct TestData {
        created_data_contract: CreatedDataContract,
        raw_data_contract: Value,
        factory: DataContractFactory,
    }

    fn get_test_data() -> TestData {
        let created_data_contract = get_data_contract_fixture(None);
        let raw_data_contract = created_data_contract.data_contract.to_object().unwrap();
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
            created_data_contract,
            raw_data_contract,
            factory,
        }
    }

    #[test]
    fn should_create_data_contract_with_specified_name_and_docs_definition() {
        let TestData {
            created_data_contract,
            raw_data_contract,
            factory,
        } = get_test_data();

        let data_contract = created_data_contract.data_contract;

        let raw_defs = raw_data_contract
            .get_value(property_names::DEFINITIONS)
            .expect("documents property should exist")
            .clone();

        let raw_documents = raw_data_contract
            .get_value(property_names::DOCUMENTS)
            .expect("documents property should exist")
            .clone();

        let result = factory
            .create(data_contract.owner_id, raw_documents, None, Some(raw_defs))
            .expect("Data Contract should be created")
            .data_contract;

        assert_eq!(data_contract.protocol_version, result.protocol_version);
        // id is generated based on entropy which is different every time the `create` call is used
        assert_eq!(data_contract.id.len(), result.id.len());
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
            created_data_contract,
            raw_data_contract,
            factory,
        } = get_test_data();

        let data_contract = created_data_contract.data_contract;

        let result = factory
            .create_from_object(raw_data_contract.into(), true)
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
            created_data_contract,
            factory,
            ..
        } = get_test_data();

        let data_contract = created_data_contract.data_contract;

        let serialized_data_contract = data_contract
            .serialize()
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
            created_data_contract,
            factory,
            raw_data_contract,
        } = get_test_data();

        let result = factory
            .create_data_contract_create_transition(created_data_contract.clone())
            .expect("Data Contract Transition should be created");

        assert_eq!(1, result.get_protocol_version());
        assert_eq!(&created_data_contract.entropy_used, &result.entropy);
        assert_eq!(raw_data_contract, result.data_contract.to_object().unwrap());
    }
}
