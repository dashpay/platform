use dpp::dashcore::{
    blockdata::{script::ScriptBuf, transaction::txout::TxOut},
    consensus::encode::serialize,
};

use dpp::dashcore::consensus::Encodable;
use dpp::identity::state_transition::asset_lock_proof::AssetLockProofType;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use wasm_bindgen::prelude::*;

use crate::utils::WithJsError;
use crate::{buffer::Buffer, errors::from_dpp_err, identifier::IdentifierWrapper, with_js_error};
use dpp::identity::state_transition::asset_lock_proof::instant::{
    InstantAssetLockProof, RawInstantLockProof,
};
use dpp::platform_value::string_encoding;
use dpp::platform_value::string_encoding::Encoding;

#[derive(Serialize, Deserialize)]
#[serde(remote = "TxOut")]
struct TxOutJS {
    #[serde(rename = "satoshis")]
    value: u64,
    #[serde(rename = "script")]
    script_pubkey: ScriptBuf,
}

#[derive(Serialize)]
struct TxOutSerdeHelper<'a>(#[serde(with = "TxOutJS")] &'a TxOut);

#[derive(Debug, Clone, Deserialize, Serialize)]
#[wasm_bindgen(js_name=InstantAssetLockProof)]
pub struct InstantAssetLockProofWasm(InstantAssetLockProof);

impl From<InstantAssetLockProof> for InstantAssetLockProofWasm {
    fn from(v: InstantAssetLockProof) -> Self {
        InstantAssetLockProofWasm(v)
    }
}

impl From<InstantAssetLockProofWasm> for InstantAssetLockProof {
    fn from(v: InstantAssetLockProofWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = InstantAssetLockProof)]
impl InstantAssetLockProofWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_parameters: JsValue) -> Result<InstantAssetLockProofWasm, JsError> {
        let raw_instant_lock: RawInstantLockProof = serde_wasm_bindgen::from_value(raw_parameters)
            .map_err(|_| JsError::new("invalid raw instant lock proof"))?;

        let instant_asset_lock_proof: InstantAssetLockProof = raw_instant_lock
            .try_into()
            .map_err(|_| JsError::new("object passed is not a raw Instant Lock"))?;

        Ok(instant_asset_lock_proof.into())
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        AssetLockProofType::Instant as u8
    }

    #[wasm_bindgen(js_name=getOutputIndex)]
    pub fn get_output_index(&self) -> u32 {
        self.0.output_index()
    }

    #[wasm_bindgen(js_name=getOutPoint)]
    pub fn get_out_point(&self) -> Result<Option<Buffer>, JsValue> {
        self.0
            .out_point()
            .map(|out_point| {
                let mut outpoint_bytes = Vec::new();

                out_point
                    .consensus_encode(&mut outpoint_bytes)
                    .map_err(|e| e.to_string())?;

                Ok(Buffer::from_bytes_owned(outpoint_bytes))
            })
            .transpose()
    }

    #[wasm_bindgen(js_name=getOutput)]
    pub fn get_output(&self) -> Result<JsValue, JsValue> {
        let output = self.0.output().unwrap();
        let output_json_string =
            serde_json::to_string(&TxOutSerdeHelper(output)).map_err(|e| from_dpp_err(e.into()))?;

        let js_object = js_sys::JSON::parse(&output_json_string)?;
        Ok(js_object)
    }

    #[wasm_bindgen(js_name=createIdentifier)]
    pub fn create_identifier(&self) -> Result<IdentifierWrapper, JsValue> {
        let identifier = self.0.create_identifier().map_err(from_dpp_err)?;
        Ok(identifier.into())
    }

    #[wasm_bindgen(js_name=getInstantLock)]
    pub fn get_instant_lock(&self) -> Buffer {
        let instant_lock = self.0.instant_lock();
        let serialized_instant_lock = serialize(instant_lock);
        Buffer::from_bytes(&serialized_instant_lock)
    }

    #[wasm_bindgen(js_name=getTransaction)]
    pub fn get_transaction(&self) -> Buffer {
        let transaction = self.0.transaction();
        let serialized_transaction = serialize(transaction);
        Buffer::from_bytes(&serialized_transaction)
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let asset_lock_value = self.0.to_cleaned_object().with_js_error()?;

        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_object = with_js_error!(asset_lock_value.serialize(&serializer))?;

        let transaction = self.get_transaction();
        let instant_lock = self.get_instant_lock();

        js_sys::Reflect::set(
            &js_object,
            &"transaction".to_owned().into(),
            &JsValue::from(transaction),
        )?;

        js_sys::Reflect::set(
            &js_object,
            &"instantLock".to_owned().into(),
            &JsValue::from(instant_lock),
        )?;

        Ok(js_object)
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let js_object = self.to_object()?;

        let transaction = self.0.transaction();
        let serialized_transaction = serialize(transaction);

        let instant_lock = self.0.instant_lock();
        let serialized_instant_lock = serialize(instant_lock);

        let instant_lock_base64 =
            string_encoding::encode(serialized_instant_lock.as_slice(), Encoding::Base64);

        let mut transaction_hex = String::new();
        for &byte in serialized_transaction.as_slice() {
            transaction_hex.push_str(&format!("{:02x}", byte));
        }

        js_sys::Reflect::set(
            &js_object,
            &"transaction".to_owned().into(),
            &JsValue::from(transaction_hex),
        )?;

        js_sys::Reflect::set(
            &js_object,
            &"instantLock".to_owned().into(),
            &JsValue::from(instant_lock_base64),
        )?;

        Ok(js_object)
    }
}
