use wasm_bindgen::prelude::wasm_bindgen;

pub mod context_provider;
pub mod dpns;
pub mod dpp;
pub mod error;
pub mod logging;
pub mod queries;
pub mod sdk;
pub mod state_transitions;
pub mod verify;
pub mod wallet;

// Re-export commonly used items
pub use dpns::*;
pub use error::{WasmSdkError, WasmSdkErrorKind};
pub use queries::{group::*, identity as query_identity};
pub use sdk::{WasmSdk, WasmSdkBuilder};
pub use state_transitions::identity as state_transition_identity;
pub use wallet::*;

// Use Rust's default allocator (dlmalloc) for wasm32-unknown-unknown
// This is more maintainable than third-party allocators and is actively supported by the Rust team

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), WasmSdkError> {
    console_error_panic_hook::set_once();

    Ok(())
}
