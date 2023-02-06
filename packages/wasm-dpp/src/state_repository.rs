//! Bindings for state repository -like objects coming from JS.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use std::{convert::Infallible, pin::Pin, sync::Mutex};

use async_trait::async_trait;
use dpp::dashcore::consensus;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::prelude::{Revision, TimestampMillis};
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
use js_sys::{Array, Number};
use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;
use crate::{
    identifier::IdentifierWrapper, DataContractWasm, IdentityPublicKeyWasm, IdentityWasm,
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

    #[wasm_bindgen(structural, method, js_name=createIdentity)]
    pub fn create_identity(
        this: &crate::state_repository::ExternalStateRepositoryLike,
        identity: IdentityWasm,
        execution_context: StateTransitionExecutionContextWasm,
    );

    #[wasm_bindgen(structural, method, js_name=addKeysToIdentity)]
    pub fn add_keys_to_identity(
        this: &crate::state_repository::ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        keys: js_sys::Array,
        execution_context: StateTransitionExecutionContextWasm,
    );

    #[wasm_bindgen(structural, method, js_name=disableIdentityKeys)]
    pub fn disable_identity_keys(
        this: &crate::state_repository::ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        keys: js_sys::Array,
        disable_at: Number,
        execution_context: StateTransitionExecutionContextWasm,
    );

    #[wasm_bindgen(structural, method, js_name=updateIdentityRevision)]
    pub fn update_identity_revision(
        this: &crate::state_repository::ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        revision: Number,
        execution_context: StateTransitionExecutionContextWasm,
    );

    #[wasm_bindgen(structural, method, js_name=fetchIdentityBalance)]
    pub fn fetch_identity_balance(
        this: &crate::state_repository::ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Option<Number>;

    #[wasm_bindgen(structural, method, js_name=fetchIdentityBalanceWithDebt)]
    pub fn fetch_identity_balance_with_debt(
        this: &crate::state_repository::ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Option<Number>;

    #[wasm_bindgen(structural, method, js_name=addToIdentityBalance)]
    pub fn add_to_identity_balance(
        this: &crate::state_repository::ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        amount: Number,
        execution_context: StateTransitionExecutionContextWasm,
    );

    #[wasm_bindgen(structural, method, js_name=removeFromIdentityBalance)]
    pub fn remove_from_identity_balance(
        this: &crate::state_repository::ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        amount: Number,
        execution_context: StateTransitionExecutionContextWasm,
    );

    #[wasm_bindgen(structural, method, js_name=addToSystemCredits)]
    pub fn add_to_system_credits(
        this: &crate::state_repository::ExternalStateRepositoryLike,
        amount: Number,
        execution_context: StateTransitionExecutionContextWasm,
    );

    #[wasm_bindgen(structural, method, js_name=removeFromSystemCredits)]
    pub fn remove_from_system_credits(
        this: &crate::state_repository::ExternalStateRepositoryLike,
        amount: Number,
        execution_context: StateTransitionExecutionContextWasm,
    );

    #[wasm_bindgen(structural, method, js_name=fetchLatestPlatformCoreChainLockedHeight)]
    pub fn fetch_latest_platform_core_chain_locked_height(
        this: &ExternalStateRepositoryLike,
    ) -> Option<u32>;

    #[wasm_bindgen(structural, method, js_name=fetchTransaction)]
    pub fn fetch_transaction(
        this: &ExternalStateRepositoryLike,
        id: JsValue,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> JsValue;

    #[wasm_bindgen(structural, method, js_name=isAssetLockTransactionOutPointAlreadyUsed)]
    pub fn is_asset_lock_transaction_out_point_already_used(
        this: &ExternalStateRepositoryLike,
        out_point_buffer: Buffer,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> bool;

    #[wasm_bindgen(structural, method, js_name=verifyInstantLock)]
    pub fn verify_instant_lock(
        this: &ExternalStateRepositoryLike,
        instant_lock: Vec<u8>,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> bool;

    #[wasm_bindgen(structural, method, js_name=markAssetLockTransactionOutPointAsUsed)]
    pub fn mark_asset_lock_transaction_out_point_as_used(
        this: &ExternalStateRepositoryLike,
        out_point_buffer: Buffer,
    );

    #[wasm_bindgen(structural, method, js_name=fetchLatestPlatformBlockHeader)]
    pub fn fetch_latest_platform_block_header(this: &ExternalStateRepositoryLike) -> Vec<u8>;

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
    ) -> Result<Option<Self::FetchDataContract>> {
        Ok(self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .fetch_data_contract((*data_contract_id).into(), execution_context.clone().into())
            .map(Into::into)) // TODO: Is it ok?
    }

    async fn store_data_contract(
        &self,
        data_contract: DataContract,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
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
    ) -> Result<Vec<T>>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static,
    {
        todo!()
    }

    async fn create_document(
        &self,
        _document: &Document,
        _execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        todo!()
    }

    async fn update_document(
        &self,
        _document: &Document,
        _execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        todo!()
    }

    async fn remove_document(
        &self,
        _data_contract: &DataContract,
        _data_contract_type: &str,
        _document_id: &Identifier,
        _execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        todo!()
    }

    async fn fetch_transaction(
        &self,
        id: &str,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<Self::FetchTransaction> {
        let response = self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .fetch_transaction(JsValue::from_str(id), execution_context.into());
        Ok(FetchTransactionResponse::from(response))
    }

    async fn fetch_identity(
        &self,
        id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<Option<Self::FetchIdentity>> {
        Ok(self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .fetch_identity((*id).into(), execution_context.clone().into())
            .map(Into::into))
    }

    async fn create_identity(
        &self,
        identity: &Identity,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        self.0
            .lock()
            .expect("unexpected concurrency issue!")
            .create_identity(identity.clone().into(), execution_context.clone().into());
        Ok(())
    }

    async fn add_keys_to_identity(
        &self,
        identity_id: &Identifier,
        keys: &[IdentityPublicKey],
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        self.0
            .lock()
            .expect("unexpected concurrency issue!")
            .add_keys_to_identity(
                (*identity_id).into(),
                keys.iter()
                    .map(|k| JsValue::from(IdentityPublicKeyWasm::from(k.clone())))
                    .collect::<Array>(),
                execution_context.clone().into(),
            );

        Ok(())
    }

    async fn disable_identity_keys(
        &self,
        identity_id: &Identifier,
        keys: &[KeyID],
        disable_at: TimestampMillis,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        self.0
            .lock()
            .expect("unexpected concurrency issue!")
            .disable_identity_keys(
                (*identity_id).into(),
                keys.iter().map(|&k| JsValue::from(k as f64)).collect(),
                Number::from(disable_at as f64),
                execution_context.clone().into(),
            );

        Ok(())
    }

    async fn update_identity_revision(
        &self,
        identity_id: &Identifier,
        revision: Revision,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        self.0
            .lock()
            .expect("unexpected concurrency issue!")
            .update_identity_revision(
                (*identity_id).into(),
                Number::from(revision as f64), // TODO: We should use BigInt
                execution_context.clone().into(),
            );

        Ok(())
    }

    async fn fetch_identity_balance(
        &self,
        identity_id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<Option<u64>> {
        let maybe_balance = self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .fetch_identity_balance((*identity_id).into(), execution_context.clone().into());

        if let Some(balance) = maybe_balance {
            Ok(balance
                .as_f64()
                .map(|b| b as u64)
                .or_else(|| panic!("can't convert balance to u64")))
        } else {
            Ok(None)
        }
    }

    async fn fetch_identity_balance_with_debt(
        &self,
        identity_id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<Option<i64>> {
        let maybe_balance = self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .fetch_identity_balance_with_debt(
                (*identity_id).into(),
                execution_context.clone().into(),
            );

        if let Some(balance) = maybe_balance {
            Ok(balance
                .as_f64()
                .map(|b| b as i64)
                .or_else(|| panic!("can't convert balance to f64")))
        } else {
            Ok(None)
        }
    }

    async fn add_to_identity_balance(
        &self,
        identity_id: &Identifier,
        amount: u64,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        self.0
            .lock()
            .expect("unexpected concurrency issue!")
            .add_to_identity_balance(
                (*identity_id).into(),
                Number::from(amount as f64),
                execution_context.clone().into(),
            );

        Ok(())
    }

    async fn remove_from_identity_balance(
        &self,
        identity_id: &Identifier,
        amount: u64,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        self.0
            .lock()
            .expect("unexpected concurrency issue!")
            .remove_from_identity_balance(
                (*identity_id).into(),
                Number::from(amount as f64),
                execution_context.clone().into(),
            );

        Ok(())
    }

    async fn add_to_system_credits(
        &self,
        amount: u64,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        self.0
            .lock()
            .expect("unexpected concurrency issue!")
            .add_to_system_credits(
                Number::from(amount as f64),
                execution_context.clone().into(),
            );

        Ok(())
    }

    async fn remove_from_system_credits(
        &self,
        _amount: u64,
        _execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        todo!()
    }

    async fn fetch_latest_platform_block_header(&self) -> anyhow::Result<Vec<u8>> {
        let header = self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .fetch_latest_platform_block_header();
        Ok(header)
    }

    async fn fetch_latest_platform_core_chain_locked_height(&self) -> anyhow::Result<Option<u32>> {
        let height = self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .fetch_latest_platform_core_chain_locked_height()
            .map(Into::into);
        Ok(height)
    }

    async fn verify_instant_lock(
        &self,
        instant_lock: &InstantLock,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<bool> {
        let raw_instant_lock = consensus::serialize(instant_lock);

        let verified = self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .verify_instant_lock(raw_instant_lock, execution_context.clone().into());

        Ok(verified)
    }

    async fn is_asset_lock_transaction_out_point_already_used(
        &self,
        out_point_buffer: &[u8],
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<bool> {
        Ok(self
            .0
            .lock()
            .expect("unexpected concurrency issue!")
            .is_asset_lock_transaction_out_point_already_used(
                Buffer::from_bytes(out_point_buffer),
                execution_context.clone().into(),
            ))
    }

    async fn mark_asset_lock_transaction_out_point_as_used(
        &self,
        out_point_buffer: &[u8],
    ) -> Result<()> {
        self.0
            .lock()
            .expect("unexpected concurrency issue!")
            .mark_asset_lock_transaction_out_point_as_used(Buffer::from_bytes(out_point_buffer));
        Ok(())
    }

    async fn fetch_sml_store<T>(&self) -> Result<T>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static,
    {
        todo!()
    }

    async fn fetch_latest_withdrawal_transaction_index(&self) -> Result<u64> {
        todo!()
    }
<<<<<<< HEAD

    async fn enqueue_withdrawal_transaction(
        &self,
        _index: u64,
        _transaction_bytes: Vec<u8>,
    ) -> Result<()> {
        todo!()
    }
=======
>>>>>>> feat/withdrawal-sync
}
