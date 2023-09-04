mod v0;

use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::data_contract::DataContract;

use crate::util::entropy_generator::EntropyGenerator;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use derive_more::From;
use platform_value::{Identifier, Value};

use crate::data_contract::config::DataContractConfig;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
#[cfg(all(feature = "state-transitions", feature = "client"))]
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
pub use v0::DataContractFactoryV0;

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
#[derive(From)]
pub enum DataContractFactory {
    /// The version 0 implementation of the data contract factory.
    V0(DataContractFactoryV0),
}

impl DataContractFactory {
    /// Create a new data contract factory knowing versions
    pub fn new(
        protocol_version: u32,
        entropy_generator: Option<Box<dyn EntropyGenerator>>,
    ) -> Result<Self, ProtocolError> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        match platform_version
            .platform_architecture
            .data_contract_factory_structure_version
        {
            0 => Ok(DataContractFactoryV0::new(protocol_version, entropy_generator).into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractFactory::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    /// Create a DataContract
    pub fn create(
        &self,
        owner_id: Identifier,
        document_schemas: Value,
        config: Option<DataContractConfig>,
        schema_defs: Option<Value>,
    ) -> Result<CreatedDataContract, ProtocolError> {
        match self {
            DataContractFactory::V0(v0) => {
                v0.create(owner_id, document_schemas, config, schema_defs)
            }
        }
    }

    #[cfg(feature = "data-contract-value-conversion")]
    /// Create a DataContract from a plain object
    pub fn create_from_object(
        &self,
        data_contract_object: Value,
        #[cfg(feature = "validation")] skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        match self {
            DataContractFactory::V0(v0) => {
                #[cfg(feature = "validation")]
                {
                    v0.create_from_object(data_contract_object, skip_validation)
                }
                #[cfg(not(feature = "validation"))]
                {
                    v0.create_from_object(data_contract_object, true)
                }
            }
        }
    }

    /// Create a DataContract from a buffer
    pub fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        #[cfg(feature = "validation")] skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        match self {
            DataContractFactory::V0(v0) => {
                #[cfg(feature = "validation")]
                {
                    v0.create_from_buffer(buffer, skip_validation)
                }
                #[cfg(not(feature = "validation"))]
                {
                    v0.create_from_buffer(buffer)
                }
            }
        }
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    /// Create a DataContractCreateTransition
    pub fn create_data_contract_create_transition(
        &self,
        created_data_contract: CreatedDataContract,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        match self {
            DataContractFactory::V0(v0) => {
                v0.create_unsigned_data_contract_create_transition(created_data_contract)
            }
        }
    }

    #[cfg(all(feature = "state-transitions", feature = "client"))]
    /// Create a DataContractUpdateTransition
    pub fn create_data_contract_update_transition(
        &self,
        data_contract: DataContract,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        match self {
            DataContractFactory::V0(v0) => {
                v0.create_unsigned_data_contract_update_transition(data_contract)
            }
        }
    }
}
