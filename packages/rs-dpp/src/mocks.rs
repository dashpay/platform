//! This module contains data structures that are left to be implemented

use crate::{
    errors::{consensus::basic::BasicError, StateError},
    prelude::*,
};

use crate::validation::ValidationResult;
use anyhow::Result as AnyResult;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use async_trait::async_trait;
use thiserror::Error;

use crate::state_repository::StateRepositoryLike;

#[derive(Debug, Clone)]
pub struct DashPlatformProtocol<SR> {
    pub state_repository: SR,
}
impl<SR> DashPlatformProtocol<SR> {
    pub fn new(state_repository: SR) -> Self {
        DashPlatformProtocol { state_repository }
    }

    pub fn get_protocol_version(&self) -> u32 {
        1
    }

    pub fn get_state_repository(&self) -> &SR {
        &self.state_repository
    }
}

#[derive(Debug, Clone, Default)]
pub struct ValidateDataContract {}
impl ValidateDataContract {
    pub async fn validate_data_contract(&self, raw_data_contract: &JsonValue) -> ValidationResult {
        ValidationResult::default()
    }
}

#[derive(Debug, Clone)]
pub struct DecodeProtocolIdentity {}
impl DecodeProtocolIdentity {}

#[derive(Debug, Clone)]
pub struct DocumentTransition {
    pub action: String,
}

#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error(transparent)]
    StateError(Box<StateError>),
    #[error(transparent)]
    BasicError(Box<BasicError>),

    #[error("Parsing of serialized object failed due to: {parsing_error}")]
    SerializedObjectParsingError { parsing_error: anyhow::Error },

    #[error("Can't read protocol version from serialized object: {parsing_error}")]
    ProtocolVersionParsingError { parsing_error: anyhow::Error },
}

impl From<StateError> for ConsensusError {
    fn from(se: StateError) -> Self {
        ConsensusError::StateError(Box::new(se))
    }
}

impl From<BasicError> for ConsensusError {
    fn from(se: BasicError) -> Self {
        ConsensusError::BasicError(Box::new(se))
    }
}

pub type IHeader = String;

pub type InstantLock = String;

#[derive(Debug, Clone)]
pub struct JsonSchemaValidator {}

pub trait JsonSchemaValidatorLike {}

pub struct StateTransition {
    pub data_contract: DataContract,
}

pub struct DocumentsBatchTransition {}

pub struct SimplifiedMNList {}
impl SimplifiedMNList {
    pub fn get_valid_master_nodes(&self) -> Vec<SMLEntry> {
        unimplemented!()
    }
}

pub struct SMLEntry {
    pub pro_reg_tx_hash: String,
    pub confirmed_hash: String,
    pub service: String,
    pub pub_key_operator: String,
    pub voting_address: String,
    pub is_valid: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SMLStore {}

impl SMLStore {
    pub fn get_sml_by_height(&self) -> AnyResult<SimplifiedMNList> {
        unimplemented!()
    }

    pub fn get_current_sml(&self) -> AnyResult<SimplifiedMNList> {
        unimplemented!()
    }
}
