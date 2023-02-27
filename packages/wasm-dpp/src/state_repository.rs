//! Bindings for state repository -like objects coming from JS.

use serde::{Deserialize, Serialize};

use std::convert::Infallible;

use std::sync::Arc;

use anyhow::{anyhow, bail};
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
use js_sys::Uint8Array;
use js_sys::{Array, Number};
use wasm_bindgen::__rt::Ref;

use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;
use crate::errors::from_js_error;
use crate::utils::generic_of_js_val;
use crate::IdentityPublicKeyWasm;
use crate::{
    identifier::IdentifierWrapper, utils::IntoWasm, DataContractWasm, DocumentWasm, IdentityWasm,
    StateTransitionExecutionContextWasm,
};

#[wasm_bindgen]
extern "C" {
    pub type ExternalStateRepositoryLike;

    #[wasm_bindgen(catch, structural, method, js_name=fetchDataContract)]
    pub async fn fetch_data_contract(
        this: &ExternalStateRepositoryLike,
        data_contract_id: IdentifierWrapper,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=storeDataContract)]
    pub async fn store_data_contract(
        this: &ExternalStateRepositoryLike,
        data_contract: DataContractWasm,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=createDocument)]
    pub async fn create_document(
        this: &ExternalStateRepositoryLike,
        document: DocumentWasm,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=updateDocument)]
    pub async fn update_document(
        this: &ExternalStateRepositoryLike,
        document: DocumentWasm,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=removeDocument)]
    pub async fn remove_document(
        this: &ExternalStateRepositoryLike,
        data_contract: DataContractWasm,
        data_contract_type: String,
        document_id: IdentifierWrapper,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=fetchDocuments)]
    pub async fn fetch_documents(
        this: &ExternalStateRepositoryLike,
        data_contract_id: IdentifierWrapper,
        data_contract_type: String,
        where_query: JsValue,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=fetchIdentity)]
    pub async fn fetch_identity(
        this: &ExternalStateRepositoryLike,
        id: IdentifierWrapper,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=createIdentity)]
    pub async fn create_identity(
        this: &ExternalStateRepositoryLike,
        identity: IdentityWasm,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=addKeysToIdentity)]
    pub async fn add_keys_to_identity(
        this: &ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        keys: js_sys::Array,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=disableIdentityKeys)]
    pub async fn disable_identity_keys(
        this: &ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        keys: js_sys::Array,
        disable_at: Number,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=updateIdentityRevision)]
    pub async fn update_identity_revision(
        this: &ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        revision: Number,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=fetchIdentityBalance)]
    pub async fn fetch_identity_balance(
        this: &ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=fetchIdentityBalanceWithDebt)]
    pub async fn fetch_identity_balance_with_debt(
        this: &ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=addToIdentityBalance)]
    pub async fn add_to_identity_balance(
        this: &ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        amount: Number,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=removeFromIdentityBalance)]
    pub async fn remove_from_identity_balance(
        this: &ExternalStateRepositoryLike,
        identity_id: IdentifierWrapper,
        amount: Number,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=addToSystemCredits)]
    pub async fn add_to_system_credits(
        this: &ExternalStateRepositoryLike,
        amount: Number,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=removeFromSystemCredits)]
    pub async fn remove_from_system_credits(
        this: &ExternalStateRepositoryLike,
        amount: Number,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=fetchLatestPlatformCoreChainLockedHeight)]
    pub async fn fetch_latest_platform_core_chain_locked_height(
        this: &ExternalStateRepositoryLike,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=fetchLatestPlatformBlockHeight)]
    pub async fn fetch_latest_platform_block_height(
        this: &ExternalStateRepositoryLike,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=fetchTransaction)]
    pub async fn fetch_transaction(
        this: &ExternalStateRepositoryLike,
        id: JsValue,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=isAssetLockTransactionOutPointAlreadyUsed)]
    pub async fn is_asset_lock_transaction_out_point_already_used(
        this: &ExternalStateRepositoryLike,
        out_point_buffer: Buffer,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=verifyInstantLock)]
    pub async fn verify_instant_lock(
        this: &ExternalStateRepositoryLike,
        instant_lock: Vec<u8>,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=markAssetLockTransactionOutPointAsUsed)]
    pub async fn mark_asset_lock_transaction_out_point_as_used(
        this: &ExternalStateRepositoryLike,
        out_point_buffer: Buffer,
        execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<(), JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=fetchLatestPlatformBlockHeader)]
    pub async fn fetch_latest_platform_block_header(
        this: &ExternalStateRepositoryLike,
    ) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, structural, method, js_name=fetchLatestPlatformBlockTime)]
    pub async fn fetch_latest_platform_block_time(
        this: &ExternalStateRepositoryLike,
    ) -> Result<JsValue, JsValue>;

    // TODO add missing declarations
}

/// Wraps external duck-typed thing into pinned box with mutex to ensure it'll stay at the same
/// place in memory and will have synchronized access.
pub(crate) struct ExternalStateRepositoryLikeWrapper(Arc<ExternalStateRepositoryLike>);

unsafe impl Send for ExternalStateRepositoryLikeWrapper {}
unsafe impl Sync for ExternalStateRepositoryLikeWrapper {}

impl ExternalStateRepositoryLikeWrapper {
    pub(crate) fn new(state_repository: ExternalStateRepositoryLike) -> Self {
        ExternalStateRepositoryLikeWrapper(Arc::new(state_repository))
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

#[async_trait(?Send)]
impl StateRepositoryLike for ExternalStateRepositoryLikeWrapper {
    type ConversionError = Infallible;
    type FetchDataContract = DataContractWasm;
    type FetchDocument = DocumentWasm;
    type FetchIdentity = IdentityWasm;
    type FetchTransaction = FetchTransactionResponse;

    async fn fetch_data_contract(
        &self,
        data_contract_id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Option<Self::FetchDataContract>> {
        let maybe_data_contract: JsValue = self
            .0
            .fetch_data_contract((*data_contract_id).into(), execution_context.clone().into())
            .await
            .map_err(from_js_error)?;

        if maybe_data_contract.is_undefined() || maybe_data_contract.is_null() {
            return Ok(None);
        }

        let data_contract_conversion_result: Result<Ref<DataContractWasm>, JsValue> =
            generic_of_js_val::<DataContractWasm>(&maybe_data_contract, "DataContract");

        let data_contract = data_contract_conversion_result
            .map_err(from_js_error)?
            .to_owned();

        Ok(Some(data_contract))
    }

    async fn store_data_contract(
        &self,
        data_contract: DataContract,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        self.0
            .store_data_contract(data_contract.into(), execution_context.clone().into())
            .await
            .map_err(from_js_error)
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
            .fetch_documents(
                contract_id.to_owned().into(),
                data_contract_type.to_owned(),
                where_query
                    .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
                    .map_err(|e| anyhow!("serialization error: {}", e))?,
                execution_context.clone().into(),
            )
            .await
            .map_err(from_js_error)?;
        let js_documents_array = js_sys::Array::from(&js_documents);

        let mut documents: Vec<DocumentWasm> = vec![];
        for js_document in js_documents_array.iter() {
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
            .create_document(document_wasm, execution_context.clone().into())
            .await
            .map_err(from_js_error)
    }

    async fn update_document(
        &self,
        document: &Document,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        let document_wasm: DocumentWasm = document.to_owned().into();
        self.0
            .update_document(document_wasm, execution_context.clone().into())
            .await
            .map_err(from_js_error)
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
            .remove_document(
                data_contract,
                data_contract_type.to_owned(),
                document_id,
                execution_context.clone().into(),
            )
            .await
            .map_err(from_js_error)
    }

    async fn fetch_transaction(
        &self,
        id: &str,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Self::FetchTransaction> {
        let transaction_data = self
            .0
            .fetch_transaction(JsValue::from_str(id), execution_context.into())
            .await
            .map_err(from_js_error)?;

        Ok(FetchTransactionResponse::from(transaction_data))
    }

    async fn fetch_identity(
        &self,
        id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Option<Self::FetchIdentity>> {
        let maybe_identity = self
            .0
            .fetch_identity((*id).into(), execution_context.clone().into())
            .await
            .map_err(from_js_error)?;

        if maybe_identity.is_undefined() {
            return Ok(None);
        }

        let identity_conversion_result: Result<Ref<IdentityWasm>, JsValue> =
            generic_of_js_val::<IdentityWasm>(&maybe_identity, "Identity");

        let identity = identity_conversion_result
            .map_err(from_js_error)?
            .to_owned();

        Ok(Some(identity))
    }

    async fn create_identity(
        &self,
        identity: &Identity,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        self.0
            .create_identity(identity.clone().into(), execution_context.clone().into())
            .await
            .map_err(from_js_error)
    }

    async fn add_keys_to_identity(
        &self,
        identity_id: &Identifier,
        keys: &[IdentityPublicKey],
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        self.0
            .add_keys_to_identity(
                (*identity_id).into(),
                keys.iter()
                    .map(|k| JsValue::from(IdentityPublicKeyWasm::from(k.clone())))
                    .collect::<Array>(),
                execution_context.clone().into(),
            )
            .await
            .map_err(from_js_error)
    }

    async fn disable_identity_keys(
        &self,
        identity_id: &Identifier,
        keys: &[KeyID],
        disable_at: TimestampMillis,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        self.0
            .disable_identity_keys(
                (*identity_id).into(),
                keys.iter().map(|&k| JsValue::from(k as f64)).collect(),
                Number::from(disable_at as f64),
                execution_context.clone().into(),
            )
            .await
            .map_err(from_js_error)
    }

    async fn update_identity_revision(
        &self,
        identity_id: &Identifier,
        revision: Revision,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        self.0
            .update_identity_revision(
                (*identity_id).into(),
                Number::from(revision as f64), // TODO: We should use BigInt
                execution_context.clone().into(),
            )
            .await
            .map_err(from_js_error)
    }

    async fn fetch_identity_balance(
        &self,
        identity_id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Option<u64>> {
        let maybe_balance = self
            .0
            .fetch_identity_balance((*identity_id).into(), execution_context.clone().into())
            .await
            .map_err(from_js_error)?;

        if maybe_balance.is_undefined() {
            return Ok(None);
        }

        let balance = maybe_balance
            .as_f64()
            .ok_or_else(|| anyhow!("Value is not a number"))?;

        Ok(Some(balance as u64))
    }

    async fn fetch_identity_balance_with_debt(
        &self,
        identity_id: &Identifier,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<Option<i64>> {
        let maybe_balance = self
            .0
            .fetch_identity_balance_with_debt(
                (*identity_id).into(),
                execution_context.clone().into(),
            )
            .await
            .map_err(from_js_error)?;

        if maybe_balance.is_undefined() {
            return Ok(None);
        }

        let balance = maybe_balance
            .as_f64()
            .ok_or_else(|| anyhow!("Value is not a number"))?;

        Ok(Some(balance as i64))
    }

    async fn add_to_identity_balance(
        &self,
        identity_id: &Identifier,
        amount: u64,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        self.0
            .add_to_identity_balance(
                (*identity_id).into(),
                Number::from(amount as f64),
                execution_context.clone().into(),
            )
            .await
            .map_err(from_js_error)
    }

    async fn remove_from_identity_balance(
        &self,
        identity_id: &Identifier,
        amount: u64,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        self.0
            .remove_from_identity_balance(
                (*identity_id).into(),
                Number::from(amount as f64),
                execution_context.clone().into(),
            )
            .await
            .map_err(from_js_error)
    }

    async fn add_to_system_credits(
        &self,
        amount: u64,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        self.0
            .add_to_system_credits(
                Number::from(amount as f64),
                execution_context.clone().into(),
            )
            .await
            .map_err(from_js_error)
    }

    async fn remove_from_system_credits(
        &self,
        _amount: u64,
        _execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn fetch_latest_platform_block_header(&self) -> anyhow::Result<Vec<u8>> {
        let value: JsValue = self
            .0
            .fetch_latest_platform_block_header()
            .await
            .map_err(from_js_error)?;

        Ok(Uint8Array::new(&value).to_vec())
    }

    async fn fetch_latest_platform_core_chain_locked_height(&self) -> anyhow::Result<Option<u32>> {
        let maybe_height = self
            .0
            .fetch_latest_platform_core_chain_locked_height()
            .await
            .map_err(from_js_error)?;

        if maybe_height.is_undefined() {
            return Ok(None);
        }

        let height = maybe_height
            .as_f64()
            .ok_or_else(|| anyhow!("Value is not a number"))?;
        Ok(Some(height as u32))
    }

    async fn fetch_latest_platform_block_height(&self) -> anyhow::Result<u64> {
        let height = self
            .0
            .fetch_latest_platform_block_height()
            .await
            .map_err(from_js_error)?;

        let height = height
            .as_f64()
            .ok_or_else(|| anyhow!("Value is not a number"))?;
        Ok(height as u64)
    }

    async fn verify_instant_lock(
        &self,
        instant_lock: &InstantLock,
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<bool> {
        let raw_instant_lock = consensus::serialize(instant_lock);

        let verification_result = self
            .0
            .verify_instant_lock(raw_instant_lock, execution_context.clone().into())
            .await
            .map_err(from_js_error)?;

        verification_result
            .as_bool()
            .ok_or_else(|| anyhow!("Value is not a bool"))
    }

    async fn is_asset_lock_transaction_out_point_already_used(
        &self,
        out_point_buffer: &[u8],
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<bool> {
        let is_used = self
            .0
            .is_asset_lock_transaction_out_point_already_used(
                Buffer::from_bytes(out_point_buffer),
                execution_context.clone().into(),
            )
            .await
            .map_err(from_js_error)?;

        is_used
            .as_bool()
            .ok_or_else(|| anyhow!("Value is not a bool"))
    }

    async fn mark_asset_lock_transaction_out_point_as_used(
        &self,
        out_point_buffer: &[u8],
        execution_context: &StateTransitionExecutionContext,
    ) -> anyhow::Result<()> {
        self.0
            .mark_asset_lock_transaction_out_point_as_used(
                Buffer::from_bytes(out_point_buffer),
                execution_context.clone().into(),
            )
            .await
            .map_err(from_js_error)
    }

    async fn fetch_sml_store<T>(&self) -> anyhow::Result<T>
    where
        T: for<'de> serde::de::Deserialize<'de> + 'static,
    {
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
            .fetch_latest_platform_block_time()
            .await
            .map_err(from_js_error)?;

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

            return Ok(float_number as u64);
        }

        bail!("fetching latest platform block failed: value is not number");
    }
}
