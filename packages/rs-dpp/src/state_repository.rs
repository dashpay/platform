use async_trait::async_trait;
use serde_json::Value as JsonValue;

use crate::{data_contract::DataContract, document::Document, identifier::Identifier, mocks};

use anyhow::Result as AnyResult;

#[async_trait]
pub trait StateRepositoryLike<S, L>
where
    L: SimplifiedMNListLike,
    S: SMLStoreLike<L>,
{
    /// Fetch the Data Contract by ID
    async fn fetch_data_contract(&self, data_contract_id: &Identifier) -> AnyResult<JsonValue>;

    /// Store Data Contract
    async fn store_data_contract(&self, data_contract: DataContract) -> AnyResult<()>;

    /// Fetch Documents by Data Contract Id and type
    async fn fetch_documents(
        &self,
        contract_id: &Identifier,
        data_contract_type: &str,
        where_query: JsonValue,
    ) -> AnyResult<Vec<Document>>;

    /// Store Document
    async fn store_document(&self, document: &Document) -> AnyResult<()>;

    /// Remove Document
    async fn remove_document(
        &self,
        contract_id: &Identifier,
        data_contract_type: &str,
        document_id: &Identifier,
    ) -> AnyResult<()>;

    async fn fetch_transaction(&self, id: &str) -> AnyResult<JsonValue>;

    /// Fetch Identity by ID
    async fn fetch_identity(&self, id: &Identifier) -> AnyResult<JsonValue>;

    /// Store Public Key hashes and Identity id pair
    async fn store_identity_public_key_hashes(
        &self,
        identity_id: &Identifier,
        public_key_hashes: Vec<Vec<u8>>,
    ) -> AnyResult<()>;

    /// Fetch Identity Ids by Public Key hashes
    async fn fetch_identity_by_public_key_hashes(
        &self,
        public_key_hashed: &[&[u8]],
    ) -> AnyResult<Vec<Identifier>>;

    /// Fetch latest platform block header
    async fn fetch_latest_platform_block_header(&self) -> AnyResult<JsonValue>;

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
    async fn fetch_sml_store(&self) -> AnyResult<S>;
}

pub trait SMLStoreLike<L>
where
    L: SimplifiedMNListLike,
{
    fn get_sml_by_height(&self) -> AnyResult<L> {
        unimplemented!()
    }

    fn get_current_sml(&self) -> AnyResult<L> {
        unimplemented!()
    }
}

pub trait SimplifiedMNListLike {
    fn get_valid_master_nodes(&self) -> Vec<SMLEntry> {
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
