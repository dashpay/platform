//! Bindings for state repository -like objects coming from JS.

use serde::{Deserialize, Serialize};
use std::{convert::Infallible, pin::Pin, sync::Mutex};

use async_trait::async_trait;
use dpp::dashcore::consensus;
use dpp::{
    dashcore::InstantLock,
    data_contract::DataContract,
    document::Document,
    prelude::{Identifier, Identity},
    state_repository::{
        FetchTransactionResponse as FetchTransactionResponseDPP, StateRepositoryLike,
    },
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
};
use wasm_bindgen::prelude::*;

use crate::{
    identifier::IdentifierWrapper, DataContractWasm, IdentityWasm,
    StateTransitionExecutionContextWasm,
};

#[wasm_bindgen]
extern "C" {
    pub type ExternalStateRepositoryLike;

    #[wasm_bindgen(structural, method, js_name=fetchDataContract)]
    pub fn fetch_data_contract(
        this: &ExternalStateRepositoryLike,
        data_contract_id: IdentifierWrapper,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Option<DataContractWasm>;

    #[wasm_bindgen(structural, method, js_name=storeDataContract)]
    pub fn store_data_contract(
        this: &ExternalStateRepositoryLike,
        data_contract: DataContractWasm,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> JsValue;

    #[wasm_bindgen(structural, method, js_name=fetchIdentity)]
    pub fn fetch_identity(
        this: &ExternalStateRepositoryLike,
        id: IdentifierWrapper,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Option<IdentityWasm>;

    #[wasm_bindgen(structural, method, js_name=fetchLatestPlatformCoreChainLockedHeight)]
    pub fn fetch_latest_platform_core_chain_locked_height(
        this: &ExternalStateRepositoryLike,
    ) -> Option<u32>;

    #[wasm_bindgen(structural, method, js_name=fetchTransaction)]
    pub fn fetch_transaction(
        this: &ExternalStateRepositoryLike,
        id: &str,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> JsValue;

    #[wasm_bindgen(structural, method, js_name=isAssetLockTransactionOutPointAlreadyUsed)]
    pub fn is_asset_lock_transaction_out_point_already_used(
        this: &ExternalStateRepositoryLike,
        out_point_buffer: &[u8],
        execution_context: StateTransitionExecutionContextWasm,
    ) -> bool;

    #[wasm_bindgen(structural, method, js_name=verifyInstantLock)]
    pub fn verify_instant_lock(
        this: &ExternalStateRepositoryLike,
        instant_lock: &[u8],
        execution_context: StateTransitionExecutionContextWasm,
    ) -> bool;

    // TODO add missing declarations
}

/// Wraps external duck-typed thing into pinned box with mutex to ensure it'll stay at the same
/// place in memory and will have synchronized access.
pub(crate) struct ExternalStateRepositoryLikeWrapper(Pin<Box<Mutex<ExternalStateRepositoryLike>>>); // bruh

unsafe impl Send for ExternalStateRepositoryLikeWrapper {}
unsafe impl Sync for ExternalStateRepositoryLikeWrapper {}

impl ExternalStateRepositoryLikeWrapper {
    pub(crate) fn new(state_repository: ExternalStateRepositoryLike) -> Self {
        ExternalStateRepositoryLikeWrapper(Box::pin(Mutex::new(state_repository)))
    }
}

// TODO: move to a dedicated module together with all From traits?
#[derive(Clone, Serialize, Deserialize)]
pub struct FetchTransactionResponse {
    pub height: Option<u32>,
    pub data: Option<Vec<u8>>,
}

impl From<JsValue> for FetchTransactionResponse {
    fn from(v: JsValue) -> Self {
        if v.is_falsy() {
            FetchTransactionResponse {
                height: Some(0),
                data: None,
            }
        } else {
            serde_wasm_bindgen::from_value(v).unwrap()
        }
    }
}

impl From<FetchTransactionResponse> for FetchTransactionResponseDPP {
    fn from(v: FetchTransactionResponse) -> Self {
        FetchTransactionResponseDPP {
            data: v.data,
            height: v.height,
        }
    }
}

#[async_trait]
impl StateRepositoryLike for ExternalStateRepositoryLikeWrapper {
    type ConversionError = Infallible;
    type FetchDataContract = DataContractWasm;
    type FetchIdentity = IdentityWasm;
    type FetchTransaction = FetchTransactionResponse;

    async fn fetch_data_contract(
        &self,
        data_contract_id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Option<Self::FetchDataContract>> {
        Ok(self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .fetch_data_contract(
                data_contract_id.clone().into(),
                execution_context.clone().into(),
            )
            .map(Into::into))
    }

    async fn store_data_contract(
        &self,
        data_contract: DataContract,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        self.0
            .lock()
            .expect("unexpected concurrency issue!")
            .store_data_contract(data_contract.into(), execution_context.clone().into());
        Ok(())
    }

    async fn fetch_documents<T>(
        &self,
        _contract_id: &Identifier,
        _data_contract_type: &str,
        _where_query: serde_json::Value,
        _execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Vec<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static,
    {
        todo!()
    }

    async fn create_document(
        &self,
        _document: &Document,
        _execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn update_document(
        &self,
        _document: &Document,
        _execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn remove_document(
        &self,
        _data_contract: &DataContract,
        _data_contract_type: &str,
        _document_id: &Identifier,
        _execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn fetch_transaction(
        &self,
        id: &str,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Self::FetchTransaction> {
        let response = self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .fetch_transaction(id, execution_context.into());
        Ok(FetchTransactionResponse::from(response))
    }

    async fn fetch_identity(
        &self,
        id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Option<Self::FetchIdentity>> {
        Ok(self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .fetch_identity(id.clone().into(), execution_context.clone().into())
            .map(Into::into))
    }

    async fn store_identity_public_key_hashes(
        &self,
        _identity_id: &Identifier,
        _public_key_hashes: Vec<Vec<u8>>,
        _execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn fetch_identity_by_public_key_hashes<T>(
        &self,
        _public_key_hashed: Vec<Vec<u8>>,
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

    async fn fetch_latest_platform_core_chain_locked_height(&self) -> anyhow::Result<Option<u32>> {
        Ok(self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .fetch_latest_platform_core_chain_locked_height()
            .map(Into::into))
    }

    async fn verify_instant_lock(
        &self,
        instant_lock: &InstantLock,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<bool> {
        let raw_instant_lock = consensus::serialize(instant_lock);

        let verified = self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .verify_instant_lock(
                raw_instant_lock.as_slice(),
                execution_context.clone().into(),
            );

        Ok(verified)
    }

    async fn is_asset_lock_transaction_out_point_already_used(
        &self,
        out_point_buffer: &[u8],
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<bool> {
        Ok(self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .is_asset_lock_transaction_out_point_already_used(
                out_point_buffer,
                execution_context.clone().into(),
            ))
    }

    async fn mark_asset_lock_transaction_out_point_as_used(
        &self,
        _out_point_buffer: &[u8],
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
        _identity: &Identity,
        _execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn update_identity(
        &self,
        _identity: &Identity,
        _execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn fetch_latest_withdrawal_transaction_index(&self) -> anyhow::Result<u64> {
        todo!()
    }

    async fn enqueue_withdrawal_transaction(
        &self,
        _index: u64,
        _transaction_bytes: Vec<u8>,
    ) -> anyhow::Result<()> {
        todo!()
    }
}
