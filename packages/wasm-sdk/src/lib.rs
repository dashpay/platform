use wasm_bindgen::prelude::wasm_bindgen;

pub mod context_provider;
pub mod dpns;
pub mod dpp;
pub mod error;
pub mod queries;
pub mod logging;
pub mod sdk;
pub mod state_transitions;
pub mod verify;
pub mod wallet;

// Re-export commonly used items
pub use dpns::*;
pub use queries::{
    data_contract::*, document::*, dpns::*, epoch::*, group::*, identity as query_identity,
    protocol::*, system::*, token::*, voting::*,
};
pub use sdk::{WasmSdk, WasmSdkBuilder};
pub use state_transitions::identity as state_transition_identity;
pub use wallet::*;
pub use error::{WasmSdkError, WasmSdkErrorKind};

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), WasmSdkError> {
    console_error_panic_hook::set_once();

    Ok(())
}
