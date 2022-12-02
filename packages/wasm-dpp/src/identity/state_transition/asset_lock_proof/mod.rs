mod chain;

pub use chain::*;

use wasm_bindgen::prelude::*;
use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;

#[wasm_bindgen(js_name=AssetLockProof)]
pub struct AssetLockProofWasm(AssetLockProof);

impl From<AssetLockProof> for AssetLockProofWasm {
    fn from(v: AssetLockProof) -> Self {
        AssetLockProofWasm(v)
    }
}