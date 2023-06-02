mod v0;

use crate::data_contract::contract_config::ContractConfigV0;
use crate::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::data_contract::CreatedDataContract;
use crate::data_contract::DataContract;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
pub use v0::*;

/// # Data Contract Factory
///
/// This module is responsible for creating instances of data contracts.
///
/// ## Versioning
///
/// The factory is versioned because the process of creating data contracts
/// can change over time. Changes may be due to modifications in contract
/// requirements, alterations in the contract structure, or evolution in the
/// dependencies of the contract. Versioning allows for these changes to be
/// tracked and managed effectively, providing flexibility to handle different
/// versions of data contracts as needed.
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
        config: Option<Value>,
        definitions: Option<Value>,
    ) -> Result<CreatedDataContract, ProtocolError> {
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
