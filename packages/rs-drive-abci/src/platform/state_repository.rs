use crate::platform::Platform;
use dashcore::anyhow::Result;
use dashcore::InstantLock;
use dpp::data_contract::DataContract;
use dpp::document::{Document, ExtendedDocument};
use dpp::identifier::Identifier;
use dpp::identity::{Identity, IdentityPublicKey, KeyID};
use dpp::prelude::{Revision, TimestampMillis};
use dpp::state_repository::{FetchTransactionResponse, StateRepositoryLike};
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use serde::Deserialize;
use std::convert::Infallible;
use anyhow::Result as AnyResult;
use dpp::platform_value::Value;
use dpp::async_trait::async_trait;

#[async_trait(?Send)]
impl<CoreRPCLike : Sync> StateRepositoryLike for Platform<CoreRPCLike> {
    type ConversionError = Infallible;
    type FetchDataContract = DataContract;
    type FetchDocument = Document;
    type FetchExtendedDocument = ExtendedDocument;
    type FetchIdentity = Identity;
    type FetchTransaction = FetchTransactionResponse;

    async fn fetch_data_contract<'a>(
        &self,
        data_contract_id: &Identifier,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Option<Self::FetchDataContract>> {
        todo!()
    }

    async fn store_data_contract<'a>(
        &self,
        data_contract: DataContract,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn fetch_documents<'a>(
        &self,
        contract_id: &Identifier,
        data_contract_type: &str,
        where_query: Value,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Vec<Self::FetchDocument>> {
        todo!()
    }

    async fn fetch_extended_documents<'a>(
        &self,
        contract_id: &Identifier,
        data_contract_type: &str,
        where_query: Value,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Vec<Self::FetchExtendedDocument>> {
        todo!()
    }

    async fn create_document<'a>(
        &self,
        document: &ExtendedDocument,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn update_document<'a>(
        &self,
        document: &ExtendedDocument,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn remove_document<'a>(
        &self,
        data_contract: &DataContract,
        data_contract_type: &str,
        document_id: &Identifier,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn fetch_transaction<'a>(
        &self,
        id: &str,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Self::FetchTransaction> {
        todo!()
    }

    async fn fetch_identity<'a>(
        &self,
        id: &Identifier,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Option<Self::FetchIdentity>> {
        todo!()
    }

    async fn create_identity<'a>(
        &self,
        identity: &Identity,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn add_keys_to_identity<'a>(
        &self,
        identity_id: &Identifier,
        keys: &[IdentityPublicKey],
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn disable_identity_keys<'a>(
        &self,
        identity_id: &Identifier,
        keys: &[KeyID],
        disable_at: TimestampMillis,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn update_identity_revision<'a>(
        &self,
        identity_id: &Identifier,
        revision: Revision,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn fetch_identity_balance<'a>(
        &self,
        identity_id: &Identifier,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Option<u64>> {
        todo!()
    }

    async fn fetch_identity_balance_with_debt<'a>(
        &self,
        identity_id: &Identifier,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<Option<i64>> {
        todo!()
    }

    async fn add_to_identity_balance<'a>(
        &self,
        identity_id: &Identifier,
        amount: u64,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn remove_from_identity_balance<'a>(
        &self,
        identity_id: &Identifier,
        amount: u64,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn add_to_system_credits<'a>(
        &self,
        amount: u64,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn remove_from_system_credits<'a>(
        &self,
        amount: u64,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn fetch_latest_platform_block_header(&self) -> AnyResult<Vec<u8>> {
        todo!()
    }

    async fn verify_instant_lock<'a>(
        &self,
        instant_lock: &InstantLock,
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<bool> {
        todo!()
    }

    async fn is_asset_lock_transaction_out_point_already_used<'a>(
        &self,
        out_point_buffer: &[u8],
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<bool> {
        todo!()
    }

    async fn mark_asset_lock_transaction_out_point_as_used<'a>(
        &self,
        out_point_buffer: &[u8],
        execution_context: Option<&'a StateTransitionExecutionContext>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn fetch_sml_store<T>(&self) -> AnyResult<T>
    where
        T: for<'de> Deserialize<'de> + 'static,
    {
        todo!()
    }

    async fn fetch_latest_withdrawal_transaction_index(&self) -> AnyResult<u64> {
        todo!()
    }

    async fn fetch_latest_platform_core_chain_locked_height(&self) -> AnyResult<Option<u32>> {
        todo!()
    }

    async fn enqueue_withdrawal_transaction(
        &self,
        index: u64,
        transaction_bytes: Vec<u8>,
    ) -> AnyResult<()> {
        todo!()
    }

    async fn fetch_latest_platform_block_time(&self) -> AnyResult<u64> {
        todo!()
    }

    async fn fetch_latest_platform_block_height(&self) -> AnyResult<u64> {
        todo!()
    }
}
