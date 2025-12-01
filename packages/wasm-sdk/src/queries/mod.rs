pub mod data_contract;
pub mod document;
pub mod epoch;
pub mod group;
pub mod identity;
pub mod protocol;
pub mod system;
pub mod token;
pub(crate) mod utils;
pub mod voting;

// Re-export all query functions for easy access
pub use group::*;

use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = "ResponseMetadata")]
#[derive(Clone, Debug)]
pub struct ResponseMetadataWasm {
    height: u64,
    core_chain_locked_height: u32,
    epoch: u32,
    time_ms: u64,
    protocol_version: u32,
    chain_id: Vec<u8>,
}

#[wasm_bindgen(js_class = ResponseMetadata)]
impl ResponseMetadataWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        height: u64,
        #[wasm_bindgen(js_name = "coreChainLockedHeight")] core_chain_locked_height: u32,
        epoch: u32,
        #[wasm_bindgen(js_name = "timeMs")] time_ms: u64,
        #[wasm_bindgen(js_name = "protocolVersion")] protocol_version: u32,
        #[wasm_bindgen(js_name = "chainId")] chain_id: Uint8Array,
    ) -> Self {
        ResponseMetadataWasm {
            height,
            core_chain_locked_height,
            epoch,
            time_ms,
            protocol_version,
            chain_id: chain_id.to_vec(),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u64 {
        self.height
    }

    #[wasm_bindgen(getter = coreChainLockedHeight)]
    pub fn core_chain_locked_height(&self) -> u32 {
        self.core_chain_locked_height
    }

    #[wasm_bindgen(getter)]
    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    #[wasm_bindgen(getter = timeMs)]
    pub fn time_ms(&self) -> u64 {
        self.time_ms
    }

    #[wasm_bindgen(getter = protocolVersion)]
    pub fn protocol_version(&self) -> u32 {
        self.protocol_version
    }

    #[wasm_bindgen(getter = chainId)]
    pub fn chain_id(&self) -> Uint8Array {
        Uint8Array::from(self.chain_id.as_slice())
    }

    #[wasm_bindgen(js_name = "setChainId")]
    pub fn set_chain_id(&mut self, #[wasm_bindgen(js_name = "chainId")] chain_id: Uint8Array) {
        self.chain_id = chain_id.to_vec();
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> JsValue {
        let obj = Object::new();
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("height"),
            &JsValue::from_f64(self.height as f64),
        );
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("coreChainLockedHeight"),
            &JsValue::from_f64(self.core_chain_locked_height as f64),
        );
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("epoch"),
            &JsValue::from_f64(self.epoch as f64),
        );
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("timeMs"),
            &JsValue::from_f64(self.time_ms as f64),
        );
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("protocolVersion"),
            &JsValue::from_f64(self.protocol_version as f64),
        );
        let _ = Reflect::set(&obj, &JsValue::from_str("chainId"), &self.chain_id());

        JsValue::from(obj)
    }
}

// Helper function to convert platform ResponseMetadata to our ResponseMetadata
impl From<dash_sdk::platform::proto::ResponseMetadata> for ResponseMetadataWasm {
    fn from(metadata: dash_sdk::platform::proto::ResponseMetadata) -> Self {
        ResponseMetadataWasm {
            height: metadata.height,
            core_chain_locked_height: metadata.core_chain_locked_height,
            epoch: metadata.epoch,
            time_ms: metadata.time_ms,
            protocol_version: metadata.protocol_version,
            chain_id: metadata.chain_id.into_bytes(),
        }
    }
}

#[wasm_bindgen(js_name = "ProofInfo")]
#[derive(Clone, Debug)]
pub struct ProofInfoWasm {
    grovedb_proof: Vec<u8>,
    quorum_hash: Vec<u8>,
    signature: Vec<u8>,
    round: u32,
    block_id_hash: Vec<u8>,
    quorum_type: u32,
}

#[wasm_bindgen(js_class = ProofInfo)]
impl ProofInfoWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(js_name = "grovedbProof")] grovedb_proof: Uint8Array,
        #[wasm_bindgen(js_name = "quorumHash")] quorum_hash: Uint8Array,
        signature: Uint8Array,
        round: u32,
        #[wasm_bindgen(js_name = "blockIdHash")] block_id_hash: Uint8Array,
        #[wasm_bindgen(js_name = "quorumType")] quorum_type: u32,
    ) -> Self {
        ProofInfoWasm {
            grovedb_proof: grovedb_proof.to_vec(),
            quorum_hash: quorum_hash.to_vec(),
            signature: signature.to_vec(),
            round,
            block_id_hash: block_id_hash.to_vec(),
            quorum_type,
        }
    }

    #[wasm_bindgen(getter = grovedbProof)]
    pub fn grovedb_proof(&self) -> Uint8Array {
        Uint8Array::from(self.grovedb_proof.as_slice())
    }

    #[wasm_bindgen(getter = quorumHash)]
    pub fn quorum_hash(&self) -> Uint8Array {
        Uint8Array::from(self.quorum_hash.as_slice())
    }

    #[wasm_bindgen(getter)]
    pub fn signature(&self) -> Uint8Array {
        Uint8Array::from(self.signature.as_slice())
    }

    #[wasm_bindgen(getter)]
    pub fn round(&self) -> u32 {
        self.round
    }

    #[wasm_bindgen(getter = blockIdHash)]
    pub fn block_id_hash(&self) -> Uint8Array {
        Uint8Array::from(self.block_id_hash.as_slice())
    }

    #[wasm_bindgen(getter = quorumType)]
    pub fn quorum_type(&self) -> u32 {
        self.quorum_type
    }

    #[wasm_bindgen(js_name = "setGrovedbProof")]
    pub fn set_grovedb_proof(
        &mut self,
        #[wasm_bindgen(js_name = "grovedbProof")] grovedb_proof: Uint8Array,
    ) {
        self.grovedb_proof = grovedb_proof.to_vec();
    }

    #[wasm_bindgen(js_name = "setQuorumHash")]
    pub fn set_quorum_hash(
        &mut self,
        #[wasm_bindgen(js_name = "quorumHash")] quorum_hash: Uint8Array,
    ) {
        self.quorum_hash = quorum_hash.to_vec();
    }

    #[wasm_bindgen(js_name = "setSignature")]
    pub fn set_signature(&mut self, signature: Uint8Array) {
        self.signature = signature.to_vec();
    }

    #[wasm_bindgen(js_name = "setBlockIdHash")]
    pub fn set_block_id_hash(
        &mut self,
        #[wasm_bindgen(js_name = "blockIdHash")] block_id_hash: Uint8Array,
    ) {
        self.block_id_hash = block_id_hash.to_vec();
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> JsValue {
        let obj = Object::new();
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("grovedbProof"),
            &self.grovedb_proof(),
        );
        let _ = Reflect::set(&obj, &JsValue::from_str("quorumHash"), &self.quorum_hash());
        let _ = Reflect::set(&obj, &JsValue::from_str("signature"), &self.signature());
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("round"),
            &JsValue::from_f64(self.round as f64),
        );
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("blockIdHash"),
            &self.block_id_hash(),
        );
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("quorumType"),
            &JsValue::from_f64(self.quorum_type as f64),
        );

        JsValue::from(obj)
    }
}

// Helper function to convert platform Proof to our ProofInfo
impl From<dash_sdk::platform::proto::Proof> for ProofInfoWasm {
    fn from(proof: dash_sdk::platform::proto::Proof) -> Self {
        ProofInfoWasm {
            grovedb_proof: proof.grovedb_proof,
            quorum_hash: proof.quorum_hash,
            signature: proof.signature,
            round: proof.round,
            block_id_hash: proof.block_id_hash,
            quorum_type: proof.quorum_type,
        }
    }
}

#[wasm_bindgen(js_name = "ProofMetadataResponse")]
#[derive(Clone, Debug)]
pub struct ProofMetadataResponseWasm {
    data: JsValue,
    metadata: ResponseMetadataWasm,
    proof: ProofInfoWasm,
}

#[wasm_bindgen(typescript_custom_section)]
const PROOF_METADATA_TYPED_TS: &'static str = r#"
export type ProofMetadataResponseTyped<T> = ProofMetadataResponse & { data: T };
"#;

#[wasm_bindgen(js_class = ProofMetadataResponse)]
impl ProofMetadataResponseWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(data: JsValue, metadata: ResponseMetadataWasm, proof: ProofInfoWasm) -> Self {
        ProofMetadataResponseWasm {
            data,
            metadata,
            proof,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn data(&self) -> JsValue {
        self.data.clone()
    }

    #[wasm_bindgen(js_name = "setData")]
    pub fn set_data(&mut self, data: JsValue) {
        self.data = data;
    }

    #[wasm_bindgen(getter)]
    pub fn metadata(&self) -> ResponseMetadataWasm {
        self.metadata.clone()
    }

    #[wasm_bindgen(js_name = "setMetadata")]
    pub fn set_metadata(&mut self, metadata: ResponseMetadataWasm) {
        self.metadata = metadata;
    }

    #[wasm_bindgen(getter)]
    pub fn proof(&self) -> ProofInfoWasm {
        self.proof.clone()
    }

    #[wasm_bindgen(js_name = "setProof")]
    pub fn set_proof(&mut self, proof: ProofInfoWasm) {
        self.proof = proof;
    }

    #[wasm_bindgen(js_name = "toJSON")]
    pub fn to_json(&self) -> JsValue {
        let metadata_obj = self.metadata.to_json();
        let proof_obj = self.proof.to_json();

        let obj = Object::new();
        let _ = Reflect::set(&obj, &JsValue::from_str("data"), &self.data);
        let _ = Reflect::set(&obj, &JsValue::from_str("metadata"), &metadata_obj);
        let _ = Reflect::set(&obj, &JsValue::from_str("proof"), &proof_obj);

        JsValue::from(obj)
    }
}

impl ProofMetadataResponseWasm {
    pub(crate) fn from_parts(
        data: JsValue,
        metadata: ResponseMetadataWasm,
        proof: ProofInfoWasm,
    ) -> Self {
        ProofMetadataResponseWasm {
            data,
            metadata,
            proof,
        }
    }

    pub(crate) fn from_sdk_parts(
        data: impl Into<JsValue>,
        metadata: dash_sdk::platform::proto::ResponseMetadata,
        proof: dash_sdk::platform::proto::Proof,
    ) -> Self {
        ProofMetadataResponseWasm {
            data: data.into(),
            metadata: metadata.into(),
            proof: proof.into(),
        }
    }
}
