//! Bindings for state repository -like objects coming from JS.

use std::{convert::Infallible, pin::Pin, sync::Mutex};

use anyhow::{anyhow, bail};
use async_trait::async_trait;
use dpp::{
    dashcore::InstantLock,
    data_contract::DataContract,
    document::Document,
    prelude::{Identifier, Identity},
    state_repository::StateRepositoryLike,
    state_transition::state_transition_execution_context::StateTransitionExecutionContext,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{
    console_log, identifier::IdentifierWrapper, utils::IntoWasm, DataContractWasm, DocumentWasm,
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

    #[wasm_bindgen(structural, method, js_name=createDocument)]
    pub fn create_document(
        this: &ExternalStateRepositoryLike,
        document: DocumentWasm,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> JsValue;

    #[wasm_bindgen(structural, method, js_name=updateDocument)]
    pub fn update_document(
        this: &ExternalStateRepositoryLike,
        document: DocumentWasm,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> JsValue;

    #[wasm_bindgen(structural, method, js_name=removeDocument)]
    pub fn remove_document(
        this: &ExternalStateRepositoryLike,
        data_contract: DataContractWasm,
        data_contract_type: String,
        document_id: IdentifierWrapper,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> JsValue;

    #[wasm_bindgen(structural, method, js_name=fetchDocuments)]
    pub fn fetch_documents(
        this: &ExternalStateRepositoryLike,
        data_contract_id: IdentifierWrapper,
        data_contract_type: String,
        where_query: JsValue,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> js_sys::Array;

    #[wasm_bindgen(structural, method, js_name=fetchLatestPlatformBlockTime)]
    pub fn fetch_latest_platform_block_time(this: &ExternalStateRepositoryLike) -> js_sys::Number;
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

#[async_trait]
impl StateRepositoryLike for ExternalStateRepositoryLikeWrapper {
    type ConversionError = Infallible;
    type FetchDataContract = DataContractWasm;
    type FetchDocument = DocumentWasm;

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

    async fn fetch_documents(
        &self,
        contract_id: &Identifier,
        data_contract_type: &str,
        where_query: serde_json::Value,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Vec<Self::FetchDocument>> {
        let js_documents = self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .fetch_documents(
                contract_id.to_owned().into(),
                data_contract_type.to_owned(),
                where_query
                    .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
                    .map_err(|e| anyhow!("serialization error: {}", e))?,
                execution_context.clone().into(),
            );

        let mut documents: Vec<DocumentWasm> = vec![];
        for js_document in js_documents.iter() {
            let document = js_document
                .to_wasm::<DocumentWasm>("Document")
                .map_err(|e| anyhow!("{e:#?}"))?;
            documents.push(document.to_owned());
        }
        Ok(documents)
    }

    async fn create_document(
        &self,
        document: &Document,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        let document_wasm: DocumentWasm = document.to_owned().into();
        self.0
            .lock()
            .expect("unexpected concurrency issue!")
            .create_document(document_wasm, execution_context.clone().into());
        Ok(())
    }

    async fn update_document(
        &self,
        document: &Document,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        let document_wasm: DocumentWasm = document.to_owned().into();
        self.0
            .lock()
            .expect("unexpected concurrency issue!")
            .update_document(document_wasm, execution_context.clone().into());
        Ok(())
    }

    async fn remove_document(
        &self,
        data_contract: &DataContract,
        data_contract_type: &str,
        document_id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        let data_contract: DataContractWasm = data_contract.to_owned().into();
        let document_id: IdentifierWrapper = document_id.to_owned().into();
        self.0
            .lock()
            .expect("unexpected concurrency issue!")
            .remove_document(
                data_contract,
                data_contract_type.to_owned(),
                document_id,
                execution_context.clone().into(),
            );
        Ok(())
    }

    async fn fetch_transaction<T>(
        &self,
        _id: &str,
        _execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Option<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static,
    {
        todo!()
    }

    async fn fetch_identity<T>(
        &self,
        _id: &Identifier,
        _execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Option<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static,
    {
        todo!()
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

    async fn verify_instant_lock(
        &self,
        _instant_lock: &InstantLock,
        _execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<bool> {
        todo!()
    }

    async fn is_asset_lock_transaction_out_point_already_used(
        &self,
        _out_point_buffer: &[u8],
        _execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<bool> {
        todo!()
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

    async fn fetch_latest_platform_block_time(&self) -> anyhow::Result<u64> {
        let js_number = self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .fetch_latest_platform_block_time();

        if let Some(float_number) = js_number.as_f64() {
            if float_number.is_nan() || float_number.is_infinite() {
                bail!("received an invalid timestamp: the number is either NaN or Inf")
            }
            if float_number < 0. {
                bail!("received an invalid timestamp: the number is negative");
            }
            if float_number.fract() != 0. {
                bail!("received an invalid timestamp: the number is fractional")
            }
            if float_number > u64::MAX as f64 {
                bail!("received an invalid timestamp: the number is > u64::max")
            }

            console_log!("returning the latest platform block time: {}", float_number);
            return Ok(float_number as u64);
        }

        bail!("fetching latest platform block failed: value is not number");
    }
}
