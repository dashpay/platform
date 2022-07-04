//! This module contains data structures that are left to be implemented

use crate::state_transition::{StateTransitionLike, StateTransitionType};
use crate::{prelude::*, state_transition::StateTransitionConvert};

use crate::validation::ValidationResult;
use anyhow::Result as AnyResult;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

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

#[derive(Debug, Clone)]
pub struct DecodeProtocolIdentity {}
impl DecodeProtocolIdentity {}

pub type IHeader = String;

pub type InstantLock = String;

#[derive(Debug, Clone)]
pub struct JsonSchemaValidator {}

pub trait JsonSchemaValidatorLike {}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityCreateTransition {}
impl StateTransitionConvert for IdentityCreateTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        unimplemented!()
    }
    fn identifiers_property_paths() -> Vec<&'static str> {
        unimplemented!()
    }
    fn binary_property_paths() -> Vec<&'static str> {
        unimplemented!()
    }
}
impl StateTransitionLike for IdentityCreateTransition {
    fn get_protocol_version(&self) -> u32 {
        unimplemented!()
    }
    /// returns the type of State Transition
    fn get_type(&self) -> StateTransitionType {
        unimplemented!()
    }
    /// returns the signature as a byte-array
    fn get_signature(&self) -> &Vec<u8> {
        unimplemented!()
    }
    /// set a new signature
    fn set_signature(&mut self, _signature: Vec<u8>) {
        unimplemented!()
    }
    fn calculate_fee(&self) -> Result<u64, ProtocolError> {
        unimplemented!()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityTopUpTransition {}
impl StateTransitionConvert for IdentityTopUpTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        unimplemented!()
    }
    fn identifiers_property_paths() -> Vec<&'static str> {
        unimplemented!()
    }
    fn binary_property_paths() -> Vec<&'static str> {
        unimplemented!()
    }
}

impl StateTransitionLike for IdentityTopUpTransition {
    fn get_protocol_version(&self) -> u32 {
        unimplemented!()
    }
    /// returns the type of State Transition
    fn get_type(&self) -> StateTransitionType {
        unimplemented!()
    }
    /// returns the signature as a byte-array
    fn get_signature(&self) -> &Vec<u8> {
        unimplemented!()
    }
    /// set a new signature
    fn set_signature(&mut self, _signature: Vec<u8>) {
        unimplemented!()
    }
    fn calculate_fee(&self) -> Result<u64, ProtocolError> {
        unimplemented!()
    }
}

pub struct FetchAndValidateDataContract {}
