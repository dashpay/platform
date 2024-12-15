use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Forbidden operation '{operation}' on '{field_path}', old config is {old_config}, new config is {new_config}")]
#[platform_serialize(unversioned)]
pub struct DataContractTokenConfigurationUpdateError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    operation: String,
    field_path: String,
    old_config: TokenConfiguration,
    new_config: TokenConfiguration,
}

impl DataContractTokenConfigurationUpdateError {
    pub fn new(
        operation: String,
        field_path: String,
        old_config: TokenConfiguration,
        new_config: TokenConfiguration,
    ) -> Self {
        Self {
            operation,
            field_path,
            old_config,
            new_config,
        }
    }

    pub fn operation(&self) -> String {
        self.operation.clone()
    }

    pub fn field_path(&self) -> String {
        self.field_path.clone()
    }

    pub fn old_config(&self) -> TokenConfiguration {
        self.old_config.clone()
    }

    pub fn new_config(&self) -> TokenConfiguration {
        self.new_config.clone()
    }
}

impl From<DataContractTokenConfigurationUpdateError> for ConsensusError {
    fn from(err: DataContractTokenConfigurationUpdateError) -> Self {
        Self::BasicError(BasicError::DataContractTokenConfigurationUpdateError(err))
    }
}
