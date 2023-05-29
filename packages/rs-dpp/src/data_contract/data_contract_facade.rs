
use crate::data_contract::contract_config::ContractConfigV0;
use crate::data_contract::validation::data_contract_validation::DataContractValidator;
use crate::data_contract::{CreatedDataContract, DataContract, DataContractFactory};

use crate::prelude::Identifier;
use crate::util::entropy_generator::EntropyGenerator;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use std::sync::Arc;
use crate::data_contract::created_data_contract::CreatedDataContract;

use super::state_transition::data_contract_create_transition::DataContractCreateTransition;
use super::state_transition::data_contract_update_transition::DataContractUpdateTransition;

pub struct DataContractFacade {
    factory: DataContractFactory,
}

impl DataContractFacade {
    pub fn new(
        protocol_version: u32,
        preferred_default_data_contract_version: Option<u16>,
    ) -> Self {
        Self {
            factory: DataContractFactory::new(protocol_version),
        }
    }

    pub fn new_with_entropy_generator(
        protocol_version: u32,
        preferred_default_data_contract_version: Option<u16>,
        entropy_generator: Box<dyn EntropyGenerator>,
    ) -> Self {
        Self {
            factory: DataContractFactory::new_with_entropy_generator(
                protocol_version,
                validator.clone(),
                entropy_generator,
            ),
        }
    }

    /// Create Data Contract
    pub fn create(
        &self,
        owner_id: Identifier,
        documents: Value,
        config: Option<Value>,
        definitions: Option<Value>,
    ) -> Result<CreatedDataContract, ProtocolError> {
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
        created_data_contract: CreatedDataContract,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        self.factory
            .create_data_contract_create_transition(created_data_contract)
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
        DataContract::validate(&data_contract)
    }

    /// Validate Data Contract for a different version
    pub async fn validate_for_version(
        &self,
        data_contract: Value,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        DataContract::validate(&data_contract)
    }
}
