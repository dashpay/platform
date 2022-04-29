use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::{
    data_contract::DataContract, document::Document, identifier::Identifier, identity::Identity,
    mocks,
};

use anyhow::Result as AnyResult;

#[async_trait]
pub trait StateRepositoryLike {
    /// Fetch the Data Contract by ID
    async fn fetch_data_contract(data_contract_id: &Identifier) -> AnyResult<DataContract>;

    /// Store Data Contract
    async fn store_data_contract(data_contract: DataContract) -> AnyResult<()>;

    /// Fetch Documents by Data Contract Id and type
    async fn fetch_documents(
        contract_id: &Identifier,
        data_contract_type: &str,
    ) -> AnyResult<Vec<Document>>;

    /// Store Document
    async fn store_document(document: &Document) -> AnyResult<()>;

    /// Remove Document
    async fn remove_document(
        contract_id: &Identifier,
        data_contract_type: &str,
        document_id: &Identifier,
    ) -> AnyResult<()>;
    /// Fetch transaction by ID
    /**
     * Fetch transaction by ID
     *
     * @async
     * @method
     * @name StateRepository#fetchTransaction
     * @param {string} id
     * @returns {Promise<Object|null>}
     */
    // TODO shouldn't we use ['dashcore::Transaction'] instead?
    async fn fetch_transaction(id: &str) -> AnyResult<JsonValue>;

    /// Fetch Identity by ID
    async fn fetch_identity(id: &Identifier) -> AnyResult<Identity>;

    /// Store Public Key hashes and Identity id pair
    async fn store_identity_public_key_hashes(
        identity_id: &Identifier,
        public_key_hashes: Vec<Vec<u8>>,
    ) -> AnyResult<()>;

    /// Fetch Identity Ids by Public Key hashes
    async fn fetch_identity_by_public_key_hashes(
        public_key_hashed: &[&[u8]],
    ) -> AnyResult<Vec<Identifier>>;

    /// Fetch latest platform block header
    async fn fetch_latest_platform_block_header() -> AnyResult<mocks::IHeader>;

    /// Verify Instant Lock
    async fn verify_instant_lock(instant_lock: &mocks::InstantLock) -> AnyResult<bool>;

    /// Check if AssetLock Transaction outPoint exists in spent list
    async fn is_asset_lock_transaction_out_point_already_used(
        out_point_buffer: &[u8],
    ) -> AnyResult<bool>;

    /// Store AssetLock Transaction outPoint in spent list
    async fn mark_asset_lock_transaction_out_point_as_used(
        out_point_buffer: &[u8],
    ) -> AnyResult<()>;

    /// Fetch Simplified Masternode List Store
    async fn fetch_sml_store() -> AnyResult<mocks::SimplifiedSMListStore>;
}
