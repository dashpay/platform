use crate::data_contract::{DataContract, DataContractFactory};

use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::prelude::{Identifier, IdentityNonce};
#[cfg(feature = "state-transitions")]
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
#[cfg(feature = "state-transitions")]
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;

use crate::ProtocolError;
use platform_value::Value;

/// # Data Contract Facade
///
/// This module acts as a simplified, high-level interface to a more complex
/// body of code. It forwards requests to appropriate subsystems.
///
/// ## Versioning
///
/// In Dash Platform, facades are not versioned because the interface they
/// provide remains stable, even when changes occur in the underlying system.
/// Since these modifications do not affect the facade's interface, versioning
/// is not necessary. The primary function of the facade is to provide a stable
/// API to the rest of the system, effectively isolating consumers of the API
/// from changes in the underlying implementation.
pub struct DataContractFacade {
    factory: DataContractFactory,
}

impl DataContractFacade {
    pub fn new(protocol_version: u32) -> Result<Self, ProtocolError> {
        Ok(Self {
            factory: DataContractFactory::new(protocol_version)?,
        })
    }

    /// Create Data Contract
    pub fn create(
        &self,
        owner_id: Identifier,
        identity_nonce: IdentityNonce,
        documents: Value,
        config: Option<Value>,
        definitions: Option<Value>,
    ) -> Result<CreatedDataContract, ProtocolError> {
        self.factory.create_with_value_config(
            owner_id,
            identity_nonce,
            documents,
            config,
            definitions,
        )
    }

    /// Create Data Contract from plain object
    #[cfg(all(feature = "identity-serialization", feature = "client"))]
    pub fn create_from_object(
        &self,
        raw_data_contract: Value,
        #[cfg(feature = "validation")] skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        self.factory.create_from_object(
            raw_data_contract,
            #[cfg(feature = "validation")]
            skip_validation,
        )
    }

    /// Create Data Contract from buffer
    #[cfg(all(feature = "identity-serialization", feature = "client"))]
    pub fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        #[cfg(feature = "validation")] skip_validation: bool,
    ) -> Result<DataContract, ProtocolError> {
        self.factory.create_from_buffer(
            buffer,
            #[cfg(feature = "validation")]
            skip_validation,
        )
    }

    #[cfg(feature = "state-transitions")]
    /// Create Data Contract Create State Transition
    pub fn create_data_contract_create_transition(
        &self,
        created_data_contract: CreatedDataContract,
    ) -> Result<DataContractCreateTransition, ProtocolError> {
        self.factory
            .create_data_contract_create_transition(created_data_contract)
    }

    #[cfg(feature = "state-transitions")]
    /// Create Data Contract Update State Transition
    pub fn create_data_contract_update_transition(
        &self,
        data_contract: DataContract,
        identity_contract_nonce: IdentityNonce,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        self.factory
            .create_data_contract_update_transition(data_contract, identity_contract_nonce)
    }
}
