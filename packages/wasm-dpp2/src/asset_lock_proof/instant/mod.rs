mod instant_lock;

use crate::asset_lock_proof::instant::instant_lock::InstantLockWASM;
use crate::asset_lock_proof::outpoint::OutPointWASM;
use crate::asset_lock_proof::tx_out::TxOutWASM;
use crate::identifier::IdentifierWASM;
use crate::utils::WithJsError;
use dpp::dashcore::consensus::{deserialize, serialize};
use dpp::dashcore::{InstantLock, Transaction};
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstantAssetLockProofRAW {
    instant_lock: Vec<u8>,
    transaction: Vec<u8>,
    output_index: u32,
}

#[derive(Clone)]
#[wasm_bindgen(js_name = "InstantAssetLockProof")]
pub struct InstantAssetLockProofWASM(InstantAssetLockProof);

impl From<InstantAssetLockProofWASM> for InstantAssetLockProof {
    fn from(proof: InstantAssetLockProofWASM) -> Self {
        proof.0
    }
}

impl From<InstantAssetLockProof> for InstantAssetLockProofWASM {
    fn from(proof: InstantAssetLockProof) -> Self {
        InstantAssetLockProofWASM(proof)
    }
}

#[wasm_bindgen(js_class = InstantAssetLockProof)]
impl InstantAssetLockProofWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "InstantAssetLockProof".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "InstantAssetLockProof".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        instant_lock: Vec<u8>,
        transaction: Vec<u8>,
        output_index: u32,
    ) -> Result<InstantAssetLockProofWASM, JsValue> {
        let instant_lock: InstantLock =
            deserialize(instant_lock.as_slice()).map_err(|err| JsValue::from(err.to_string()))?;
        let transaction: Transaction =
            deserialize(transaction.as_slice()).map_err(|err| JsValue::from(err.to_string()))?;

        Ok(InstantAssetLockProofWASM(InstantAssetLockProof {
            instant_lock,
            transaction,
            output_index,
        }))
    }

    #[wasm_bindgen(js_name = "fromObject")]
    pub fn from_object(value: JsValue) -> Result<InstantAssetLockProofWASM, JsValue> {
        let parameters: InstantAssetLockProofRAW =
            serde_wasm_bindgen::from_value(value).map_err(|err| JsValue::from(err.to_string()))?;

        InstantAssetLockProofWASM::new(
            parameters.instant_lock,
            parameters.transaction,
            parameters.output_index,
        )
    }

    #[wasm_bindgen(js_name = "toObject")]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        self.0
            .to_object()
            .with_js_error()?
            .serialize(&serializer)
            .map_err(JsValue::from)
    }

    #[wasm_bindgen(js_name = "getOutput")]
    pub fn get_output(&self) -> Option<TxOutWASM> {
        match self.0.output() {
            Some(output) => Some(output.clone().into()),
            None => None,
        }
    }

    #[wasm_bindgen(js_name = "getOutPoint")]
    pub fn get_out_point(&self) -> Option<OutPointWASM> {
        match self.0.out_point() {
            Some(output) => Some(output.clone().into()),
            None => None,
        }
    }

    #[wasm_bindgen(getter = "outputIndex")]
    pub fn get_output_index(&self) -> u32 {
        self.0.output_index()
    }

    #[wasm_bindgen(getter = "instantLock")]
    pub fn get_instant_lock(&self) -> InstantLockWASM {
        self.0.instant_lock.clone().into()
    }

    #[wasm_bindgen(setter = "outputIndex")]
    pub fn set_output_index(&mut self, output_index: u32) {
        self.0.output_index = output_index;
    }

    #[wasm_bindgen(setter = "instantLock")]
    pub fn set_instant_lock(&mut self, instant_lock: &InstantLockWASM) {
        self.0.instant_lock = instant_lock.clone().into();
    }

    #[wasm_bindgen(js_name=getTransaction)]
    pub fn get_transaction(&self) -> Vec<u8> {
        let transaction = self.0.transaction();
        serialize(transaction)
    }

    #[wasm_bindgen(js_name=getInstantLockBytes)]
    pub fn get_instant_lock_bytes(&self) -> Vec<u8> {
        let instant_lock = self.0.instant_lock();
        serialize(instant_lock)
    }

    #[wasm_bindgen(js_name = "createIdentityId")]
    pub fn create_identifier(&self) -> Result<IdentifierWASM, JsValue> {
        let identifier = self.0.create_identifier().with_js_error()?;

        Ok(identifier.into())
    }
}
