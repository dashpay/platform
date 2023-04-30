mod v0;

use crate::data_contract::contract_config::ContractConfig;
use crate::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::data_contract::DataContract;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
pub use v0::*;

/// A factory for creating and managing data contracts.
///
/// The factory is versioned to support different implementations of data contracts in the future.
pub enum DataContractFactory {
    /// The version 0 implementation of the data contract factory.
    V0(DataContractFactoryV0),
}

impl DataContractFactory {
    /// Create a DataContract
    pub fn create(
        &self,
        owner_id: Identifier,
        documents: Value,
        config: Option<ContractConfig>,
        definitions: Option<Value>,
    ) -> Result<DataContract, ProtocolError> {
        match self {
            DataContractFactory::V0(v0) => v0.create(owner_id, documents, config, definitions),
        }
    }

    /// Create a DataContract from a plain object
    pub async fn create_from_object(
        &self,
        data_contract_object: Value,
        skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        match self {
            DataContractFactory::V0(v0) => {
                v0.create_from_object(data_contract_object, skip_validation)
                    .await
            }
        }
    }

    /// Create a DataContract from a buffer
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        match self {
            DataContractFactory::V0(v0) => v0.create_from_buffer(buffer, skip_validation).await,
        }
    }

    /// Create a DataContractCreateTransition
    pub fn create_data_contract_create_transition(
        &self,
        data_contract: DataContract,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        match self {
            DataContractFactory::V0(v0) => v0.create_data_contract_create_transition(data_contract),
        }
    }

    /// Create a DataContractUpdateTransition
    pub fn create_data_contract_update_transition(
        &self,
        data_contract: DataContract,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        match self {
            DataContractFactory::V0(v0) => v0.create_data_contract_update_transition(data_contract),
        }
    }
}
