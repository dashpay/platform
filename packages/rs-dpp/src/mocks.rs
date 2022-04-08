//! This module contains data structures that are left to be implemented

use crate::prelude::*;

use anyhow::Result as AnyResult;
use serde_json::Value as JsonValue;

use async_trait::async_trait;
use thiserror::Error;

use crate::state_repository::StateRepositoryLike;

#[derive(Debug, Clone)]
pub struct DashPlatformProtocol {}
impl DashPlatformProtocol {
    pub fn get_protocol_version(&self) -> u32 {
        unimplemented!()
    }
}

#[derive(Debug, Clone)]
pub struct ValidateDataContract {}
impl ValidateDataContract {}

#[derive(Debug, Clone)]
pub struct DecodeProtocolIdentity {}
impl DecodeProtocolIdentity {}

#[derive(Debug, Clone)]
pub struct DocumentTransition {
    pub action: String,
}

#[derive(Error, Debug, Clone)]
pub enum ConsensusError {}

pub type IHeader = String;

pub type InstantLock = String;

pub type SimplifiedSMListStore = String;

pub struct StateRepository {}

#[derive(Debug, Clone)]
pub struct JsonSchemaValidator {}

pub trait JsonSchemaValidatorLike {}

#[async_trait]
impl StateRepositoryLike for StateRepository {
    /// Fetch the Data Contract by ID
    async fn fetch_data_contract(&self, _data_contract_id: &Identifier) -> AnyResult<JsonValue> {
        unimplemented!()
    }

    /// Store Data Contract
    async fn store_data_contract(&self, _data_contract: DataContract) -> AnyResult<()> {
        unimplemented!()
    }

    /// Fetch Documents by Data Contract Id and type
    async fn fetch_documents(
        &self,
        _contract_id: &Identifier,
        _data_contract_type: &str,
    ) -> AnyResult<Vec<Document>> {
        unimplemented!()
    }

    /// Store Document
    async fn store_document(&self, _document: &Document) -> AnyResult<()> {
        unimplemented!()
    }

    /// Remove Document
    async fn remove_document(
        &self,
        _contract_id: &Identifier,
        _data_contract_type: &str,
        _document_id: &Identifier,
    ) -> AnyResult<()> {
        unimplemented!()
    }

    /// Fetch transaction by ID
    async fn fetch_transaction(&self, _id: &str) -> AnyResult<JsonValue> {
        unimplemented!()
    }

    /// Fetch Identity by ID
    async fn fetch_identity(&self, _id: &Identifier) -> AnyResult<JsonValue> {
        unimplemented!()
    }

    /// Store Public Key hashes and Identity id pair
    async fn store_identity_public_key_hashes(
        &self,
        _identity_id: &Identifier,
        _public_key_hashes: Vec<Vec<u8>>,
    ) -> AnyResult<()> {
        unimplemented!()
    }

    /// Fetch Identity Ids by Public Key hashes
    async fn fetch_identity_by_public_key_hashes(
        &self,
        _public_key_hashed: &[&[u8]],
    ) -> AnyResult<Vec<Identifier>> {
        unimplemented!()
    }

    /// Fetch latest platform block header
    async fn fetch_latest_platform_block_header(&self) -> AnyResult<JsonValue> {
        unimplemented!()
    }

    /// Verify Instant Lock
    async fn verify_instant_lock(&self, _instant_lock: &InstantLock) -> AnyResult<bool> {
        unimplemented!()
    }

    /// Check if AssetLock Transaction outPoint exists in spent list
    async fn is_asset_lock_transaction_out_point_already_used(
        &self,
        _out_point_buffer: &[u8],
    ) -> AnyResult<bool> {
        unimplemented!()
    }

    /// Store AssetLock Transaction outPoint in spent list
    async fn mark_asset_lock_transaction_out_point_as_used(
        &self,
        _out_point_buffer: &[u8],
    ) -> AnyResult<()> {
        unimplemented!()
    }

    /// Fetch Simplified Masternode List Store
    async fn fetch_sml_store(&self) -> AnyResult<JsonValue> {
        unimplemented!()
    }
}
