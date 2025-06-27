use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

pub mod asset_lock;
pub mod bip39;
pub mod bls;
pub mod broadcast;
pub mod cache;
pub mod context_provider;
pub mod contract_cache;
pub mod contract_history;
pub mod dapi_client;
pub mod dpp;
pub mod epoch;
pub mod error;
pub mod fetch;
pub mod fetch_many;
pub mod fetch_unproved;
pub mod group_actions;
pub mod identity_info;
pub mod metadata;
pub mod monitoring;
pub mod nonce;
pub mod optimize;
pub mod prefunded_balance;
pub mod query;
pub mod request_settings;
pub mod sdk;
pub mod signer;
pub mod serializer;
pub mod state_transitions;
pub mod subscriptions;
pub mod subscriptions_v2;
pub mod token;
pub mod verify;
pub mod verify_bridge;
pub mod voting;
pub mod withdrawal;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    // We use tracing-wasm together with console_error_panic_hook to get logs from the wasm module.
    // Other alternatives are:
    // * https://github.com/jquesada2016/tracing_subscriber_wasm
    // * https://crates.io/crates/tracing-web
    console_error_panic_hook::set_once();

    tracing_wasm::set_as_global_default();

    Ok(())
}
