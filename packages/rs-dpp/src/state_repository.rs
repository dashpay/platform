use anyhow::Result as AnyResult;
use async_trait::async_trait;
use dashcore::InstantLock;
#[cfg(test)]
use mockall::{automock, predicate::*};
use serde_json::Value as JsonValue;

use crate::{
    prelude::*,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
};

#[cfg_attr(test, automock)]
#[async_trait]
pub trait StateRepositoryLike: Send + Sync {
    /// Fetch the Data Contract by ID
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`DataContract`] should be also possible
    async fn fetch_data_contract<T>(
        &self,
        data_contract_id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> AnyResult<T>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static;

    /// Store Data Contract
    async fn store_data_contract(
        &self,
        data_contract: DataContract,
        execution_context: &StateTransitionExecutionContext,
    ) -> AnyResult<()>;

    /// Fetch Documents by Data Contract Id and type
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`Document`] should be also possible
    async fn fetch_documents<T>(
        &self,
        contract_id: &Identifier,
        data_contract_type: &str,
        where_query: JsonValue,
        execution_context: &StateTransitionExecutionContext,
    ) -> AnyResult<Vec<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static;

    /// Create Document
    async fn create_document(
        &self,
        document: &Document,
        execution_context: &StateTransitionExecutionContext,
    ) -> AnyResult<()>;

    /// Update Document
    async fn update_document(
        &self,
        document: &Document,
        execution_context: &StateTransitionExecutionContext,
    ) -> AnyResult<()>;

    /// Remove Document
    async fn remove_document(
        &self,
        data_contract: &DataContract,
        data_contract_type: &str,
        document_id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> AnyResult<()>;

    /// Fetch the Transaction
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`Transaction`] should be also possible
    async fn fetch_transaction<T>(
        &self,
        id: &str,
        execution_context: &StateTransitionExecutionContext,
    ) -> AnyResult<Option<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static;

    /// Fetch Identity by ID
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`Identity`] should be also possible
    async fn fetch_identity<T>(
        &self,
        id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> AnyResult<Option<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static;

    /// Store Public Key hashes and Identity id pair
    async fn store_identity_public_key_hashes(
        &self,
        identity_id: &Identifier,
        public_key_hashes: Vec<Vec<u8>>,
        execution_context: &StateTransitionExecutionContext,
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
    async fn verify_instant_lock(
        &self,
        instant_lock: &InstantLock,
        execution_context: &StateTransitionExecutionContext,
    ) -> AnyResult<bool>;

    /// Check if AssetLock Transaction outPoint exists in spent list
    async fn is_asset_lock_transaction_out_point_already_used(
        &self,
        out_point_buffer: &[u8],
        execution_context: &StateTransitionExecutionContext,
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

    /// Create an identity
    async fn create_identity(
        &self,
        identity: &Identity,
        execution_context: &StateTransitionExecutionContext,
    ) -> AnyResult<()>;

    /// Update an identity
    async fn update_identity(
        &self,
        identity: &Identity,
        execution_context: &StateTransitionExecutionContext,
    ) -> AnyResult<()>;

    // Get latest (in a queue) withdrawal transaction index
    async fn fetch_latest_withdrawal_transaction_index(&self) -> AnyResult<u64>;

    // Enqueue withdrawal transaction
    async fn enqueue_withdrawal_transaction(
        &self,
        index: u64,
        transaction_bytes: Vec<u8>,
    ) -> AnyResult<()>;
}
