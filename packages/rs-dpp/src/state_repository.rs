use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::mocks;
use crate::prelude::*;

use anyhow::Result as AnyResult;

#[cfg(test)]
use mockall::{automock, predicate::*};

#[cfg_attr(test, automock)]
#[async_trait]
pub trait StateRepositoryLike: Send + Sync {
    /// Fetch the Data Contract by ID
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`DataContract`] should be also possible
    async fn fetch_data_contract<T>(&self, data_contract_id: &Identifier) -> AnyResult<T>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static;

    /// Store Data Contract
    async fn store_data_contract(&self, data_contract: DataContract) -> AnyResult<()>;

    /// Fetch Documents by Data Contract Id and type
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`Document`] should be also possible
    async fn fetch_documents<T>(
        &self,
        contract_id: &Identifier,
        data_contract_type: &str,
        where_query: JsonValue,
    ) -> AnyResult<Vec<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static;

    /// Store Document
    async fn store_document(&self, document: &Document) -> AnyResult<()>;

    /// Remove Document
    async fn remove_document(
        &self,
        contract_id: &Identifier,
        data_contract_type: &str,
        document_id: &Identifier,
    ) -> AnyResult<()>;

    /// Fetch the Transaction
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`Transaction`] should be also possible
    async fn fetch_transaction<T>(&self, id: &str) -> AnyResult<Option<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static;

    /// Fetch Identity by ID
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`Identity`] should be also possible
    async fn fetch_identity<T>(&self, id: &Identifier) -> AnyResult<Option<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static;

    /// Store Public Key hashes and Identity id pair
    async fn store_identity_public_key_hashes(
        &self,
        identity_id: &Identifier,
        public_key_hashes: Vec<Vec<u8>>,
    ) -> AnyResult<()>;

    /// Fetch Identity Ids by Public Key hashes
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`Identity`] should be also possible
    async fn fetch_identity_by_public_key_hashes<T>(
        &self,
        public_key_hashed: Vec<Vec<u8>>,
    ) -> AnyResult<Vec<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static;

    /// Fetch latest platform block header
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`serde_json::Value`] should be also possible
    async fn fetch_latest_platform_block_header<T>(&self) -> AnyResult<T>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static;

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
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`mocks::SMLStore`] should be also possible
    async fn fetch_sml_store<T>(&self) -> AnyResult<T>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static;
}
