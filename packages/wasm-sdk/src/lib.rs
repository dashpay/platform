use wasm_bindgen::prelude::wasm_bindgen;

pub mod context_provider;
pub mod dpns;
pub mod dpp;
pub mod error;
pub mod queries;
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
    // We use tracing-wasm together with console_error_panic_hook to get logs from the wasm module.
    // Other alternatives are:
    // * https://github.com/jquesada2016/tracing_subscriber_wasm
    // * https://crates.io/crates/tracing-web
    console_error_panic_hook::set_once();

    tracing_wasm::set_as_global_default();

    Ok(())
}
