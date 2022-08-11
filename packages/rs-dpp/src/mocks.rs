//! This module contains data structures that are left to be implemented

use anyhow::Result as AnyResult;
use serde::{Deserialize, Serialize};

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

pub struct FetchAndValidateDataContract {}
