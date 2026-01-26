use crate::asset_lock_proof::outpoint::OutPointWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::utils::try_to_u32;
use dpp::dashcore::consensus::{deserialize, serialize};
use dpp::dashcore::{InstantLock, Transaction};
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * InstantAssetLockProof serialized as a plain object.
 */
export interface InstantAssetLockProofObject {
    instantLock: Uint8Array;
    transaction: Uint8Array;
    outputIndex: number;
}

/**
 * InstantAssetLockProof serialized as JSON.
 */
export interface InstantAssetLockProofJSON {
    instantLock: string;
    transaction: string;
    outputIndex: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "InstantAssetLockProofObject")]
    pub type InstantAssetLockProofObjectJs;

    #[wasm_bindgen(typescript_type = "InstantAssetLockProofJSON")]
    pub type InstantAssetLockProofJSONJs;
}

#[derive(Clone)]
#[wasm_bindgen(js_name = "InstantAssetLockProof")]
pub struct InstantAssetLockProofWasm(InstantAssetLockProof);

impl From<InstantAssetLockProofWasm> for InstantAssetLockProof {
    fn from(proof: InstantAssetLockProofWasm) -> Self {
        proof.0
    }
}

impl From<InstantAssetLockProof> for InstantAssetLockProofWasm {
    fn from(proof: InstantAssetLockProof) -> Self {
        InstantAssetLockProofWasm(proof)
    }
}

#[wasm_bindgen(js_class = InstantAssetLockProof)]
impl InstantAssetLockProofWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        #[wasm_bindgen(js_name = "instantLock")] instant_lock: Vec<u8>,
        transaction: Vec<u8>,
        #[wasm_bindgen(js_name = "outputIndex")] output_index: u32,
    ) -> WasmDppResult<InstantAssetLockProofWasm> {
        let instant_lock: InstantLock = deserialize(instant_lock.as_slice())
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;
        let transaction: Transaction = deserialize(transaction.as_slice())
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;

        Ok(InstantAssetLockProofWasm(InstantAssetLockProof {
            instant_lock,
            transaction,
            output_index,
        }))
    }

    #[wasm_bindgen(getter = "output")]
    pub fn output(&self) -> Option<Vec<u8>> {
        self.0.output().map(serialize)
    }

    #[wasm_bindgen(getter = "outPoint")]
    pub fn out_point(&self) -> Option<OutPointWasm> {
        self.0.out_point().map(|output| output.into())
    }

    #[wasm_bindgen(getter = "outputIndex")]
    pub fn output_index(&self) -> u32 {
        self.0.output_index()
    }

    #[wasm_bindgen(getter = "instantLock")]
    pub fn instant_lock(&self) -> Vec<u8> {
        serialize(&self.0.instant_lock)
    }

    #[wasm_bindgen(setter = "outputIndex")]
    pub fn set_output_index(&mut self, #[wasm_bindgen(js_name = "outputIndex")] output_index: &js_sys::Number) -> WasmDppResult<()> {
        self.0.output_index = try_to_u32(output_index, "outputIndex")?;
        Ok(())
    }

    #[wasm_bindgen(setter = "instantLock")]
    pub fn set_instant_lock(
        &mut self,
        #[wasm_bindgen(js_name = "instantLock")] instant_lock: Vec<u8>,
    ) -> WasmDppResult<()> {
        self.0.instant_lock = deserialize(instant_lock.as_slice())
            .map_err(|err| WasmDppError::serialization(err.to_string()))?;
        Ok(())
    }

    #[wasm_bindgen(getter = "transaction")]
    pub fn transaction(&self) -> Vec<u8> {
        let transaction = self.0.transaction();
        serialize(transaction)
    }

    #[wasm_bindgen(js_name = "createIdentityId")]
    pub fn create_identifier(&self) -> WasmDppResult<IdentifierWasm> {
        let identifier = self.0.create_identifier()?;

        Ok(identifier.into())
    }
}

impl_wasm_conversions!(
    InstantAssetLockProofWasm,
    InstantAssetLockProof,
    InstantAssetLockProofObjectJs,
    InstantAssetLockProofJSONJs
);
impl_wasm_type_info!(InstantAssetLockProofWasm, InstantAssetLockProof);
