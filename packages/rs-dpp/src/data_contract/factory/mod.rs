mod v0;

use crate::consensus::basic::UnsupportedVersionError;
use crate::consensus::ConsensusError;
use crate::data_contract::contract_config::ContractConfigV0;
use crate::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::data_contract::CreatedDataContract;
use crate::data_contract::DataContract;
use crate::util::deserializer::ProtocolVersion;
use crate::util::entropy_generator::EntropyGenerator;
use crate::version::{FeatureVersion, PlatformVersion};
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
    /// Create a new data contract factory knowing versions
    pub fn new(
        version: FeatureVersion,
        preferred_default_data_contract_version: FeatureVersion,
        entropy_generator: Option<Box<dyn EntropyGenerator>>,
    ) -> Result<Self, ProtocolError> {
        match version {
            0 => Ok(DataContractFactoryV0::new(
                preferred_default_data_contract_version,
                entropy_generator,
            )
            .into()),
            version => Err(ProtocolError::UnknownVersionError(format!(
                "version {version} not known for data contract factory"
            ))),
        }
    }

    /// Create a new data contract factory knowing the protocol version
    /// The preferred_default_data_contract_factory_version and the preferred_default_data_contract_version
    /// can be given. If they are they must be valid for the protocol version.
    pub fn new_from_protocol_version(
        protocol_version: u32,
        preferred_default_data_contract_factory_version: Option<FeatureVersion>,
        preferred_default_data_contract_version: Option<FeatureVersion>,
        entropy_generator: Option<Box<dyn EntropyGenerator>>,
    ) -> Result<Self, ProtocolError> {
        let platform_version = PlatformVersion::get(protocol_version)?;
        if let Some(preferred_default_data_contract_factory_version) =
            preferred_default_data_contract_factory_version
        {
            if !platform_version
                .platform_architecture
                .data_contract_factory
                .check_version(preferred_default_data_contract_factory_version)
            {
                // we are asking for a data contract factory version that isn't supported by the protocol version
                return Err(ConsensusError::BasicError(
                    UnsupportedVersionError::new(
                        preferred_default_data_contract_factory_version,
                        platform_version
                            .platform_architecture
                            .data_contract_factory
                            .min_version,
                        platform_version
                            .platform_architecture
                            .data_contract_factory
                            .max_version,
                    )
                    .into(),
                )
                .into());
            } else if let Some(preferred_default_data_contract_version) =
                preferred_default_data_contract_version
            {
                if !platform_version
                    .contract
                    .check_version(preferred_default_data_contract_version)
                {
                    // we are asking for a data contract factory version that isn't supported by the protocol version
                    return Err(ConsensusError::BasicError(
                        UnsupportedVersionError::new(
                            preferred_default_data_contract_version,
                            platform_version.contract.min_version,
                            platform_version.contract.max_version,
                        )
                        .into(),
                    )
                    .into());
                } else {
                    DataContractFactory::new(
                        preferred_default_data_contract_factory_version,
                        preferred_default_data_contract_version,
                        entropy_generator,
                    )
                }
            } else {
                DataContractFactory::new(
                    preferred_default_data_contract_factory_version,
                    platform_version.contract.default_current_version,
                    entropy_generator,
                )
            }
        } else {
            DataContractFactory::new(
                platform_version
                    .platform_architecture
                    .data_contract_factory
                    .default_current_version,
                platform_version.contract.default_current_version,
                entropy_generator,
            )
        }
    }

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
        created_data_contract: CreatedDataContract,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        match self {
            DataContractFactory::V0(v0) => {
                v0.create_data_contract_create_transition(created_data_contract)
            }
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
