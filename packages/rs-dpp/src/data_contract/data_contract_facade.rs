use crate::data_contract::state_transition::{
    DataContractCreateTransition, DataContractUpdateTransition,
};
use crate::data_contract::validation::data_contract_validator::DataContractValidator;
use crate::data_contract::{DataContract, DataContractFactory};
use crate::document::document_transition::JsonValue;
use crate::prelude::{Document, Identifier, ValidationResult};
use crate::version::ProtocolVersionValidator;
use crate::ProtocolError;
use std::sync::Arc;

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

    /// Create Data Contract
    pub fn create(
        &self,
        owner_id: Identifier,
        documents: JsonValue,
    ) -> Result<DataContract, ProtocolError> {
        self.factory.create(owner_id, documents)
    }

    /// Create Data Contract from plain object
    pub async fn create_from_object(
        &self,
        raw_data_contract: JsonValue,
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
        data_contract: JsonValue,
    ) -> Result<ValidationResult<()>, ProtocolError> {
        // TODO: figure out what to do with a case where it's not a raw data contract
        // let rawDataContract;
        // if (dataContract instanceof DataContract) {
        //     rawDataContract = dataContract.toObject();
        // } else {
        //     rawDataContract = dataContract;
        // }

        self.data_contract_validator.validate(&data_contract)
    }
}
