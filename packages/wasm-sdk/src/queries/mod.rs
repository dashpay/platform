pub mod data_contract;
pub mod document;
pub mod epoch;
pub mod group;
pub mod identity;
pub mod protocol;
pub mod system;
pub mod token;
pub mod voting;

// Re-export all query functions for easy access
pub use group::*;

use js_sys::Uint8Array;
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
        core_chain_locked_height: u32,
        epoch: u32,
        time_ms: u64,
        protocol_version: u32,
        chain_id: Uint8Array,
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
    pub fn set_chain_id(&mut self, chain_id: Uint8Array) {
        self.chain_id = chain_id.to_vec();
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
        grovedb_proof: Uint8Array,
        quorum_hash: Uint8Array,
        signature: Uint8Array,
        round: u32,
        block_id_hash: Uint8Array,
        quorum_type: u32,
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
    pub fn set_grovedb_proof(&mut self, grovedb_proof: Uint8Array) {
        self.grovedb_proof = grovedb_proof.to_vec();
    }

    #[wasm_bindgen(js_name = "setQuorumHash")]
    pub fn set_quorum_hash(&mut self, quorum_hash: Uint8Array) {
        self.quorum_hash = quorum_hash.to_vec();
    }

    #[wasm_bindgen(js_name = "setSignature")]
    pub fn set_signature(&mut self, signature: Uint8Array) {
        self.signature = signature.to_vec();
    }

    #[wasm_bindgen(js_name = "setBlockIdHash")]
    pub fn set_block_id_hash(&mut self, block_id_hash: Uint8Array) {
        self.block_id_hash = block_id_hash.to_vec();
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

    pub(crate) fn into_parts(self) -> (JsValue, ResponseMetadataWasm, ProofInfoWasm) {
        (self.data, self.metadata, self.proof)
    }
}
