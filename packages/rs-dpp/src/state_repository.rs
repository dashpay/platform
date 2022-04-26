use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::{data_contract::DataContract, document::Document, identifier::Identifier, mocks};

use anyhow::Result as AnyResult;

#[async_trait]
pub trait StateRepositoryLike: Send + Sync {
    /// Fetch the Data Contract by ID
    async fn fetch_data_contract(&self, data_contract_id: &Identifier) -> AnyResult<Vec<u8>>;

    /// Store Data Contract
    async fn store_data_contract(&self, data_contract: DataContract) -> AnyResult<()>;

    /// Fetch Documents by Data Contract Id and type
    async fn fetch_documents<D>(
        &self,
        contract_id: &Identifier,
        data_contract_type: &str,
        where_query: JsonValue,
    ) -> AnyResult<Vec<D>>
    where
        D: for<'de> serde::de::Deserialize<'de>;

    /// Store Document
    async fn store_document(&self, document: &Document) -> AnyResult<()>;

    /// Remove Document
    async fn remove_document(
        &self,
        contract_id: &Identifier,
        data_contract_type: &str,
        document_id: &Identifier,
    ) -> AnyResult<()>;

    async fn fetch_transaction(&self, id: &str) -> AnyResult<Vec<u8>>;

    /// Fetch Identity by ID
    async fn fetch_identity(&self, id: &Identifier) -> AnyResult<Vec<u8>>;

    /// Store Public Key hashes and Identity id pair
    async fn store_identity_public_key_hashes(
        &self,
        identity_id: &Identifier,
        public_key_hashes: Vec<Vec<u8>>,
    ) -> AnyResult<()>;

    /// Fetch Identity Ids by Public Key hashes
    async fn fetch_identity_by_public_key_hashes(
        &self,
        public_key_hashed: Vec<Vec<u8>>,
    ) -> AnyResult<Vec<Vec<u8>>>;

    /// Fetch latest platform block header
    async fn fetch_latest_platform_block_header(&self) -> AnyResult<Vec<u8>>;

    /// Verify Instant Lock
    async fn verify_instant_lock(&self, instant_lock: &mocks::InstantLock) -> AnyResult<bool>;

    /// Check if AssetLock Transaction outPoint exists in spent list
    async fn is_asset_lock_transaction_out_point_already_used(
        &self,
        out_point_buffer: &[u8],
    ) -> AnyResult<bool>;

    /// Store AssetLock Transaction outPoint in spent list
    async fn mark_asset_lock_transaction_out_point_as_used(
        &self,
        out_point_buffer: &[u8],
    ) -> AnyResult<()>;

    /// Fetch Simplified Masternode List Store
    async fn fetch_sml_store<T>(&self) -> AnyResult<T>
    where
        T: for<'de> serde::de::Deserialize<'de>;
}
