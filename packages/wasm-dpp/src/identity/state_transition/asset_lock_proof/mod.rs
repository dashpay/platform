mod chain;
mod instant;

pub use chain::*;
pub use instant::*;

use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=AssetLockProof)]
pub struct AssetLockProofWasm(AssetLockProof);

impl From<AssetLockProof> for AssetLockProofWasm {
    fn from(v: AssetLockProof) -> Self {
        AssetLockProofWasm(v)
    }
}
