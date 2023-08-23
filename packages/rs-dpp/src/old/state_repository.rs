use std::convert::{Infallible, TryInto};

use anyhow::Result as AnyResult;
use async_trait::async_trait;
use dashcore::InstantLock;
#[cfg(feature = "fixtures-and-mocks")]
use mockall::{automock, predicate::*};
use platform_value::Value;

use crate::document::Document;
use crate::identity::KeyID;
use crate::{
    prelude::*,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
};

impl From<Infallible> for ProtocolError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

#[derive(Clone, Default)]
pub struct FetchTransactionResponse {
    pub height: Option<u32>,
    pub data: Option<Vec<u8>>,
}

// Let StateRepositoryLike mock return DataContracts instead of bytes to simplify things a bit.
#[cfg_attr(any(test, feature="fixtures-and-mocks"), automock(
    type ConversionError=Infallible;
    type FetchDataContract=DataContract;
    type FetchIdentity=Identity;
    type FetchTransaction=FetchTransactionResponse;
    type FetchDocument=Document;
    type FetchExtendedDocument=ExtendedDocument;
))]
#[async_trait(?Send)]
pub trait StateRepositoryLike: Sync {
    type ConversionError: Into<ProtocolError>;
    type FetchDataContract: TryInto<DataContract, Error = Self::ConversionError> + Send;
    type FetchDocument: TryInto<Document, Error = Self::ConversionError>;
    type FetchExtendedDocument: TryInto<ExtendedDocument, Error = Self::ConversionError>;
    type FetchIdentity: TryInto<Identity, Error = Self::ConversionError>;
    type FetchTransaction: TryInto<FetchTransactionResponse, Error = Self::ConversionError>
        + Send
        + Sync;

    /// Fetch the Data Contract by ID
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`DataContract`] should be also possible
    async fn fetch_data_contract<'a>(
        &self,
        data_contract_id: &Identifier,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Option<Self::FetchDataContract>>;

    /// Store Data Contract
    async fn create_data_contract<'a>(
        &self,
        data_contract: DataContract,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    async fn update_data_contract<'a>(
        &self,
        data_contract: DataContract,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    /// Fetch Documents by Data Contract Id and type
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`Document`] should be also possible
    async fn fetch_documents<'a>(
        &self,
        contract_id: &Identifier,
        data_contract_type: &str,
        where_query: Value,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Vec<Self::FetchDocument>>;

    /// Fetch Documents by Data Contract Id and type
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`ExtendedDocument`] should be also possible
    async fn fetch_extended_documents<'a>(
        &self,
        contract_id: &Identifier,
        data_contract_type: &str,
        where_query: Value,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Vec<Self::FetchExtendedDocument>>;

    /// Create Document
    async fn create_document<'a>(
        &self,
        document: &ExtendedDocument,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    /// Update Document
    async fn update_document<'a>(
        &self,
        document: &ExtendedDocument,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    /// Remove Document
    async fn remove_document<'a>(
        &self,
        data_contract: &DataContract,
        data_contract_type: &str,
        document_id: &Identifier,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    /// Fetch the Transaction
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`Transaction`] should be also possible
    async fn fetch_transaction<'a>(
        &self,
        id: &str,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Self::FetchTransaction>;

    /// Fetch Identity by ID
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`Identity`] should be also possible
    async fn fetch_identity<'a>(
        &self,
        id: &Identifier,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Option<Self::FetchIdentity>>;

    /// Create an identity
    async fn create_identity<'a>(
        &self,
        identity: &Identity,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    /// Add keys to identity
    async fn add_keys_to_identity<'a>(
        &self,
        identity_id: &Identifier,
        keys: &[IdentityPublicKey],
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    /// Disable identity keys
    async fn disable_identity_keys<'a>(
        &self,
        identity_id: &Identifier,
        keys: &[KeyID],
        disable_at: TimestampMillis,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    /// Update identity revision
    async fn update_identity_revision<'a>(
        &self,
        identity_id: &Identifier,
        revision: Revision,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    /// Fetch identity balance by identity ID
    async fn fetch_identity_balance<'a>(
        &self,
        identity_id: &Identifier,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Option<u64>>; // TODO we should use Credits type

    /// Fetch identity balance including debt by identity ID
    async fn fetch_identity_balance_with_debt<'a>(
        &self,
        identity_id: &Identifier,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Option<i64>>; // TODO we should use SignedCredits type

    /// Add to identity balance
    async fn add_to_identity_balance<'a>(
        &self,
        identity_id: &Identifier,
        amount: u64, // TODO we should use Credits type
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    /// Remove from identity balance
    async fn remove_from_identity_balance<'a>(
        &self,
        identity_id: &Identifier,
        amount: u64, // TODO we should use Credits type
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    /// Add to system credits
    async fn add_to_system_credits<'a>(
        &self,
        amount: u64, // TODO we should use Credits type
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    /// Remove from system credits
    async fn remove_from_system_credits<'a>(
        &self,
        amount: u64, // TODO we should use Credits type
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    async fn fetch_latest_platform_block_header(&self) -> AnyResult<Vec<u8>>;

    /// Verify Instant Lock
    async fn verify_instant_lock<'a>(
        &self,
        instant_lock: &InstantLock,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<bool>;

    /// Check if AssetLock Transaction outPoint exists in spent list
    async fn is_asset_lock_transaction_out_point_already_used<'a>(
        &self,
        out_point_buffer: &[u8],
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<bool>;

    /// Store AssetLock Transaction outPoint in spent list
    async fn mark_asset_lock_transaction_out_point_as_used<'a>(
        &self,
        out_point_buffer: &[u8],
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()>;

    /// Fetch Simplified Masternode List Store
    /// By default, the method should return data as bytes (`Vec<u8>`), but the deserialization to [`mocks::SMLStore`] should be also possible
    async fn fetch_sml_store<T>(&self) -> AnyResult<T>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static;

    /// Check if AssetLock Transaction outPoint exists in spent list
    async fn is_in_the_valid_master_nodes_list(
        &self,
        out_point_buffer: [u8; 32],
    ) -> AnyResult<bool>;

    // Get latest (in a queue) withdrawal transaction index
    async fn fetch_latest_withdrawal_transaction_index(&self) -> AnyResult<u64>;

    // Get latest (in a queue) withdrawal transaction index
    async fn fetch_latest_platform_core_chain_locked_height(&self) -> AnyResult<Option<u32>>;

    // Enqueue withdrawal transaction
    async fn enqueue_withdrawal_transaction(
        &self,
        index: u64,
        transaction_bytes: Vec<u8>,
    ) -> AnyResult<()>;

    // Fetch latest platform block time
    async fn fetch_latest_platform_block_time(&self) -> AnyResult<u64>;

    // Get latest platform block height
    async fn fetch_latest_platform_block_height(&self) -> AnyResult<u64>;
}
