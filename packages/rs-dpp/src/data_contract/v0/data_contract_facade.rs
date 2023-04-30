use crate::data_contract::contract_config::ContractConfig;
use crate::data_contract::validation::data_contract_validator::DataContractValidator;
use crate::data_contract::{DataContract, DataContractFactory};

use crate::prelude::Identifier;
use crate::util::entropy_generator::EntropyGenerator;
use crate::validation::SimpleConsensusValidationResult;
use crate::version::ProtocolVersionValidator;
use crate::ProtocolError;
use platform_value::Value;
use std::sync::Arc;

use super::state_transition::data_contract_create_transition::DataContractCreateTransition;
use super::state_transition::data_contract_update_transition::DataContractUpdateTransition;

pub struct DataContractFacade {
    factory: DataContractFactory,
    data_contract_validator: Arc<DataContractValidator>,
}

impl DataContractFacade {
    pub fn new(
        protocol_version: u32,
        protocol_version_validator: Arc<ProtocolVersionValidator>,
    ) -> Self {
        let validator = Arc::new(DataContractValidator::new(protocol_version_validator));
        Self {
            factory: DataContractFactory::new(protocol_version, validator.clone()),
            data_contract_validator: validator,
        }
    }

    pub fn new_with_entropy_generator(
        protocol_version: u32,
        protocol_version_validator: Arc<ProtocolVersionValidator>,
        entropy_generator: Box<dyn EntropyGenerator>,
    ) -> Self {
        let validator = Arc::new(DataContractValidator::new(protocol_version_validator));
        Self {
            factory: DataContractFactory::new_with_entropy_generator(
                protocol_version,
                validator.clone(),
                entropy_generator,
            ),
            data_contract_validator: validator,
        }
    }

    /// Create Data Contract
    pub fn create(
        &self,
        owner_id: Identifier,
        documents: Value,
        config: Option<ContractConfig>,
        definitions: Option<Value>,
    ) -> Result<DataContract, ProtocolError> {
        self.factory
            .create(owner_id, documents, config, definitions)
    }

    /// Create Data Contract from plain object
    pub async fn create_from_object(
        &self,
        raw_data_contract: Value,
        skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        self.factory
            .create_from_object(raw_data_contract, skip_validation)
            .await
    }

    /// Create Data Contract from buffer
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        self.factory
            .create_from_buffer(buffer, skip_validation)
            .await
    }

    /// Create Data Contract Create State Transition
    pub fn create_data_contract_create_transition(
        &self,
        data_contract: DataContract,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        self.factory
            .create_data_contract_create_transition(data_contract)
    }

    /// Create Data Contract Update State Transition
    pub fn create_data_contract_update_transition(
        &self,
        data_contract: DataContract,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        self.factory
            .create_data_contract_update_transition(data_contract)
    }

    /// Validate Data Contract
    pub async fn validate(
        &self,
        data_contract: Value,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        self.data_contract_validator.validate(&data_contract)
    }
}
