use dpp::{
    dashcore::{
        blockdata::{script::Script, transaction::txout::TxOut},
        consensus::encode::serialize,
    },
    util::string_encoding,
    util::string_encoding::Encoding,
};

use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use wasm_bindgen::prelude::*;

use crate::{
    buffer::Buffer,
    errors::{from_dpp_err, RustConversionError},
    identifier::IdentifierWrapper,
    with_js_error,
};
use dpp::identity::state_transition::asset_lock_proof::instant::{
    InstantAssetLockProof, RawInstantLock,
};

#[derive(Serialize, Deserialize)]
#[serde(remote = "TxOut")]
struct TxOutJS {
    #[serde(rename = "satoshis")]
    value: u64,
    #[serde(rename = "script")]
    script_pubkey: Script,
}

#[derive(Serialize)]
struct TxOutSerdeHelper<'a>(#[serde(with = "TxOutJS")] &'a TxOut);

#[wasm_bindgen(js_name=InstantAssetLockProof)]
pub struct InstantAssetLockProofWasm(InstantAssetLockProof);

impl From<InstantAssetLockProof> for InstantAssetLockProofWasm {
    fn from(v: InstantAssetLockProof) -> Self {
        InstantAssetLockProofWasm(v)
    }
}

#[wasm_bindgen(js_class = InstantAssetLockProof)]
impl InstantAssetLockProofWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_parameters: JsValue) -> Result<InstantAssetLockProofWasm, JsValue> {
        let raw_instant_lock: RawInstantLock =
            with_js_error!(serde_wasm_bindgen::from_value(raw_parameters))?;

        let instant_asset_lock_proof: InstantAssetLockProof =
            raw_instant_lock.try_into().map_err(|_| {
                RustConversionError::Error(String::from("object passed is not a raw Instant Lock"))
                    .to_js_value()
            })?;

        Ok(instant_asset_lock_proof.into())
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        self.0.asset_lock_type()
    }

    #[wasm_bindgen(js_name=getOutputIndex)]
    pub fn get_output_index(&self) -> usize {
        self.0.output_index()
    }

    #[wasm_bindgen(js_name=getOutPoint)]
    pub fn get_out_point(&self) -> Option<Buffer> {
        self.0
            .out_point()
            .map(|out_point| Buffer::from_bytes(out_point.as_slice()))
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
        let identifier = self
            .0
            .create_identifier()
            .map_err(|e| from_dpp_err(e.into()))?;
        Ok(identifier.into())
    }

    #[wasm_bindgen(js_name=getInstantLock)]
    pub fn get_instant_lock(&self) -> Buffer {
        let instant_lock = self.0.instant_lock();
        let serialized_instant_lock = serialize(instant_lock);
        Buffer::from_bytes(serialized_instant_lock.as_slice())
    }

    #[wasm_bindgen(js_name=getTransaction)]
    pub fn get_transaction(&self) -> Buffer {
        let transaction = self.0.transaction();
        let serialized_transaction = serialize(transaction);
        Buffer::from_bytes(serialized_transaction.as_slice())
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let asset_lock_json =
            serde_json::to_value(self.0.clone()).map_err(|e| from_dpp_err(e.into()))?;

        let asset_lock_json_string =
            serde_json::to_string(&asset_lock_json).map_err(|e| from_dpp_err(e.into()))?;
        let js_object = js_sys::JSON::parse(&asset_lock_json_string)?;

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
