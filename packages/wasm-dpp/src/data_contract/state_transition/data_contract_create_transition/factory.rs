use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use dpp::{dashcore::InstantLock, data_contract::{DataContract, state_transition::apply_data_contract_create_transition_factory::ApplyDataContractCreateTransition}, document::Document, prelude::{Identifier, Identity}, state_repository::StateRepositoryLike, state_transition::state_transition_execution_context::StateTransitionExecutionContext};
use wasm_bindgen::prelude::*;

use crate::identifier::IdentifierWrapper;

#[wasm_bindgen]
extern "C" {
    type ExternalStateRepositoryLike;

    #[wasm_bindgen(structural, method, js_name=fetchDataContract)]
    pub fn fetch_data_contract(
        this: &ExternalStateRepositoryLike,
        data_contract_id: IdentifierWrapper,
        // execution_context: &StateTransitionExecutionContextWasm,
    ) -> JsValue;
}

/// Wraps external duck-typed thing into box with mutex to ensure it'll stay at the same
/// place in memory and will have synchronized access.
struct ExternalStateRepositoryLikeWrapper(Box<Mutex<ExternalStateRepositoryLike>>);

unsafe impl Send for ExternalStateRepositoryLikeWrapper {}
unsafe impl Sync for ExternalStateRepositoryLikeWrapper {}

#[async_trait]
impl StateRepositoryLike for ExternalStateRepositoryLikeWrapper {
    async fn fetch_data_contract<T>(
        &self,
        data_contract_id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<T>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static,
    {
        todo!()
    }

    async fn store_data_contract(
        &self,
        data_contract: DataContract,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn fetch_documents<T>(
        &self,
        contract_id: &Identifier,
        data_contract_type: &str,
        where_query: serde_json::Value,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Vec<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static,
    {
        todo!()
    }

    async fn create_document(
        &self,
        document: &Document,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn update_document(
        &self,
        document: &Document,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn remove_document(
        &self,
        data_contract: &DataContract,
        data_contract_type: &str,
        document_id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn fetch_transaction<T>(
        &self,
        id: &str,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Option<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static,
    {
        todo!()
    }

    async fn fetch_identity<T>(
        &self,
        id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Option<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static,
    {
        todo!()
    }

    async fn store_identity_public_key_hashes(
        &self,
        identity_id: &Identifier,
        public_key_hashes: Vec<Vec<u8>>,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn fetch_identity_by_public_key_hashes<T>(
        &self,
        public_key_hashed: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static,
    {
        todo!()
    }

    async fn fetch_latest_platform_block_header<T>(&self) -> anyhow::Result<T>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static,
    {
        todo!()
    }

    async fn verify_instant_lock(
        &self,
        instant_lock: &InstantLock,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<bool> {
        todo!()
    }

    async fn is_asset_lock_transaction_out_point_already_used(
        &self,
        out_point_buffer: &[u8],
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<bool> {
        todo!()
    }

    async fn mark_asset_lock_transaction_out_point_as_used(
        &self,
        out_point_buffer: &[u8],
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn fetch_sml_store<T>(&self) -> anyhow::Result<T>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static,
    {
        todo!()
    }

    async fn create_identity(
        &self,
        identity: &Identity,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn update_identity(
        &self,
        identity: &Identity,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn fetch_latest_withdrawal_transaction_index(&self) -> anyhow::Result<u64> {
        todo!()
    }

    async fn enqueue_withdrawal_transaction(
        &self,
        index: u64,
        transaction_bytes: Vec<u8>,
    ) -> anyhow::Result<()> {
        todo!()
    }
}

pub struct ApplyDataContractCreateTransitionWasm(
    ApplyDataContractCreateTransition<ExternalStateRepositoryLikeWrapper>,
);
